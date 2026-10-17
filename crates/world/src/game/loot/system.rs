use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::game::common::update_fields::GAMEOBJECT_STATE;
use crate::game::gameobject::{GOState, GameObjectType, LootState};
use crate::game::items::ItemManager;
use crate::game::group::LootMethod;
use crate::game::loot::manager::LootManager;
use crate::game::loot::manager::LootStore;
use crate::game::loot::types::{Loot, LootItem, LootKind};
use crate::game::player::PlayerManager;
use crate::World;
use oxcore_shared::messages::loot::*;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

/// What a viewer may do with a loot window's shared (non-quest) items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LootPermission {
    /// Free-for-all groups, chests, and other unrestricted loot.
    All,
    /// The solo tapper.
    Owner,
    /// Group loot / need-before-greed member: rolled items are shown as rolling, under-threshold
    /// items only to this corpse's round-robin owner.
    Group,
    /// Round-robin member: everything belongs to this corpse's round-robin owner.
    RoundRobin,
    /// Master-loot member: over-threshold items are handed out by the master looter.
    Master,
    /// Not entitled to anything.
    None,
}

/// Loot slot types on the 1.12 wire. The 1.14 encoder renumbers them.
const SLOT_ALLOW_LOOT: u8 = 0;
const SLOT_ROLL_ONGOING: u8 = 1;
const SLOT_MASTER: u8 = 2;

/// How far a player may stand from a corpse or chest and still loot it. Generous on purpose:
/// the client already enforces interaction range, this only stops looting from across a room.
const LOOT_DISTANCE: f32 = 15.0;

/// A looted corpse lingers this long before it decays (half the normal corpse time).
const LOOTED_CORPSE_DECAY_MS: u32 = 30_000;

/// Loot types on the 1.12 wire. Pockets and skins share one (the client has no skinning type).
const LOOT_TYPE_CORPSE: u8 = 1;
const LOOT_TYPE_PICKPOCKETING: u8 = 2;
const LOOT_TYPE_DISENCHANTING: u8 = 4;

/// Item flag: the item can be opened for loot ("Right click to open").
const ITEM_FLAG_LOOTABLE: u32 = 0x0000_0004;
/// Item instance flag: a locked item has been picked or unlocked.
const ITEM_DYNFLAG_UNLOCKED: u32 = 0x0000_0004;

/// Chests more than this many levels above the player cannot be opened.
const CHEST_LEVEL_ALLOWANCE: i32 = 10;

/// LootSystem - handles all loot business logic and packet sending
pub struct LootSystem {
    manager: Arc<LootManager>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>, // OWNS broadcast_mgr
    item_mgr: Arc<ItemManager>,
    player_mgr: Arc<PlayerManager>,
}

impl LootSystem {
    pub fn new(
        manager: Arc<LootManager>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>, // INJECTED
        item_mgr: Arc<ItemManager>,
        player_mgr: Arc<PlayerManager>,
    ) -> Self {
        Self {
            manager,
            broadcast_mgr, // STORED
            item_mgr,
            player_mgr,
        }
    }

    /// Initialize the loot system
    pub async fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Shutdown the loot system
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Handle CMSG_LOOT - full business logic here
    pub async fn handle_loot_request(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        self.handle_loot_request_with_type(player_guid, target_guid, 1, world)
            .await
    }

    /// Open loot through `SPELL_EFFECT_OPEN_LOCK`.
    ///
    /// Passes a loot type the client-facing packet converts to `LOOT_PICKPOCKETING` (2).
    pub async fn handle_open_lock_loot(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        self.handle_loot_request_with_type(player_guid, target_guid, 2, world)
            .await
    }

    async fn handle_loot_request_with_type(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        loot_type: u8,
        world: &World,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "[LOOT] handle_loot_request: player={:?} target={:?}",
            player_guid,
            target_guid
        );

        // 0. The player must be able to loot at all, and be close enough.
        if let Some(error) = self.open_error(player_guid, target_guid, world) {
            self.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgLootError {
                    loot_guid: target_guid,
                    error,
                },
            );
            return Ok(());
        }

        // 1. Validate access
        if !self.can_loot(player_guid, target_guid, world) {
            tracing::info!(
                "[LOOT] can_loot failed for player={:?} target={:?}",
                player_guid,
                target_guid
            );
            // Send release so the client cursor resets cleanly
            let msg = SmsgLootReleaseResponse {
                loot_guid: target_guid,
                unknown: 1,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // 2. Generate loot if needed
        if !self.manager.has_loot(target_guid) {
            tracing::info!("[LOOT] Generating loot for target={:?}", target_guid);
            if target_guid.is_game_object() {
                self.generate_gameobject_loot(target_guid, world)?;
            } else {
                self.generate_creature_loot(target_guid, world).await?;
            }
        }

        // 3. Group rolls start the first time a tap-group member opens the corpse, so the
        // rolled items already show as rolling in this window.
        if !target_guid.is_game_object() {
            self.start_group_loot(player_guid, target_guid, world).await;
        }

        // Mark this player as viewing the loot
        self.manager.add_viewer(target_guid, player_guid);

        if target_guid.is_game_object() {
            self.set_gameobject_state(player_guid, target_guid, GOState::Active, world);
        }

        // 4. Set player's looting target
        self.player_mgr.set_looting_target(player_guid, target_guid);
        self.set_looting_flag(player_guid, true);

        // 5. Build and send loot response
        tracing::info!("[LOOT] Sending loot window to player={:?}", player_guid);
        self.send_loot_window(player_guid, target_guid, loot_type, world);

        Ok(())
    }

    /// Handle CMSG_LOOT_MONEY
    ///
    /// A grouped looter splits the coins evenly with every group member within loot distance of
    /// them (the remainder is lost, as on retail); everyone viewing the window sees the coins go.
    pub async fn handle_loot_money(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        if !self.within_loot_range(player_guid, target_guid, world) {
            return Ok(());
        }
        let gold = self.manager.take_gold(target_guid);

        if gold > 0 {
            let mut viewers = self.manager.viewers(target_guid);
            if !viewers.contains(&player_guid) {
                viewers.push(player_guid);
            }
            self.broadcast_mgr
                .broadcast_msg_to_players(
                    &viewers,
                    &SmsgLootClearMoney {
                        loot_guid: target_guid,
                    },
                );

            // Pickpocketed coins belong to the thief alone.
            let pickpocket = self.manager.get_loot(target_guid).is_some_and(|l| {
                matches!(l.kind, LootKind::Pickpocket | LootKind::Item | LootKind::Disenchant)
            });
            let sharers = if pickpocket {
                vec![player_guid]
            } else {
                self.money_sharers(player_guid, world)
            };
            if sharers.len() > 1 {
                let share = gold / sharers.len() as u32;
                for guid in sharers {
                    self.player_mgr.add_money(guid, share).await?;
                    let msg = SmsgLootMoneyNotify {
                        gold: share,
                        sole_looter: false,
                    };
                    self.broadcast_mgr.send_msg_to_player(guid, msg);
                }
            } else {
                self.player_mgr.add_money(player_guid, gold).await?;
                let msg = SmsgLootMoneyNotify {
                    gold,
                    sole_looter: true,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            }
        }

        // Check if loot is empty
        self.check_and_clear_if_empty(target_guid, world);

        Ok(())
    }

    /// Group members who share a coin drop with `looter`: online, on the same map and instance,
    /// and within the group reward distance of the looter. Just the looter when ungrouped.
    fn money_sharers(&self, looter: ObjectGuid, world: &World) -> Vec<ObjectGuid> {
        let Some(group) = world.systems.group.get_player_group(looter) else {
            return vec![looter];
        };
        let Some((map_id, instance_id, position)) = self
            .player_mgr
            .get_player(looter)
            .map(|p| (p.map_id, p.instance_id, p.movement.position))
        else {
            return vec![looter];
        };
        let range = world.config.group_xp_distance;
        let mut sharers: Vec<ObjectGuid> = group
            .members
            .iter()
            .filter(|m| {
                m.guid == looter
                    || self.player_mgr.get_player(m.guid).is_some_and(|p| {
                        p.map_id == map_id
                            && p.instance_id == instance_id
                            && p.movement.position.is_within_range(&position, range)
                    })
            })
            .map(|m| m.guid)
            .collect();
        if !sharers.contains(&looter) {
            sharers.push(looter);
        }
        sharers
    }

    /// Handle CMSG_AUTOSTORE_LOOT_ITEM
    ///
    /// `slot` is the wire slot the player was shown: a normal item's storage index, or a quest
    /// item numbered after the normal items by its position in that player's quest list.
    pub async fn handle_loot_item(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        slot: u8,
        world: &World,
    ) -> anyhow::Result<()> {
        use crate::game::inventory::types::AddItemResult;

        if !self.within_loot_range(player_guid, target_guid, world) {
            self.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgLootError {
                    loot_guid: target_guid,
                    error: loot_error::TOO_FAR,
                },
            );
            return Ok(());
        }

        // Look at the item without consuming it — only mark it looted once the
        // item is actually stored, otherwise a full bag destroys the drop.
        let Some((is_quest, item)) = self.manager.peek_item(target_guid, slot, player_guid) else {
            tracing::info!(
                "[LOOT] handle_loot_item: player={:?} target={:?} slot={} not found (already looted, wrong slot, or not visible to this player)",
                player_guid,
                target_guid,
                slot
            );
            return Ok(());
        };

        // The same rules decide what the window offers and what may be taken: rolling items,
        // master-loot items, and another member's round-robin items are refused.
        if !self.can_take(player_guid, target_guid, is_quest, &item, world) {
            tracing::info!(
                "[LOOT] handle_loot_item: player={:?} may not take slot {} of {:?}",
                player_guid,
                slot,
                target_guid
            );
            return Ok(());
        }

        tracing::info!(
            "[LOOT] handle_loot_item: player={:?} target={:?} slot={} item_id={} count={} -> add_item",
            player_guid,
            target_guid,
            slot,
            item.item_id,
            item.count
        );

        // Add to player inventory
        let result = world
            .systems
            .inventory
            .add_item(player_guid, item.item_id, item.count)
            .await;

        match result {
            AddItemResult::Success { .. } => {
                // Stored — now remove it from the loot
                self.manager.loot_item(target_guid, slot, player_guid);

                // Notify quest system about item gain
                world
                    .systems
                    .quest
                    .handle_item_added(player_guid, item.item_id, item.count);

                // Collecting the last of a quest item takes its chests back out of play, and the
                // first of one can bring a follow-up objective's objects in.
                crate::game::gameobject::quest_activation::refresh_quest_gameobjects(
                    player_guid,
                    world,
                );

                self.notify_item_removed(target_guid, is_quest, item.slot, player_guid);

                // Check if loot is now empty
                self.check_and_clear_if_empty(target_guid, world);
            }
            _ => {
                // Item stays in the loot so the player can retry after making room
                tracing::warn!(
                    "[LOOT] handle_loot_item: add_item FAILED for item {} count {} player {:?} (loot slot {} left intact): {:?}",
                    item.item_id,
                    item.count,
                    player_guid,
                    slot,
                    result
                );
            }
        }

        Ok(())
    }

    /// Tell every viewer of `target_guid` that an item left the loot.
    ///
    /// A normal item has the same wire slot for everyone. A quest item is numbered per player,
    /// so each viewer who was shown it is told at their own slot.
    pub fn notify_item_removed(
        &self,
        target_guid: ObjectGuid,
        is_quest: bool,
        storage_slot: u8,
        looter: ObjectGuid,
    ) {
        let mut viewers = self.manager.viewers(target_guid);
        if !viewers.contains(&looter) {
            viewers.push(looter);
        }

        if !is_quest {
            let msg = SmsgLootRemoved {
                loot_guid: target_guid,
                slot: storage_slot,
            };
            self.broadcast_mgr.broadcast_msg_to_players(&viewers, &msg);
            return;
        }

        let Some(loot) = self.manager.get_loot(target_guid) else {
            return;
        };
        let per_viewer: Vec<(ObjectGuid, u8)> = viewers
            .iter()
            .filter_map(|v| loot.quest_wire_slot(*v, storage_slot).map(|s| (*v, s)))
            .collect();
        drop(loot);
        for (viewer, slot) in per_viewer {
            self.broadcast_mgr.send_msg_to_player(
                viewer,
                SmsgLootRemoved {
                    loot_guid: target_guid,
                    slot,
                },
            );
        }
    }

    /// Handle CMSG_LOOT_RELEASE
    pub async fn handle_loot_release(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        if target_guid.is_game_object() {
            self.set_gameobject_state(player_guid, target_guid, GOState::Ready, world);
        }

        // This player no longer sees the window
        self.manager.remove_viewer(target_guid, player_guid);

        // Clear player's looting target
        self.player_mgr.clear_looting_target(player_guid);
        self.set_looting_flag(player_guid, false);

        // Send release response
        let msg = SmsgLootReleaseResponse {
            loot_guid: target_guid,
            unknown: 1,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        // A disenchanted item is always used up; a lootable item once it is empty. A partly
        // emptied item keeps its loot for the next opening.
        let item_kind = self
            .manager
            .get_loot(target_guid)
            .map(|l| l.kind)
            .filter(|k| matches!(k, LootKind::Item | LootKind::Disenchant));
        if let Some(kind) = item_kind {
            if kind == LootKind::Disenchant || self.manager.is_loot_empty(target_guid) {
                self.manager.remove_loot(target_guid);
                let _ = world
                    .systems
                    .inventory
                    .remove_item(player_guid, target_guid, 1);
            }
            return Ok(());
        }

        // Reference consumes a fully looted chest only after CMSG_LOOT_RELEASE.
        if target_guid.is_game_object() && self.manager.is_loot_empty(target_guid) {
            crate::game::gameobject::system::despawn_looted_gameobject(
                player_guid,
                target_guid,
                world,
            );
            return Ok(());
        }

        // The round-robin owner walking away from a corpse with loot left opens the remainder to
        // the rest of the group (master loot keeps its own rules).
        if !target_guid.is_game_object() && !self.manager.is_loot_empty(target_guid) {
            let method = world
                .systems
                .group
                .get_player_group(player_guid)
                .map(|g| g.loot_method);
            if method != Some(LootMethod::MasterLooter) {
                let released = self
                    .manager
                    .with_loot_mut(target_guid, |loot| {
                        if loot.round_robin_player == Some(player_guid) {
                            loot.round_robin_player = None;
                            Some(loot.allowed_looters.clone())
                        } else {
                            None
                        }
                    })
                    .flatten();
                // 1.14 clients learn the corpse is now open to everyone.
                if let Some(looters) = released {
                    world
                        .systems
                        .group
                        .send_loot_list(target_guid, None, None, &looters);
                }
            }
        }

        self.check_and_clear_if_empty(target_guid, world);

        Ok(())
    }

    /// Start group rolls the first time a tap-group member opens the corpse, and send the
    /// master-loot candidate list to whoever opens it under master loot.
    async fn start_group_loot(&self, player_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) {
        let Some((group_id, started)) = self
            .manager
            .get_loot(target_guid)
            .map(|loot| (loot.group_id, loot.group_loot_started))
        else {
            return;
        };
        let Some(group_id) = group_id else {
            return;
        };
        if world.systems.group.get_player_group_id(player_guid) != Some(group_id) {
            return;
        }
        let Some(method) = world
            .systems
            .group
            .get_group(group_id)
            .map(|g| g.loot_method)
        else {
            return;
        };
        let Some(context) = crate::game::group::GroupSystem::loot_context_for(world, target_guid)
        else {
            return;
        };

        if !started {
            self.manager.with_loot_mut(target_guid, |loot| {
                loot.group_loot_started = true;
            });
            match method {
                LootMethod::GroupLoot => {
                    world
                        .systems
                        .group
                        .group_loot(world, group_id, target_guid, context.clone())
                        .await;
                }
                LootMethod::NeedBeforeGreed => {
                    world
                        .systems
                        .group
                        .need_before_greed(world, group_id, target_guid, context.clone())
                        .await;
                }
                _ => {}
            }
        }

        if method == LootMethod::MasterLooter {
            world
                .systems
                .group
                .master_loot(world, group_id, target_guid, player_guid, context)
                .await;
        }
    }

    /// Why `player_guid` cannot open `target_guid` right now, if anything stops them.
    fn open_error(&self, player_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) -> Option<u8> {
        use crate::game::common::unit_flags::STUNNED;
        use crate::game::player::death::DeathState;

        let (alive, stand_state, unit_flags, level) = self.player_mgr.with_player(player_guid, |p| {
            (
                matches!(p.death.death_state, DeathState::Alive),
                p.stand_state,
                p.unit_flags,
                p.level,
            )
        })?;
        if !alive {
            return Some(loot_error::PLAYER_NOT_FOUND);
        }
        if !target_guid.is_game_object() {
            // Opening a corpse (not a chest through a spell) requires standing and not stunned.
            if stand_state != 0 {
                return Some(loot_error::NOT_STANDING);
            }
            if unit_flags & STUNNED != 0 {
                return Some(loot_error::STUNNED);
            }
        } else {
            let chest_level = world
                .managers
                .gameobject_mgr
                .with_gameobject(target_guid, |go| {
                    world
                        .managers
                        .gameobject_mgr
                        .get_template(go.entry)
                        .filter(|t| t.go_type == GameObjectType::Chest as u32)
                        .map(|t| t.data[9])
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            if chest_level > i32::from(level) + CHEST_LEVEL_ALLOWANCE {
                return Some(loot_error::DIDNT_KILL);
            }
        }
        if !self.within_loot_range(player_guid, target_guid, world) {
            return Some(loot_error::TOO_FAR);
        }
        None
    }

    /// Whether the player stands close enough to a corpse or chest to loot it. Unknown
    /// positions pass, so a missing object never strands an open window.
    fn within_loot_range(&self, player_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) -> bool {
        let Some((map_id, position)) = self
            .player_mgr
            .with_player(player_guid, |p| (p.map_id, p.movement.position))
        else {
            return false;
        };
        let target = if target_guid.is_game_object() {
            world
                .managers
                .gameobject_mgr
                .with_gameobject(target_guid, |go| (go.map_id, go.position))
        } else {
            world
                .managers
                .creature_mgr
                .with_creature(target_guid, |c| (c.map_id, c.position))
        };
        match target {
            Some((target_map, target_pos)) => {
                target_map == map_id && position.is_within_range(&target_pos, LOOT_DISTANCE)
            }
            None => true,
        }
    }

    /// Set or clear the looting animation flag on a player and show it to everyone nearby.
    fn set_looting_flag(&self, player_guid: ObjectGuid, on: bool) {
        use crate::game::common::unit_flags::LOOTING;
        use crate::game::common::update_fields::UNIT_FIELD_FLAGS;
        use oxcore_shared::messages::update::{
            ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
        };

        let flags = {
            let Some(mut player) = self.player_mgr.get_player_mut(player_guid) else {
                return;
            };
            let flags = if on {
                player.unit_flags | LOOTING
            } else {
                player.unit_flags & !LOOTING
            };
            if flags == player.unit_flags {
                return;
            }
            player.unit_flags = flags;
            flags
        };
        let world_guid = crate::core::common::guid::ObjectGuid::from_raw(player_guid.raw());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                // Players always carry PLAYER_CONTROLLED; it is not stored in `unit_flags`.
                .set_field(UNIT_FIELD_FLAGS, flags | crate::game::common::unit_flags::PLAYER_CONTROLLED),
        ));
        self.broadcast_mgr
            .broadcast_msg_nearby(player_guid, &update, true);
    }

    /// A non-repeatable quest's starter item is hidden from a player who already has or did
    /// that quest, unless one of their quests needs the item itself.
    pub fn quest_starter_hidden(&self, world: &World, player_guid: ObjectGuid, item_id: u32) -> bool {
        let Some(start_quest) = self
            .item_mgr
            .get_template(item_id)
            .map(|t| t.start_quest)
            .filter(|q| *q != 0)
        else {
            return false;
        };
        let repeatable = world
            .systems
            .quest
            .manager
            .get_quest_template(start_quest)
            .map(|q| q.is_repeatable())
            .unwrap_or(true);
        if repeatable {
            return false;
        }
        let known = self
            .player_mgr
            .with_player(player_guid, |p| {
                p.rewarded_quests.contains(&start_quest)
                    || p.active_quests.iter().any(|q| q.quest_id == start_quest)
            })
            .unwrap_or(false);
        known
            && !world
                .systems
                .quest
                .player_has_quest_for_item(player_guid, item_id)
    }

    /// The permission a viewer has over a loot's shared items.
    fn permission_for(
        &self,
        loot: &Loot,
        viewer: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> LootPermission {
        if target_guid.is_game_object() {
            return LootPermission::All;
        }
        // A tap group that has since disbanded falls back to the tapper alone.
        let group = loot
            .group_id
            .and_then(|gid| world.systems.group.get_group(gid));
        match group {
            Some(group) => {
                if !group.has_member(viewer) {
                    return LootPermission::None;
                }
                match group.loot_method {
                    LootMethod::FreeForAll => LootPermission::All,
                    LootMethod::RoundRobin => LootPermission::RoundRobin,
                    LootMethod::MasterLooter => LootPermission::Master,
                    LootMethod::GroupLoot | LootMethod::NeedBeforeGreed => LootPermission::Group,
                }
            }
            None => {
                if loot.recipient.map_or(true, |r| r == viewer) {
                    LootPermission::Owner
                } else {
                    LootPermission::None
                }
            }
        }
    }

    /// Slot type a viewer is shown for a shared item, or `None` when it is hidden from them.
    fn slot_type_for(
        permission: LootPermission,
        item: &LootItem,
        viewer: ObjectGuid,
        round_robin_player: Option<ObjectGuid>,
    ) -> Option<u8> {
        if item.is_looted || !Loot::item_allowed_for(item, viewer) {
            return None;
        }
        let owns_round_robin = round_robin_player.map_or(true, |rr| rr == viewer);
        match permission {
            LootPermission::Group => {
                if item.is_blocked {
                    Some(SLOT_ROLL_ONGOING)
                } else if owns_round_robin || !item.is_underthreshold {
                    Some(SLOT_ALLOW_LOOT)
                } else {
                    None
                }
            }
            LootPermission::RoundRobin => owns_round_robin.then_some(SLOT_ALLOW_LOOT),
            LootPermission::Master => Some(if item.is_underthreshold {
                SLOT_ALLOW_LOOT
            } else {
                SLOT_MASTER
            }),
            LootPermission::All | LootPermission::Owner => Some(SLOT_ALLOW_LOOT),
            LootPermission::None => None,
        }
    }

    /// Whether a quest item is offered to `viewer`: it must be in their quest list, and while a
    /// round-robin owner who also needs it holds the corpse, it is theirs.
    fn quest_item_visible(loot: &Loot, viewer: ObjectGuid, quest_slot: u8) -> bool {
        let Some(visible) = loot.visible_quest_slots(viewer) else {
            return false;
        };
        if !visible.contains(&quest_slot) {
            return false;
        }
        match loot.round_robin_player {
            Some(rr) if rr != viewer => !loot.player_quest_items.contains_key(&rr),
            _ => true,
        }
    }

    /// Whether `player` may autostore this item: exactly the items their window offers as
    /// lootable.
    fn can_take(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        is_quest: bool,
        item: &LootItem,
        world: &World,
    ) -> bool {
        let Some(loot) = self.manager.get_loot(target_guid) else {
            return false;
        };
        let permission = self.permission_for(&loot, player_guid, target_guid, world);
        if permission == LootPermission::None {
            return false;
        }
        if is_quest {
            return Self::quest_item_visible(&loot, player_guid, item.slot);
        }
        Self::slot_type_for(permission, item, player_guid, loot.round_robin_player)
            == Some(SLOT_ALLOW_LOOT)
            && !self.quest_starter_hidden(world, player_guid, item.item_id)
    }

    /// Generate loot for a creature on death. A tapped creature with skinning loot becomes
    /// skinnable straight away; the skinning spell still requires the body to be looted first.
    pub async fn generate_creature_loot_on_death(
        &self,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        self.generate_creature_loot(target_guid, world).await?;

        let skinnable = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |c| (c.entry, c.loot_recipient.is_some()))
            .filter(|(_, tapped)| *tapped)
            .and_then(|(entry, _)| world.managers.creature_mgr.get_template(entry))
            .is_some_and(|t| self.manager.has_store_loot(LootStore::Skinning, t.skinning_loot_id));
        if skinnable {
            self.set_creature_skinnable(target_guid, true, world);
        }
        Ok(())
    }

    /// Set or clear UNIT_FLAG_SKINNABLE on a corpse and show it to players nearby.
    fn set_creature_skinnable(&self, target_guid: ObjectGuid, on: bool, world: &World) {
        use crate::game::common::unit_flags::SKINNABLE;
        use crate::game::common::update_fields::UNIT_FIELD_FLAGS;
        use oxcore_shared::messages::update::{
            ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
        };
        let Some(flags) = world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |c| {
                if on {
                    c.unit_flags |= SKINNABLE;
                } else {
                    c.unit_flags &= !SKINNABLE;
                }
                c.unit_flags
            })
        else {
            return;
        };
        let world_guid =
            crate::core::common::guid::ObjectGuid::new_creature(target_guid.entry(), target_guid.counter());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Unit).set_field(UNIT_FIELD_FLAGS, flags),
        ));
        crate::game::broadcast_mgr::broadcast_around_creature(world, target_guid, &update);
    }

    /// Open a living creature's pockets for a rogue (SPELL_EFFECT_PICKPOCKET).
    ///
    /// The first pick fills the pockets — the pickpocketing template plus a few coins scaled by
    /// both levels — and they stay picked until the creature respawns. Only the thief may loot
    /// them, and the coins are not shared with the group.
    pub async fn open_pickpocket_loot(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        use crate::game::creature::DeathState;
        use rand::Rng;

        let Some((alive, entry, level, picked)) = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |c| {
                (
                    matches!(c.death_state, DeathState::Alive),
                    c.entry,
                    c.level,
                    c.pickpocketed,
                )
            })
        else {
            return Ok(());
        };
        if !alive {
            return Ok(());
        }

        let own_pockets = self
            .manager
            .get_loot(target_guid)
            .is_some_and(|l| l.kind == LootKind::Pickpocket && l.recipient == Some(player_guid));
        if picked && !own_pockets {
            self.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgLootError {
                    loot_guid: target_guid,
                    error: loot_error::ALREADY_PICKPOCKETED,
                },
            );
            return Ok(());
        }

        if !picked {
            let loot_id = world
                .managers
                .creature_mgr
                .get_template(entry)
                .map(|t| t.pickpocket_loot_id)
                .unwrap_or(0);
            let player_level = self.player_mgr.with_player(player_guid, |p| p.level).unwrap_or(1);
            let mut rng = rand::thread_rng();
            let gold = 10
                * (rng.gen_range(0..=u32::from(level) / 2)
                    + rng.gen_range(0..=u32::from(player_level) / 2));
            self.manager.generate_store_loot(
                target_guid,
                LootStore::Pickpocketing,
                loot_id,
                gold,
                LootKind::Pickpocket,
                player_guid,
            );
            world
                .managers
                .creature_mgr
                .with_creature_mut(target_guid, |c| c.pickpocketed = true);
        }

        self.open_window(player_guid, target_guid, LOOT_TYPE_PICKPOCKETING, world);
        Ok(())
    }

    /// Skin a looted corpse (SPELL_EFFECT_SKINNING): fill the skinning template, clear the
    /// skinnable flag, and open the window for the skinner.
    pub async fn open_skinning_loot(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        use crate::game::common::unit_flags::SKINNABLE;
        use crate::game::creature::DeathState;
        use crate::game::player::skills::constants::SKILL_SKINNING;

        let Some((corpse, skinnable, entry, level)) = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |c| {
                (
                    matches!(c.death_state, DeathState::JustDied | DeathState::Corpse),
                    c.unit_flags & SKINNABLE != 0,
                    c.entry,
                    c.level,
                )
            })
        else {
            return Ok(());
        };
        // The body must be looted first; an existing skinning loot can simply be reopened.
        let body_left = self
            .manager
            .get_loot(target_guid)
            .is_some_and(|l| l.kind == LootKind::Body && !l.is_empty());
        let already_skinned = self
            .manager
            .get_loot(target_guid)
            .is_some_and(|l| l.kind == LootKind::Skinning);
        if !corpse || body_left || (!skinnable && !already_skinned) {
            return Ok(());
        }

        if !already_skinned {
            let template = world.managers.creature_mgr.get_template(entry);
            let loot_id = template.as_ref().map(|t| t.skinning_loot_id).unwrap_or(0);
            let elite = template.as_ref().is_some_and(|t| t.rank != 0 && t.rank != 4);
            self.manager.generate_store_loot(
                target_guid,
                LootStore::Skinning,
                loot_id,
                0,
                LootKind::Skinning,
                player_guid,
            );
            world
                .managers
                .creature_mgr
                .with_creature_mut(target_guid, |c| c.set_has_loot(true));
            self.set_creature_skinnable(target_guid, false, world);

            // Skinning skill: required skill by creature level, elites count double.
            let level = i32::from(level);
            let required = if level < 10 {
                0
            } else if level < 20 {
                (level - 10) * 10
            } else {
                level * 5
            };
            if let Some(skill) = world
                .systems
                .skills
                .get_skill_value(player_guid, SKILL_SKINNING, world)
            {
                world.systems.skills.update_gather_skill(
                    player_guid,
                    SKILL_SKINNING,
                    skill,
                    required.max(0) as u16,
                    if elite { 2 } else { 1 },
                    world,
                );
            }
        }

        self.open_window(player_guid, target_guid, LOOT_TYPE_PICKPOCKETING, world);
        Ok(())
    }

    /// Open a lootable item in the player's bags (CMSG_OPEN_ITEM): clams, lockboxes, bags of
    /// goods. A locked item must have been unlocked first. The loot is filled once and kept
    /// while the item exists, so a half-emptied item reopens with what is left.
    pub async fn open_item_loot(
        &self,
        player_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        world: &World,
    ) -> anyhow::Result<()> {
        use crate::game::player::death::DeathState;
        use oxcore_shared::messages::{
            SmsgInventoryChangeFailure, EQUIP_ERR_ITEM_LOCKED, EQUIP_ERR_ITEM_NOT_FOUND,
            EQUIP_ERR_YOU_ARE_DEAD,
        };

        let inventory = &world.systems.inventory;
        let found = inventory.get_item_at(player_guid, bag, slot).and_then(|guid| {
            inventory.cache().get_item(player_guid, guid).map(|item| {
                let item = item.read();
                (guid, item.entry, item.flags)
            })
        });
        let Some((item_guid, entry, item_flags)) = found else {
            self.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgInventoryChangeFailure::new(EQUIP_ERR_ITEM_NOT_FOUND),
            );
            return Ok(());
        };
        let Some(template) = self.item_mgr.get_template(entry) else {
            return Ok(());
        };
        let alive = self
            .player_mgr
            .with_player(player_guid, |p| matches!(p.death.death_state, DeathState::Alive))
            .unwrap_or(false);
        if !alive {
            self.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgInventoryChangeFailure::new(EQUIP_ERR_YOU_ARE_DEAD),
            );
            return Ok(());
        }
        if template.lock_id != 0 && item_flags & ITEM_DYNFLAG_UNLOCKED == 0 {
            self.broadcast_mgr.send_msg_to_player(
                player_guid,
                SmsgInventoryChangeFailure::new(EQUIP_ERR_ITEM_LOCKED),
            );
            return Ok(());
        }
        if template.flags & ITEM_FLAG_LOOTABLE == 0 && template.lock_id == 0 {
            return Ok(());
        }

        if !self.manager.has_loot(item_guid) {
            let gold = crate::game::loot::manager::roll_money(
                template.min_money_loot,
                template.max_money_loot,
            );
            self.manager.generate_store_loot(
                item_guid,
                LootStore::Item,
                entry,
                gold,
                LootKind::Item,
                player_guid,
            );
        }
        self.open_window(player_guid, item_guid, LOOT_TYPE_CORPSE, world);
        Ok(())
    }

    /// Disenchant an item (SPELL_EFFECT_DISENCHANT): show its disenchant loot. The item is
    /// destroyed when the window closes, whatever is left in it.
    pub async fn open_disenchant_loot(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        let Some(entry) = world
            .systems
            .inventory
            .cache()
            .get_item(player_guid, item_guid)
            .map(|item| item.read().entry)
        else {
            return Ok(());
        };
        let disenchant_id = self
            .item_mgr
            .get_template(entry)
            .map(|t| t.disenchant_id)
            .unwrap_or(0);
        if disenchant_id == 0 {
            return Ok(());
        }
        self.manager.generate_store_loot(
            item_guid,
            LootStore::Disenchant,
            disenchant_id,
            0,
            LootKind::Disenchant,
            player_guid,
        );
        self.open_window(player_guid, item_guid, LOOT_TYPE_DISENCHANTING, world);
        Ok(())
    }

    /// Show an already generated loot to `player_guid` as an open window.
    fn open_window(&self, player_guid: ObjectGuid, target_guid: ObjectGuid, loot_type: u8, world: &World) {
        self.manager.add_viewer(target_guid, player_guid);
        self.player_mgr.set_looting_target(player_guid, target_guid);
        self.set_looting_flag(player_guid, true);
        self.send_loot_window(player_guid, target_guid, loot_type, world);
    }

    /// Remove loot when corpse is removed
    pub fn remove_loot(&self, source_guid: ObjectGuid) {
        self.manager.remove_loot(source_guid);
    }

    // Private methods

    fn send_loot_window(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        loot_type: u8,
        world: &World,
    ) {
        let (loot_method, threshold) = world
            .systems
            .group
            .get_player_group(player_guid)
            .map(|g| (Into::<u8>::into(g.loot_method), g.loot_threshold))
            .unwrap_or((0, 0));
        // Decide (once per player) which quest drops this player may see. This also makes
        // them count as unlooted loot — quest items nobody needs never do.
        let visible_quest_slots =
            self.manager
                .fill_quest_loot(target_guid, player_guid, |item_id| {
                    world
                        .systems
                        .quest
                        .player_has_quest_for_item(player_guid, item_id)
                });

        let loot = self.manager.get_loot(target_guid);

        let Some(loot_ref) = loot else {
            tracing::warn!(
                "[LOOT] send_loot_window: no loot found for target={:?}",
                target_guid
            );
            return;
        };

        let permission = self.permission_for(&loot_ref, player_guid, target_guid, world);
        let round_robin_player = loot_ref.round_robin_player;

        let mut items = Vec::new();
        let gold = if permission == LootPermission::None {
            0
        } else {
            loot_ref.gold
        };

        // Slots are stable for the life of the window: a normal item keeps its storage index,
        // and quest items follow at `items.len() + position in this player's quest list`. The
        // client echoes these back unchanged when an item is clicked.
        for item in &loot_ref.items {
            let Some(slot_type) =
                Self::slot_type_for(permission, item, player_guid, round_robin_player)
            else {
                continue;
            };
            if self.quest_starter_hidden(world, player_guid, item.item_id) {
                continue;
            }

            let display_id = self
                .item_mgr
                .get_template(item.item_id)
                .map(|t| t.display_id)
                .unwrap_or(0);

            items.push(LootResponseItem {
                slot: item.slot,
                item_id: item.item_id,
                count: item.count,
                display_id,
                random_suffix: 0,
                random_property: 0,
                slot_type,
            });
        }

        // Add quest items only for players who have the required quest active.
        if permission != LootPermission::None {
            for &quest_slot in &visible_quest_slots {
                let Some(item) = loot_ref.get_quest_item(quest_slot) else {
                    continue; // already looted
                };
                if !Self::quest_item_visible(&loot_ref, player_guid, quest_slot) {
                    continue;
                }
                let Some(slot) = loot_ref.quest_wire_slot(player_guid, quest_slot) else {
                    continue;
                };

                let display_id = self
                    .item_mgr
                    .get_template(item.item_id)
                    .map(|t| t.display_id)
                    .unwrap_or(0);

                items.push(LootResponseItem {
                    slot,
                    item_id: item.item_id,
                    count: item.count,
                    display_id,
                    random_suffix: 0,
                    random_property: 0,
                    slot_type: SLOT_ALLOW_LOOT,
                });
            }
        }

        drop(loot_ref); // Release lock

        tracing::info!(
            "[LOOT] send_loot_window: player={:?} target={:?} gold={} items={:?}",
            player_guid,
            target_guid,
            gold,
            items
                .iter()
                .map(|i| (i.slot, i.item_id, i.count))
                .collect::<Vec<_>>()
        );

        // Build and send message
        let msg = SmsgLootResponse {
            loot_guid: target_guid,
            loot_type,
            gold,
            loot_method,
            threshold,
            items,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    async fn generate_creature_loot(
        &self,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        // Get creature info from the creature manager
        let creature_info = world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                (creature.entry, creature.loot_recipient)
            });

        let Some((entry, recipient)) = creature_info else {
            return Err(anyhow::anyhow!("Creature not found for loot generation"));
        };

        // Items come from the template's loot id (0 = no items), not the creature entry.
        let (loot_id, gold_min, gold_max) = world
            .managers
            .creature_mgr
            .get_template(entry)
            .map(|t| (t.loot_id, t.gold_min, t.gold_max))
            .unwrap_or((0, 0, 0));

        let allowed = recipient.map(|r| vec![r]).unwrap_or_default();
        self.manager
            .generate_creature_loot(target_guid, loot_id, gold_min, gold_max, allowed);
        self.manager.with_loot_mut(target_guid, |loot| {
            loot.recipient = recipient;
        });

        Ok(())
    }

    fn generate_gameobject_loot(
        &self,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        let (loot_id, gold_min, gold_max) = world
            .managers
            .gameobject_mgr
            .with_gameobject(target_guid, |gameobject| {
                world
                    .managers
                    .gameobject_mgr
                    .get_template(gameobject.entry)
                    .map(|template| {
                        (
                            template.data[1].max(0) as u32,
                            template.min_gold,
                            template.max_gold,
                        )
                    })
                    .unwrap_or((0, 0, 0))
            })
            .ok_or_else(|| anyhow::anyhow!("Gameobject not found for loot generation"))?;

        self.manager
            .generate_gameobject_loot(target_guid, loot_id, gold_min, gold_max);
        world
            .managers
            .gameobject_mgr
            .with_gameobject_mut(target_guid, |gameobject| {
                gameobject.loot_state = LootState::Activated;
            });
        // An opened chest stops being solid in its own copy right away.
        world
            .managers
            .gameobject_mgr
            .update_collision_state(target_guid, &world.managers.vmap_mgr);
        Ok(())
    }

    fn set_gameobject_state(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        state: GOState,
        world: &World,
    ) {
        if world
            .managers
            .gameobject_mgr
            .with_gameobject_mut(target_guid, |gameobject| gameobject.go_state = state)
            .is_none()
        {
            return;
        }
        world
            .managers
            .gameobject_mgr
            .update_collision_state(target_guid, &world.managers.vmap_mgr);

        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(target_guid, ObjectType::GameObject)
                .set_field(GAMEOBJECT_STATE, state as u32),
        ));
        self.broadcast_mgr.send_msg_to_player(player_guid, update);
    }

    /// Clear the corpse's lootable state and drop its loot once nothing is left.
    pub fn clear_if_empty(&self, target_guid: ObjectGuid, world: &World) {
        self.check_and_clear_if_empty(target_guid, world);
    }

    fn check_and_clear_if_empty(&self, target_guid: ObjectGuid, world: &World) {
        // Reference defers chest despawn until CMSG_LOOT_RELEASE, after the client
        // has received its final loot-item removal packet.
        if target_guid.is_game_object() {
            return;
        }

        let is_empty = self.manager.is_loot_empty(target_guid);
        let kind = self
            .manager
            .get_loot(target_guid)
            .map(|l| l.kind)
            .unwrap_or_default();

        // An item's loot is settled when its window closes (see `handle_loot_release`).
        if matches!(kind, LootKind::Item | LootKind::Disenchant) {
            return;
        }

        // Emptied pockets leave the living creature alone; the "picked" mark stays on it.
        if is_empty && kind == LootKind::Pickpocket {
            self.manager.remove_loot(target_guid);
            return;
        }
        // A skinned corpse has nothing more to give and goes on the next corpse update.
        if is_empty && kind == LootKind::Skinning {
            world
                .managers
                .creature_mgr
                .with_creature_mut(target_guid, |creature| {
                    creature.set_has_loot(false);
                    creature.corpse_decay_timer = 0;
                });
            world
                .managers
                .creature_mgr
                .clear_lootable_flag(target_guid, world);
            self.manager.remove_loot(target_guid);
            return;
        }

        if is_empty {
            if target_guid.is_game_object() {
                world
                    .managers
                    .gameobject_mgr
                    .with_gameobject_mut(target_guid, |gameobject| {
                        gameobject.loot_state = LootState::JustDeactivated;
                    });
                world
                    .managers
                    .gameobject_mgr
                    .update_collision_state(target_guid, &world.managers.vmap_mgr);
            } else {
                // Clear has_loot flag on creature. A looted corpse that cannot be skinned
                // lingers only briefly.
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(target_guid, |creature| {
                        creature.set_has_loot(false);
                        if creature.unit_flags & crate::game::common::unit_flags::SKINNABLE == 0 {
                            creature.corpse_decay_timer =
                                creature.corpse_decay_timer.min(LOOTED_CORPSE_DECAY_MS);
                        }
                    });

                // Clear lootable flag on corpse
                world
                    .managers
                    .creature_mgr
                    .clear_lootable_flag(target_guid, world);
            }

            // Remove loot data
            self.manager.remove_loot(target_guid);
        }
    }

    fn can_loot(&self, player_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) -> bool {
        use crate::game::creature::DeathState;

        if target_guid.is_game_object() {
            return world
                .managers
                .gameobject_mgr
                .with_gameobject(target_guid, |gameobject| {
                    gameobject.go_type == GameObjectType::Chest
                        && matches!(
                            gameobject.loot_state,
                            LootState::Ready | LootState::Activated
                        )
                })
                .unwrap_or(false);
        }

        // Check if target is a lootable corpse
        let result = world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                // Allow JustDied and Corpse states — checks !creature->IsAlive()
                // Dead state means the corpse was already removed (respawning), don't allow
                let is_corpse = matches!(
                    creature.death_state,
                    DeathState::JustDied | DeathState::Corpse
                );
                // Any member of the loot's allowed set can open a group corpse; the tapper's
                // wholesale loot_recipient check would lock group members out otherwise.
                let is_recipient = creature
                    .loot_recipient
                    .map(|r| r == player_guid)
                    .unwrap_or(true)
                    || self
                        .manager
                        .is_allowed_looter(target_guid, player_guid)
                        .unwrap_or(false);
                let has_loot = creature.has_loot;
                if !is_corpse {
                    tracing::info!(
                        "[LOOT] can_loot: target={:?} not lootable (death_state={:?})",
                        target_guid,
                        creature.death_state
                    );
                }
                if !is_recipient {
                    tracing::info!(
                        "[LOOT] can_loot: player={:?} is not loot recipient (recipient={:?})",
                        player_guid,
                        creature.loot_recipient
                    );
                }
                if !has_loot {
                    tracing::info!(
                        "[LOOT] can_loot: target={:?} has no loot remaining",
                        target_guid
                    );
                }

                is_corpse && is_recipient && has_loot
            });

        match result {
            Some(val) => val,
            None => {
                tracing::info!(
                    "[LOOT] can_loot: creature not found for target={:?}",
                    target_guid
                );
                false
            }
        }
    }
}
