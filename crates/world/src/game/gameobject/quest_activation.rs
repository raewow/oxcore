//! Which gameobjects the client is allowed to interact with, per player.
//!
//! An object carrying `GO_FLAG_INTERACT_COND` is inert on the client until the server says
//! otherwise, and the way it says so is the `Activate` dynamic flag — computed per viewer, since
//! the same vineyard is clickable for the player on the quest and scenery for everyone else.
//!
//! Two halves, mirroring the protocol's own split:
//!
//! * [`activates_to_quest`] answers "is this object live for this player right now", and is what
//!   the create block and the refresh pass below both call.
//! * [`refresh_quest_gameobjects`] re-answers it for everything the player can already see. The
//!   create block only runs when an object enters visibility, so without this an object that was
//!   already on screen when the quest was accepted would stay inert until the player walked out
//!   of range and back.

use std::collections::HashSet;

use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::protocol::update_fields::{GAMEOBJECT_ANIMPROGRESS, GAMEOBJECT_DYN_FLAGS};
use oxcore_shared::protocol::ObjectGuid;

use super::types::{go_dyn_flags, GameObjectType};
use super::GameObjectTemplate;
use crate::game::npc::quest::types::QuestStatus;
use crate::World;

/// The quest a gameobject's template names, or 0 for the types that name none.
///
/// One `data` column per type, at the offset that type's fields put it at — the array is a union,
/// so reading index 8 on a goober would be its `openTextID`, not a quest.
fn template_quest_id(go_type: GameObjectType, template: &GameObjectTemplate) -> u32 {
    let index = match go_type {
        GameObjectType::Chest => 8,
        GameObjectType::Generic => 5,
        GameObjectType::SpellFocus => 4,
        GameObjectType::Goober => 1,
        _ => return 0,
    };
    template.data[index].max(0) as u32
}

/// Whether this entry could ever be quest-relevant to *someone*.
///
/// The cheap gate in front of the per-player rules below: an ordinary chest or signpost can skip
/// them entirely. Ordinary objects deliberately answer "not activated" rather than "always
/// activated" — the flag only ever unlocks conditional objects, and a chest without it is still
/// clickable through its own type.
fn is_gameobject_for_quests(
    entry: u32,
    go_type: GameObjectType,
    template: &GameObjectTemplate,
    world: &World,
) -> bool {
    match go_type {
        GameObjectType::QuestGiver => {
            !world
                .systems
                .quest
                .manager
                .get_go_quest_relations(entry)
                .is_empty()
                || !world
                    .systems
                    .quest
                    .manager
                    .get_go_involved_relations(entry)
                    .is_empty()
        }
        // A chest counts if it names a quest, or if its loot table holds a quest item at all —
        // the object itself never mentions the quest those drops belong to.
        GameObjectType::Chest => {
            template_quest_id(go_type, template) != 0
                || world
                    .systems
                    .loot_manager
                    .gameobject_loot_has_quest_drop(template.data[1].max(0) as u32)
        }
        GameObjectType::Generic | GameObjectType::SpellFocus | GameObjectType::Goober => {
            template_quest_id(go_type, template) != 0
        }
        _ => false,
    }
}

/// Whether `viewer_guid` should see this gameobject as activated.
///
/// Ported from `GameObject::ActivateToQuest`. Being an objective of one of the player's own quests
/// wins outright; past that each type asks about the quest its template names.
pub fn activates_to_quest(
    entry: u32,
    go_type: GameObjectType,
    template: &GameObjectTemplate,
    viewer_guid: ObjectGuid,
    world: &World,
) -> bool {
    if world
        .systems
        .quest
        .player_has_quest_for_gameobject(viewer_guid, entry)
    {
        return true;
    }

    if !is_gameobject_for_quests(entry, go_type, template, world) {
        return false;
    }

    let quest_id = template_quest_id(go_type, template);
    let is_incomplete = |quest_id: u32| {
        quest_id != 0
            && world.systems.quest.get_quest_status(viewer_guid, quest_id)
                == QuestStatus::Incomplete
    };

    match go_type {
        GameObjectType::QuestGiver => {
            let startable = world
                .systems
                .quest
                .manager
                .get_go_quest_relations(entry)
                .into_iter()
                .filter_map(|quest_id| world.systems.quest.manager.get_quest_template(quest_id))
                .any(|quest| {
                    world
                        .systems
                        .quest
                        .can_take_quest(viewer_guid, &quest, world)
                });
            if startable {
                return true;
            }

            world
                .systems
                .quest
                .manager
                .get_go_involved_relations(entry)
                .into_iter()
                .any(|quest_id| {
                    let status = world.systems.quest.get_quest_status(viewer_guid, quest_id);
                    let turnable =
                        matches!(status, QuestStatus::Incomplete | QuestStatus::Complete);
                    turnable
                        && !world
                            .managers
                            .player_mgr
                            .with_player(viewer_guid, |p| p.rewarded_quests.contains(&quest_id))
                            .unwrap_or(false)
                })
        }
        GameObjectType::Chest => {
            if is_incomplete(quest_id) {
                return true;
            }
            // The chest holds a quest drop this player still needs.
            world
                .systems
                .loot_manager
                .player_needs_gameobject_quest_loot(template.data[1].max(0) as u32, |item_id| {
                    world
                        .systems
                        .quest
                        .player_has_quest_for_item(viewer_guid, item_id)
                })
        }
        GameObjectType::Generic | GameObjectType::SpellFocus | GameObjectType::Goober => {
            is_incomplete(quest_id)
        }
        _ => false,
    }
}

/// Look the object up and answer [`activates_to_quest`] for it.
///
/// Returns false for anything that is not a live gameobject with a template, so callers can hand
/// it any guid out of a visibility set.
pub fn gameobject_activates_to_quest(
    guid: ObjectGuid,
    viewer_guid: ObjectGuid,
    world: &World,
) -> bool {
    let Some((entry, go_type)) = world
        .managers
        .gameobject_mgr
        .with_gameobject(guid, |go| (go.entry, go.go_type))
    else {
        return false;
    };
    let Some(template) = world.managers.gameobject_mgr.get_template(entry) else {
        return false;
    };

    activates_to_quest(entry, go_type, &template, viewer_guid, world)
}

/// Re-evaluate every gameobject the player can currently see, and tell the client about the ones
/// that changed.
///
/// Called wherever quest state moves — accepting, turning in, abandoning, or gaining and losing a
/// quest item. Objects whose answer did not change are left alone, so the common case of a quest
/// that touches nothing on screen sends nothing at all.
pub fn refresh_quest_gameobjects(player_guid: ObjectGuid, world: &World) {
    let Some((visible, previously_activated)) =
        world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                let visible: Vec<ObjectGuid> = player
                    .visibility
                    .visible_objects
                    .iter()
                    .copied()
                    .filter(|guid| guid.is_game_object())
                    .collect();
                (visible, player.visibility.gameobjects_activated.clone())
            })
    else {
        return;
    };

    let mut activated_now = HashSet::new();
    let mut changed: Vec<(ObjectGuid, bool)> = Vec::new();
    for guid in visible {
        let activated = gameobject_activates_to_quest(guid, player_guid, world);
        if activated {
            activated_now.insert(guid);
        }
        if activated != previously_activated.contains(&guid) {
            changed.push((guid, activated));
        }
    }

    // Store even when nothing changed: objects that left visibility have to drop out of the set,
    // or a later re-entry would compare against a stale answer.
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.visibility.gameobjects_activated = activated_now;
        });

    if changed.is_empty() {
        return;
    }

    let mut msg = SmsgUpdateObject::new();
    for (guid, activated) in &changed {
        let dyn_flags = if *activated {
            go_dyn_flags::GO_DYNFLAG_LO_ACTIVATE
        } else {
            0
        };
        let anim_progress = world
            .managers
            .gameobject_mgr
            .with_gameobject(*guid, |go| go.anim_progress)
            .unwrap_or(0);

        msg = msg.add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(*guid, ObjectType::GameObject)
                .set_field(GAMEOBJECT_DYN_FLAGS, dyn_flags)
                // The 1.12 client wants the animation progress alongside a dynamic-flag change on
                // a gameobject; sent without it, the flag update does not take.
                .set_field(GAMEOBJECT_ANIMPROGRESS, anim_progress),
        ));
    }

    if let Some(broadcaster) = world.managers.player_mgr.get_broadcaster(player_guid) {
        if let Err(error) = broadcaster.send_update_object(&msg) {
            tracing::warn!(
                "Failed to send quest activation update to {:?}: {}",
                player_guid,
                error
            );
        }
    }

    tracing::debug!(
        "Quest activation changed for {} gameobject(s) visible to {:?}",
        changed.len(),
        player_guid
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::gameobject::{GameObject, GameObjectTemplate};
    use crate::game::npc::quest::types::{QuestProgress, QuestTemplate};
    use crate::game::player::broadcaster::PlayerBroadcaster;
    use crate::game::player::Player;
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::{HighGuid, Position, WorldPacket};
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn lazy_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });

        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn add_player(world: &World) -> (ObjectGuid, mpsc::UnboundedReceiver<WorldPacket>) {
        let guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
        let (tx, rx) = mpsc::unbounded_channel();
        let mut player = Player::new(guid, "Tester".to_string(), 0, 0, 0, 10, 1, 1, 0);
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(tx, guid)));
        world.managers.player_mgr.add_player(player, 1);
        (guid, rx)
    }

    /// Spawn a gameobject of `go_type` whose template carries `data`, and return its guid.
    fn add_gameobject(
        world: &World,
        entry: u32,
        go_type: GameObjectType,
        data: [i32; 24],
    ) -> ObjectGuid {
        let template = GameObjectTemplate {
            entry,
            go_type: go_type as u32,
            display_id: 1,
            name: format!("GameObject {entry}"),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data,
        };
        let guid = ObjectGuid::new_gameobject(entry, 1);
        world
            .managers
            .gameobject_mgr
            .add_template_for_test(template.clone());
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(GameObject::new(
                guid,
                entry,
                1,
                Position::default(),
                0,
                &template,
                [0.0; 4],
                1,
                100,
            ));
        guid
    }

    /// Give the player quest `quest_id`, optionally as an objective naming `go_entry`.
    ///
    /// Every quest here carries an unmet creature objective in slot 1, because activation asks for
    /// `Incomplete` specifically: a quest with no objectives at all reads as already complete.
    fn give_quest(world: &World, player_guid: ObjectGuid, quest_id: u32, go_entry: Option<u32>) {
        let mut template = QuestTemplate {
            id: quest_id,
            title: format!("Quest {quest_id}"),
            is_active: true,
            ..QuestTemplate::default()
        };
        template.req_creature_or_go_id[1] = 999; // positive: a creature, never satisfied here
        template.req_creature_or_go_count[1] = 5;
        if let Some(entry) = go_entry {
            template.req_creature_or_go_id[0] = -(entry as i32);
            template.req_creature_or_go_count[0] = 5;
        }
        world.systems.quest.manager.add_quest_template(template);
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.active_quests.push(QuestProgress::new(quest_id));
            });
    }

    fn quest_id_data(index: usize, quest_id: i32) -> [i32; 24] {
        let mut data = [0; 24];
        data[index] = quest_id;
        data
    }

    /// A goober naming an incomplete quest is exactly the case the old chest-only rule missed:
    /// nothing about it involves loot, so it never lit up for anybody.
    #[tokio::test]
    async fn goober_with_an_incomplete_quest_activates() {
        let world = test_world();
        let (player_guid, _rx) = add_player(&world);
        let go = add_gameobject(&world, 300, GameObjectType::Goober, quest_id_data(1, 42));

        assert!(!gameobject_activates_to_quest(go, player_guid, &world));
        give_quest(&world, player_guid, 42, None);
        assert!(gameobject_activates_to_quest(go, player_guid, &world));
    }

    /// Generic and spell-focus objects read their quest from a different `data` column each. Read
    /// at the wrong offset they look quest-less, which is why the offsets are per type.
    #[tokio::test]
    async fn generic_and_spell_focus_read_their_own_quest_column() {
        let world = test_world();
        let (player_guid, _rx) = add_player(&world);
        let generic = add_gameobject(&world, 301, GameObjectType::Generic, quest_id_data(5, 43));
        let focus = add_gameobject(
            &world,
            302,
            GameObjectType::SpellFocus,
            quest_id_data(4, 44),
        );

        give_quest(&world, player_guid, 43, None);
        give_quest(&world, player_guid, 44, None);

        assert!(gameobject_activates_to_quest(generic, player_guid, &world));
        assert!(gameobject_activates_to_quest(focus, player_guid, &world));
    }

    /// Being an objective of the player's own quest activates an object that names no quest of its
    /// own -- and stops doing so once the objective is finished.
    #[tokio::test]
    async fn an_objective_gameobject_activates_until_its_count_is_met() {
        let world = test_world();
        let (player_guid, _rx) = add_player(&world);
        let go = add_gameobject(&world, 303, GameObjectType::Goober, [0; 24]);
        give_quest(&world, player_guid, 45, Some(303));

        assert!(gameobject_activates_to_quest(go, player_guid, &world));

        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.active_quests[0].creature_or_go_count[0] = 5;
            });
        assert!(!gameobject_activates_to_quest(go, player_guid, &world));
    }

    /// An object with no quest connection at all stays inert. The flag only ever unlocks
    /// conditional objects; a plain chest is clickable through its own type.
    #[tokio::test]
    async fn an_ordinary_gameobject_never_activates() {
        let world = test_world();
        let (player_guid, _rx) = add_player(&world);
        let chest = add_gameobject(&world, 304, GameObjectType::Chest, [0; 24]);
        let door = add_gameobject(&world, 305, GameObjectType::Door, [0; 24]);

        assert!(!gameobject_activates_to_quest(chest, player_guid, &world));
        assert!(!gameobject_activates_to_quest(door, player_guid, &world));
    }

    /// The bug this fixes: an object already on screen when the quest is accepted got no create
    /// block, so nothing ever told the client it had become clickable.
    #[tokio::test]
    async fn refresh_updates_an_object_that_was_already_visible() {
        let world = test_world();
        let (player_guid, mut rx) = add_player(&world);
        let go = add_gameobject(&world, 306, GameObjectType::Goober, quest_id_data(1, 46));
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.visibility.visible_objects.insert(go);
            });

        // Nothing to say before the quest exists.
        refresh_quest_gameobjects(player_guid, &world);
        assert!(rx.try_recv().is_err(), "no quest state changed");

        give_quest(&world, player_guid, 46, None);
        refresh_quest_gameobjects(player_guid, &world);

        let packet = rx.try_recv().expect("the object turned clickable");
        assert_eq!(
            packet.opcode(),
            oxcore_shared::protocol::Opcode::SMSG_UPDATE_OBJECT
        );
        assert!(world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p
                .visibility
                .gameobjects_activated
                .contains(&go))
            .unwrap_or(false));

        // Saying it twice is just noise on the wire.
        refresh_quest_gameobjects(player_guid, &world);
        assert!(rx.try_recv().is_err(), "activation did not change");
    }

    /// Objects that leave visibility drop out of the remembered set, so walking back to one
    /// compares against a fresh answer rather than a stale one.
    #[tokio::test]
    async fn refresh_forgets_objects_that_left_visibility() {
        let world = test_world();
        let (player_guid, _rx) = add_player(&world);
        let go = add_gameobject(&world, 307, GameObjectType::Goober, quest_id_data(1, 47));
        give_quest(&world, player_guid, 47, None);
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.visibility.visible_objects.insert(go);
            });
        refresh_quest_gameobjects(player_guid, &world);

        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.visibility.visible_objects.remove(&go);
            });
        refresh_quest_gameobjects(player_guid, &world);

        assert!(!world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p
                .visibility
                .gameobjects_activated
                .contains(&go))
            .unwrap_or(true));
    }
}
