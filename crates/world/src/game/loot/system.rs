use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::game::gameobject::{GameObjectType, LootState};
use crate::game::items::ItemManager;
use crate::game::loot::manager::LootManager;
use crate::game::player::PlayerManager;
use crate::World;
use oxcore_shared::messages::loot::*;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

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
            // Send release so the client cursor resets cleanly (mirrors vmangos SendLootRelease)
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

        // 3. Mark as being looted
        self.manager.set_looting(target_guid, player_guid);

        // 4. Set player's looting target
        self.player_mgr.set_looting_target(player_guid, target_guid);

        // 5. Build and send loot response
        tracing::info!("[LOOT] Sending loot window to player={:?}", player_guid);
        self.send_loot_window(player_guid, target_guid, world);

        Ok(())
    }

    /// Handle CMSG_LOOT_MONEY
    pub async fn handle_loot_money(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        // Take gold from loot
        let gold = self.manager.take_gold(target_guid);

        if gold > 0 {
            // Add to player
            self.player_mgr.add_money(player_guid, gold).await?;

            // Send notification
            let msg = SmsgLootMoneyNotify { gold };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);

            // Clear money display
            let msg = SmsgLootClearMoney;
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
        }

        // Check if loot is empty
        self.check_and_clear_if_empty(target_guid, world);

        Ok(())
    }

    /// Handle CMSG_AUTOSTORE_LOOT_ITEM
    pub async fn handle_loot_item(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        slot: u8,
        world: &World,
    ) -> anyhow::Result<()> {
        use crate::game::inventory::types::AddItemResult;

        // Look at the item without consuming it — vmangos only marks it looted once the
        // item is actually stored, otherwise a full bag destroys the drop.
        let loot_item = self.manager.peek_item(target_guid, slot, player_guid);

        let Some(item) = loot_item else {
            return Ok(()); // Item not found, already looted, or not visible to this player
        };

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

                // Send loot removed to player
                let msg = SmsgLootRemoved { slot };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                // Check if loot is now empty
                self.check_and_clear_if_empty(target_guid, world);
            }
            _ => {
                // Item stays in the loot so the player can retry after making room
                tracing::warn!(
                    "Failed to add loot item {} to inventory for {:?}: {:?}",
                    item.item_id,
                    player_guid,
                    result
                );
            }
        }

        Ok(())
    }

    /// Handle CMSG_LOOT_RELEASE
    pub async fn handle_loot_release(
        &self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        world: &World,
    ) -> anyhow::Result<()> {
        // Clear looting state
        self.manager.clear_looting(target_guid);

        // Clear player's looting target
        self.player_mgr.clear_looting_target(player_guid);

        // Send release response
        let msg = SmsgLootReleaseResponse {
            loot_guid: target_guid,
            unknown: 1,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        // Check if loot is empty
        self.check_and_clear_if_empty(target_guid, world);

        Ok(())
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

    fn send_loot_window(&self, player_guid: ObjectGuid, target_guid: ObjectGuid, world: &World) {
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

        let mut items = Vec::new();
        let gold = loot_ref.gold;

        // Build normal item list
        for item in &loot_ref.items {
            if item.is_looted {
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
                slot_type: 0, // Normal
            });
        }

        // Add quest items only for players who have the required quest active.
        // Quest item slots start after normal items to avoid slot collisions.
        let quest_slot_offset = loot_ref.items.len() as u8;
        for item in &loot_ref.quest_items {
            if item.is_looted || !visible_quest_slots.contains(&item.slot) {
                continue;
            }

            let display_id = self
                .item_mgr
                .get_template(item.item_id)
                .map(|t| t.display_id)
                .unwrap_or(0);

            items.push(LootResponseItem {
                slot: quest_slot_offset + item.slot,
                item_id: item.item_id,
                count: item.count,
                display_id,
                random_suffix: 0,
                random_property: 0,
                slot_type: 0,
            });
        }

        drop(loot_ref); // Release lock

        // Build and send message
        let msg = SmsgLootResponse {
            loot_guid: target_guid,
            loot_type: 1, // LOOT_CORPSE
            gold,
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
                (creature.entry, creature.level, creature.loot_recipient)
            });

        let Some((entry, level, recipient)) = creature_info else {
            return Err(anyhow::anyhow!("Creature not found for loot generation"));
        };

        let allowed = recipient.map(|r| vec![r]).unwrap_or_default();
        self.manager
            .generate_creature_loot(target_guid, entry, level, allowed);

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
        Ok(())
    }

    fn check_and_clear_if_empty(&self, target_guid: ObjectGuid, world: &World) {
        let is_empty = self.manager.is_loot_empty(target_guid);

        if is_empty {
            if target_guid.is_game_object() {
                world
                    .managers
                    .gameobject_mgr
                    .with_gameobject_mut(target_guid, |gameobject| {
                        gameobject.loot_state = LootState::JustDeactivated;
                    });
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
                // Allow JustDied and Corpse states — vmangos checks !creature->IsAlive()
                // Dead state means the corpse was already removed (respawning), don't allow
                let is_corpse = matches!(
                    creature.death_state,
                    DeathState::JustDied | DeathState::Corpse
                );
                let is_recipient = creature
                    .loot_recipient
                    .map(|r| r == player_guid)
                    .unwrap_or(true);
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
