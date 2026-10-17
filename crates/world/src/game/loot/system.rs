use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::game::common::update_fields::GAMEOBJECT_STATE;
use crate::game::gameobject::{GOState, GameObjectType, LootState};
use crate::game::items::ItemManager;
use crate::game::group::LootMethod;
use crate::game::loot::manager::LootManager;
use crate::game::loot::types::{Loot, LootItem};
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

            let sharers = self.money_sharers(player_guid, world);
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

        // Send release response
        let msg = SmsgLootReleaseResponse {
            loot_guid: target_guid,
            unknown: 1,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

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
                self.manager.with_loot_mut(target_guid, |loot| {
                    if loot.round_robin_player == Some(player_guid) {
                        loot.round_robin_player = None;
                    }
                });
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
    }

    /// Generate loot for a creature on death
    pub async fn generate_creature_loot_on_death(
        &self,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        self.generate_creature_loot(target_guid, world).await
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

        let (gold_min, gold_max) = world
            .managers
            .creature_mgr
            .get_template(entry)
            .map(|t| (t.gold_min, t.gold_max))
            .unwrap_or((0, 0));

        let allowed = recipient.map(|r| vec![r]).unwrap_or_default();
        self.manager
            .generate_creature_loot(target_guid, entry, gold_min, gold_max, allowed);
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
        let loot_id = world
            .managers
            .gameobject_mgr
            .with_gameobject(target_guid, |gameobject| {
                world
                    .managers
                    .gameobject_mgr
                    .get_template(gameobject.entry)
                    .map(|template| template.data[1].max(0) as u32)
                    .unwrap_or(0)
            })
            .ok_or_else(|| anyhow::anyhow!("Gameobject not found for loot generation"))?;

        self.manager.generate_gameobject_loot(target_guid, loot_id);
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
                // Clear has_loot flag on creature
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(target_guid, |creature| {
                        creature.set_has_loot(false);
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
