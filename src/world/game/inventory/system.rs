// Inventory System - main business logic for world
//
// This is the core inventory system that orchestrates all inventory operations.
// It uses the repository for database access, cache for fast lookups,
// and BroadcastManager for sending packets to players.

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::shared::database::characters::models::item::ItemInstanceRow;
use crate::shared::database::characters::repositories::inventory_repository_trait::{
    InventoryRepositoryTrait, InventorySlotRow,
};
use crate::shared::messages::inventory::{SmsgDestroyItem, SmsgItemPushResult};
use crate::shared::messages::inventory_update::{
    SmsgInventorySlotUpdate, SmsgInventorySlotsUpdate, SmsgVisibleItemUpdate,
};
use crate::shared::messages::login::EquipmentSlot;
use crate::shared::messages::player::SmsgPlayerMoneyUpdate;
use crate::shared::messages::update::{CreateObjectBlock, SmsgUpdateObject, UpdateBlockData};
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::shared::protocol::{HighGuid, ObjectGuid};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::inventory::inventory_types::{
    EquipmentSlot as EquipmentSlotEnum, INVENTORY_SLOT_BAG_0,
};
use crate::world::game::ItemManager;
use tracing::{info, warn};

use super::cache::{CachedItemInfo, InventoryCache, PlayerInventoryData};
use super::types::*;
use crate::world::game::items::{Bag, Item};

pub struct InventorySystem {
    repository: Arc<dyn InventoryRepositoryTrait>,
    cache: InventoryCache,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    item_mgr: Arc<ItemManager>,
}

impl InventorySystem {
    pub fn new(
        repository: Arc<dyn InventoryRepositoryTrait>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        item_mgr: Arc<ItemManager>,
    ) -> Self {
        Self {
            repository,
            cache: InventoryCache::new(),
            broadcast_mgr,
            item_mgr,
        }
    }

    #[cfg(test)]
    pub fn with_mocks(
        repository: Arc<dyn InventoryRepositoryTrait>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        cache: InventoryCache,
        item_mgr: Arc<ItemManager>,
    ) -> Self {
        Self {
            repository,
            cache,
            broadcast_mgr,
            item_mgr,
        }
    }

    fn send_inventory_error(&self, player_guid: ObjectGuid, error: u8) {
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            crate::shared::messages::SmsgInventoryChangeFailure::new(error),
        );
    }

    fn send_inventory_error_with_items(
        &self,
        player_guid: ObjectGuid,
        error: u8,
        src_item: Option<ObjectGuid>,
        dst_item: Option<ObjectGuid>,
    ) {
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            crate::shared::messages::SmsgInventoryChangeFailure::with_items(
                error, src_item, dst_item,
            ),
        );
    }

    fn send_inventory_error_with_level(&self, player_guid: ObjectGuid, required_level: u32) {
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            crate::shared::messages::SmsgInventoryChangeFailure::with_level_requirement(
                crate::shared::messages::EQUIP_ERR_CANT_EQUIP_LEVEL_I,
                required_level,
            ),
        );
    }

    fn send_buy_failed(
        &self,
        player_guid: ObjectGuid,
        vendor_guid: ObjectGuid,
        item_id: u32,
        error: crate::shared::messages::vendor::BuyError,
    ) {
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            crate::shared::messages::vendor::SmsgBuyFailed {
                vendor_guid,
                item_id,
                error,
            },
        );
    }

    pub fn send_buy_bank_slot_result(&self, player_guid: ObjectGuid, result: u8) {
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            crate::shared::messages::SmsgBuyBankSlotResult { result },
        );
    }

    fn send_slot_update(
        &self,
        player_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        item_guid: Option<ObjectGuid>,
    ) {
        let msg = SmsgInventorySlotUpdate {
            player_guid,
            bag,
            slot,
            item_guid,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    fn send_slots_update(
        &self,
        player_guid: ObjectGuid,
        src_bag: u8,
        src_slot: u8,
        src_item: Option<ObjectGuid>,
        dst_bag: u8,
        dst_slot: u8,
        dst_item: Option<ObjectGuid>,
    ) {
        let msg = SmsgInventorySlotsUpdate::swap(
            player_guid,
            src_bag,
            src_slot,
            src_item,
            dst_bag,
            dst_slot,
            dst_item,
        );
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    fn send_visible_item_update(
        &self,
        player_guid: ObjectGuid,
        slot: u8,
        item_guid: Option<ObjectGuid>,
    ) {
        let item_entry = item_guid
            .map(|guid| self.cache.get_item_entry(player_guid, guid))
            .unwrap_or(0);

        tracing::debug!(
            "[VISIBLE_ITEM_UPDATE] Player={:?} slot={} item_guid={:?} item_entry={}",
            player_guid,
            slot,
            item_guid,
            item_entry
        );

        let msg = SmsgVisibleItemUpdate::new(player_guid, slot, item_entry);
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    fn send_item_update(&self, player_guid: ObjectGuid, item_guid: ObjectGuid) {
        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => {
                tracing::warn!(
                    "[INVENTORY] Item {:?} not found in cache for update",
                    item_guid
                );
                return;
            }
        };

        let item_read = item.read();
        let update_block = item_read.to_create_block();

        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject2(update_block));

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    pub fn cache(&self) -> &InventoryCache {
        &self.cache
    }

    pub fn is_player_loaded(&self, player_guid: ObjectGuid) -> bool {
        self.cache.has_player_inventory(player_guid)
    }

    pub async fn load_player_inventory(&self, player_guid: ObjectGuid) -> Result<()> {
        let player_guid_low = player_guid.low();

        let slots = self
            .repository
            .load_player_inventory(player_guid_low)
            .await
            .context("Failed to load inventory slots")?;

        let items = self
            .repository
            .find_items_by_owner(player_guid_low)
            .await
            .context("Failed to load item instances")?;

        let money = self
            .repository
            .get_player_money(player_guid_low)
            .await
            .context("Failed to load player money")?;

        let mut data = PlayerInventoryData::new(player_guid);
        data.money = money;

        let item_map: std::collections::HashMap<u32, &ItemInstanceRow> =
            items.iter().map(|i| (i.guid, i)).collect();

        let mut bag_guids: std::collections::HashMap<u8, ObjectGuid> =
            std::collections::HashMap::new();

        for slot in &slots {
            let item_guid_low = slot.item_guid;
            let item_row = match item_map.get(&item_guid_low) {
                Some(row) => row,
                None => continue,
            };

            let normalized_bag = if slot.bag == 0 {
                INVENTORY_SLOT_BAG_0
            } else {
                slot.bag
            };

            let item_guid = ObjectGuid::new_without_entry(HighGuid::Item, item_guid_low);

            if normalized_bag == INVENTORY_SLOT_BAG_0 && slot.slot >= 19 && slot.slot < 23 {
                let bag_slot = slot.slot - 19;
                let bag_guid = ObjectGuid::new_without_entry(HighGuid::Item, item_guid_low);

                if let Some(template) = self.item_mgr.get_template(item_row.item_id) {
                    let bag_size = template.container_slots.max(16).min(36) as u8;
                    let bag = Bag::new(bag_guid, item_row.item_id);
                    data.bags.insert(bag_guid, bag);
                    data.bag_guids[bag_slot as usize] = Some(bag_guid);
                    bag_guids.insert(bag_slot, bag_guid);
                }
            }
        }

        for slot in &slots {
            let item_guid_low = slot.item_guid;
            let item_row = match item_map.get(&item_guid_low) {
                Some(row) => row,
                None => continue,
            };

            let normalized_bag = if slot.bag == 0 {
                INVENTORY_SLOT_BAG_0
            } else {
                slot.bag
            };

            let item_guid = ObjectGuid::new_without_entry(HighGuid::Item, item_guid_low);

            if normalized_bag != INVENTORY_SLOT_BAG_0 {
                if let Some(bag_guid) = bag_guids.get(&normalized_bag) {
                    if let Some(mut bag_ref) = data.bags.get_mut(bag_guid) {
                        bag_ref.set_slot(slot.slot, Some(item_guid));
                    }
                }
            }

            data.set_item_at(normalized_bag, slot.slot, Some(item_guid));

            let enchantments = Self::parse_enchantments(&item_row.enchantments);
            let creator_guid = if item_row.creator_guid != 0 {
                Some(ObjectGuid::new_without_entry(
                    HighGuid::Player,
                    item_row.creator_guid,
                ))
            } else {
                None
            };
            let gift_creator_guid = if item_row.gift_creator_guid != 0 {
                Some(ObjectGuid::new_without_entry(
                    HighGuid::Player,
                    item_row.gift_creator_guid,
                ))
            } else {
                None
            };

            let max_durability = match self.item_mgr.get_template(item_row.item_id) {
                Some(t) => t.max_durability,
                None => {
                    warn!(
                        "[INVENTORY] Template NOT found for item_id={}!",
                        item_row.item_id
                    );
                    0
                }
            };

            let item = Item::from_db_row(
                item_guid,
                item_row.item_id,
                item_row.count,
                player_guid,
                slot.slot,
                normalized_bag,
                item_row.flags,
                item_row.durability as u32,
                max_durability,
                enchantments,
                item_row.random_property_id as i32,
                creator_guid,
                gift_creator_guid,
                item_row.duration as u32,
                Self::parse_spell_charges(item_row.charges.as_deref()),
            );

            data.items.insert(item_guid, Arc::new(RwLock::new(item)));
        }

        let item_count = data.items.len();
        self.cache.add_player_inventory(data);

        Ok(())
    }

    fn parse_enchantments(enchantments_str: &str) -> Vec<(u32, u32, u32)> {
        let values: Vec<u32> = enchantments_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut enchantments = Vec::new();
        for i in (0..values.len()).step_by(3) {
            if i + 2 < values.len() {
                enchantments.push((values[i], values[i + 1], values[i + 2]));
            }
        }
        enchantments
    }

    fn parse_spell_charges(charges_str: Option<&str>) -> [i32; 5] {
        let mut charges = [0i32; 5];
        if let Some(s) = charges_str {
            let values: Vec<i32> = s
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            for (i, value) in values.iter().enumerate().take(5) {
                charges[i] = *value;
            }
        }
        charges
    }

    fn format_enchantments(enchantments: &[(u32, u32, u32)]) -> String {
        enchantments
            .iter()
            .filter(|(id, _, _)| *id != 0)
            .map(|(id, duration, charges)| format!("{} {} {}", id, duration, charges))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn format_spell_charges(charges: &[i32; 5]) -> String {
        charges
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn unload_player_inventory(&self, player_guid: ObjectGuid) {
        self.cache.remove_player_inventory(player_guid);
    }

    pub fn get_money(&self, player_guid: ObjectGuid) -> Option<u32> {
        self.cache.get_money(player_guid)
    }

    pub async fn add_item(
        &self,
        player_guid: ObjectGuid,
        item_id: u32,
        count: u32,
    ) -> AddItemResult {
        if !self.cache.has_player_inventory(player_guid) {
            return AddItemResult::PlayerNotLoaded;
        }

        // Get item template for stack info
        let proto = match self.item_mgr.get_template(item_id) {
            Some(t) => t,
            None => return AddItemResult::InvalidItem,
        };

        let max_stack = proto.stackable;
        let mut remaining_count = count;
        let mut items_modified = Vec::new();
        let mut items_created = Vec::new();

        // Step 1: Try to fill existing stacks
        if max_stack > 1 {
            let stackable_slots = match self
                .repository
                .find_stackable_slots(player_guid.low(), item_id, max_stack)
                .await
            {
                Ok(slots) => slots,
                Err(e) => return AddItemResult::DatabaseError(e.to_string()),
            };

            for slot_info in stackable_slots {
                if remaining_count == 0 {
                    break;
                }

                let available_space = max_stack - slot_info.current_count;
                let add_count = remaining_count.min(available_space);
                let new_count = slot_info.current_count + add_count;

                // Update database
                if let Err(e) = self
                    .repository
                    .update_item_count(slot_info.item_guid, new_count)
                    .await
                {
                    return AddItemResult::DatabaseError(e.to_string());
                }

                // Update cache
                let item_obj_guid =
                    ObjectGuid::new_without_entry(HighGuid::Item, slot_info.item_guid);
                self.cache
                    .update_item_count(player_guid, item_obj_guid, new_count);

                // Send item update to client so stack count is visible
                self.send_item_update(player_guid, item_obj_guid);

                items_modified.push((item_obj_guid, new_count));
                remaining_count -= add_count;
            }
        }

        // Step 2: Create new stacks for remaining items
        while remaining_count > 0 {
            // Find free slot
            let (bag, slot) = match self.cache.find_free_inventory_slot(player_guid) {
                Some(pos) => pos,
                None => return AddItemResult::InventoryFull,
            };

            let stack_count = remaining_count.min(max_stack);

            // Get next item GUID
            let item_guid_raw = match self.repository.get_next_item_guid().await {
                Ok(g) => g,
                Err(e) => return AddItemResult::DatabaseError(e.to_string()),
            };

            let item_obj_guid = ObjectGuid::new_without_entry(HighGuid::Item, item_guid_raw);

            // Create item row
            let item_row = ItemInstanceRow {
                guid: item_guid_raw,
                item_id,
                owner_guid: player_guid.low(),
                creator_guid: 0,
                gift_creator_guid: 0,
                count: stack_count,
                duration: 0,
                charges: None,
                flags: 0,
                enchantments: String::new(),
                random_property_id: 0,
                durability: 0,
                text: 0,
                generated_loot: None,
            };

            let slot_row = InventorySlotRow {
                guid: player_guid.low(),
                bag,
                slot,
                item_guid: item_guid_raw,
            };

            // Save to database
            if let Err(e) = self.repository.create_item(&item_row, &slot_row).await {
                return AddItemResult::DatabaseError(e.to_string());
            }

            // Update cache
            self.cache
                .set_item_at(player_guid, bag, slot, Some(item_obj_guid));

            let creator_guid = if item_row.creator_guid != 0 {
                Some(ObjectGuid::new_without_entry(
                    HighGuid::Player,
                    item_row.creator_guid,
                ))
            } else {
                None
            };
            let gift_creator_guid = if item_row.gift_creator_guid != 0 {
                Some(ObjectGuid::new_without_entry(
                    HighGuid::Player,
                    item_row.gift_creator_guid,
                ))
            } else {
                None
            };
            let enchantments = Self::parse_enchantments(&item_row.enchantments);

            let max_durability = self
                .item_mgr
                .get_template(item_id)
                .map(|t| t.max_durability)
                .unwrap_or(0);

            let new_item = Item::from_db_row(
                item_obj_guid,
                item_id,
                stack_count,
                player_guid,
                slot,
                bag,
                item_row.flags,
                item_row.durability as u32,
                max_durability,
                enchantments,
                item_row.random_property_id as i32,
                creator_guid,
                gift_creator_guid,
                item_row.duration as u32,
                Self::parse_spell_charges(item_row.charges.as_deref()),
            );

            self.cache
                .add_item(player_guid, Arc::new(RwLock::new(new_item)));

            // Send CREATE_OBJECT2 so the client knows about this item object
            self.send_item_update(player_guid, item_obj_guid);
            // Update client inventory slot so item appears in bag
            self.send_slot_update(player_guid, bag, slot, Some(item_obj_guid));

            items_created.push(item_obj_guid);
            remaining_count -= stack_count;

            let push_result = SmsgItemPushResult {
                player_guid,
                received: 0,
                created: 1,
                show_in_chat: 1,
                bagslot: bag,
                item_entry: item_id,
                suffix_factor: 0,
                random_property_id: 0,
                count: stack_count,
            };
            self.broadcast_mgr
                .send_msg_to_player(player_guid, push_result);
        }

        AddItemResult::Success {
            items_modified,
            items_created,
        }
    }

    pub fn remove_item(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        count: u32,
    ) -> RemoveItemResult {
        if !self.cache.has_player_inventory(player_guid) {
            return RemoveItemResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => {
                self.send_inventory_error(player_guid, crate::shared::messages::ERR_ITEM_NOT_FOUND);
                return RemoveItemResult::ItemNotFound;
            }
        };

        let item_read = item.read();
        let item_count = item_read.count;
        let item_bag = item_read.bag;
        let item_slot = item_read.slot;
        drop(item_read);

        if count > item_count {
            self.send_inventory_error(player_guid, crate::shared::messages::ERR_ITEM_NOT_FOUND);
            return RemoveItemResult::InsufficientCount;
        }

        let new_count = item_count - count;

        if new_count == 0 {
            // Queue DB op (deferred)
            self.cache.push_pending_op(
                player_guid,
                super::cache::PendingInventoryOp::DeleteItem {
                    item_guid: item_guid.low(),
                },
            );

            self.cache
                .set_item_at(player_guid, item_bag, item_slot, None);
            self.cache.remove_item(player_guid, item_guid);

            self.send_slot_update(player_guid, item_bag, item_slot, None);

            // Destroy item object on client
            let destroy_msg = SmsgDestroyItem {
                item_guid,
                count: 0,
            };
            self.broadcast_mgr
                .send_msg_to_player(player_guid, destroy_msg);

            if self.cache.is_equipment_slot(player_guid, item_slot) {
                self.send_visible_item_update(player_guid, item_slot, None);
            }

            RemoveItemResult::ItemRemoved { item_guid }
        } else {
            // Reduce count - queue DB op (deferred)
            self.cache.push_pending_op(
                player_guid,
                super::cache::PendingInventoryOp::UpdateCount {
                    item_guid: item_guid.low(),
                    count: new_count,
                },
            );

            // Update cache
            self.cache
                .update_item_count(player_guid, item_guid, new_count);

            RemoveItemResult::CountReduced {
                item_guid,
                new_count,
            }
        }
    }

    pub fn move_item(
        &self,
        player_guid: ObjectGuid,
        src_bag: u8,
        src_slot: u8,
        dst_bag: u8,
        dst_slot: u8,
    ) -> MoveItemResult {
        tracing::debug!(
            "[INVENTORY] move_item: player={:?} from bag={} slot={} to bag={} slot={}",
            player_guid,
            src_bag,
            src_slot,
            dst_bag,
            dst_slot
        );

        // === VALIDATION PHASE ===

        if !self.cache.has_player_inventory(player_guid) {
            tracing::error!(
                "[INVENTORY] move_item FAILED: player not loaded {:?}",
                player_guid
            );
            return MoveItemResult::PlayerNotLoaded;
        }

        if src_bag == dst_bag && src_slot == dst_slot {
            return MoveItemResult::Moved;
        }

        let src_item_guid = match self.cache.get_item_at(player_guid, src_bag, src_slot) {
            Some(g) => g,
            None => {
                tracing::warn!(
                    "[INVENTORY] move_item FAILED: item not found at src bag={} slot={}",
                    src_bag,
                    src_slot
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                );
                return MoveItemResult::InvalidSource;
            }
        };

        let dst_item_guid = self.cache.get_item_at(player_guid, dst_bag, dst_slot);

        // === EXECUTION PHASE ===

        match dst_item_guid {
            Some(dst_guid) => {
                // Destination has an item - check for stacking or swap
                if let Some(src_item_arc) = self.cache.get_item(player_guid, src_item_guid) {
                    if let Some(dst_item_arc) = self.cache.get_item(player_guid, dst_guid) {
                        let src_item = src_item_arc.read();
                        let dst_item = dst_item_arc.read();

                        // If same item type and stackable, merge instead of swap
                        if src_item.entry == dst_item.entry {
                            if let Some(template) = self.item_mgr.get_template(src_item.entry) {
                                if template.container_slots == 0 {
                                    let max_stack = template.stackable;

                                    if max_stack > 1 {
                                        let src_count = src_item.count;
                                        let dst_count = dst_item.count;
                                        let available_space = max_stack.saturating_sub(dst_count);

                                        // Release read locks before proceeding
                                        drop(src_item);
                                        drop(dst_item);

                                        if available_space == 0 {
                                            self.send_inventory_error_with_items(
                                                player_guid,
                                                crate::shared::messages::EQUIP_ERR_ITEM_CANT_STACK,
                                                Some(src_item_guid),
                                                Some(dst_guid),
                                            );
                                            return MoveItemResult::InvalidDestination;
                                        }

                                        let items_to_move = src_count.min(available_space);
                                        let new_src_count = src_count - items_to_move;
                                        let new_dst_count = dst_count + items_to_move;

                                        if new_src_count == 0 {
                                            // Moving all items - delete source, update destination
                                            // Queue DB ops (deferred)
                                            self.cache.push_pending_op(
                                                player_guid,
                                                super::cache::PendingInventoryOp::DeleteItem {
                                                    item_guid: src_item_guid.low(),
                                                },
                                            );
                                            self.cache.push_pending_op(
                                                player_guid,
                                                super::cache::PendingInventoryOp::UpdateCount {
                                                    item_guid: dst_guid.low(),
                                                    count: new_dst_count,
                                                },
                                            );

                                            // Update cache
                                            self.cache.remove_item(player_guid, src_item_guid);
                                            self.cache.set_item_at(
                                                player_guid,
                                                src_bag,
                                                src_slot,
                                                None,
                                            );
                                            self.cache.update_item_count(
                                                player_guid,
                                                dst_guid,
                                                new_dst_count,
                                            );

                                            // Send updates
                                            self.send_slot_update(
                                                player_guid,
                                                src_bag,
                                                src_slot,
                                                None,
                                            );
                                            self.send_item_update(player_guid, dst_guid);

                                            return MoveItemResult::Merged {
                                                source_removed: true,
                                            };
                                        } else {
                                            // Moving partial stack - update both counts
                                            self.cache.push_pending_op(
                                                player_guid,
                                                super::cache::PendingInventoryOp::UpdateCount {
                                                    item_guid: src_item_guid.low(),
                                                    count: new_src_count,
                                                },
                                            );
                                            self.cache.push_pending_op(
                                                player_guid,
                                                super::cache::PendingInventoryOp::UpdateCount {
                                                    item_guid: dst_guid.low(),
                                                    count: new_dst_count,
                                                },
                                            );

                                            // Update cache
                                            self.cache.update_item_count(
                                                player_guid,
                                                src_item_guid,
                                                new_src_count,
                                            );
                                            self.cache.update_item_count(
                                                player_guid,
                                                dst_guid,
                                                new_dst_count,
                                            );

                                            // Send updates
                                            self.send_item_update(player_guid, src_item_guid);
                                            self.send_item_update(player_guid, dst_guid);

                                            return MoveItemResult::Merged {
                                                source_removed: false,
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // No stacking possible, proceed with swap logic
                // Queue DB op (deferred)
                self.cache.push_pending_op(
                    player_guid,
                    super::cache::PendingInventoryOp::SwapItems {
                        player_guid: player_guid.low(),
                        item1_guid: src_item_guid.low(),
                        bag1: dst_bag,
                        slot1: dst_slot,
                        item2_guid: Some(dst_guid.low()),
                        bag2: src_bag,
                        slot2: src_slot,
                    },
                );

                // Update cache
                self.cache
                    .set_item_at(player_guid, src_bag, src_slot, Some(dst_guid));
                self.cache
                    .set_item_at(player_guid, dst_bag, dst_slot, Some(src_item_guid));

                // Send updates to client
                self.send_slots_update(
                    player_guid,
                    src_bag,
                    src_slot,
                    Some(dst_guid),
                    dst_bag,
                    dst_slot,
                    Some(src_item_guid),
                );

                if self.cache.is_equipment_slot(player_guid, src_slot) {
                    self.send_visible_item_update(player_guid, src_slot, Some(dst_guid));
                }
                if self.cache.is_equipment_slot(player_guid, dst_slot) {
                    self.send_visible_item_update(player_guid, dst_slot, Some(src_item_guid));
                }

                MoveItemResult::Swapped
            }
            None => {
                // Destination is empty - simple move
                // Queue DB op (deferred)
                self.cache.push_pending_op(
                    player_guid,
                    super::cache::PendingInventoryOp::MoveItem {
                        player_guid: player_guid.low(),
                        item_guid: src_item_guid.low(),
                        bag: dst_bag,
                        slot: dst_slot,
                    },
                );

                // Update cache
                self.cache.set_item_at(player_guid, src_bag, src_slot, None);
                self.cache
                    .set_item_at(player_guid, dst_bag, dst_slot, Some(src_item_guid));

                // Send updates to client
                self.send_slot_update(player_guid, src_bag, src_slot, None);
                self.send_slot_update(player_guid, dst_bag, dst_slot, Some(src_item_guid));

                if self.cache.is_equipment_slot(player_guid, src_slot) {
                    self.send_visible_item_update(player_guid, src_slot, None);
                }
                if self.cache.is_equipment_slot(player_guid, dst_slot) {
                    self.send_visible_item_update(player_guid, dst_slot, Some(src_item_guid));
                }

                MoveItemResult::Moved
            }
        }
    }

    pub fn add_gold(&self, player_guid: ObjectGuid, amount: u32) -> GoldResult {
        tracing::debug!(
            "[INVENTORY] add_gold: player={:?} amount={}",
            player_guid,
            amount
        );

        if !self.cache.has_player_inventory(player_guid) {
            tracing::error!(
                "[INVENTORY] add_gold FAILED: player not loaded {:?}",
                player_guid
            );
            return GoldResult::PlayerNotLoaded;
        }

        let current = match self.cache.get_money(player_guid) {
            Some(m) => m,
            None => {
                tracing::error!(
                    "[INVENTORY] add_gold FAILED: no money in cache for {:?}",
                    player_guid
                );
                return GoldResult::PlayerNotLoaded;
            }
        };

        if current > MAX_MONEY - amount {
            tracing::error!(
                "[INVENTORY] add_gold FAILED: cap exceeded for {:?}",
                player_guid
            );
            return GoldResult::CapExceeded;
        }

        let new_balance = current + amount;

        // Queue DB op (deferred)
        self.cache.push_pending_op(
            player_guid,
            super::cache::PendingInventoryOp::UpdateMoney {
                player_guid: player_guid.low(),
                amount: new_balance,
            },
        );

        self.cache.set_money(player_guid, new_balance);
        self.send_money_update(player_guid, new_balance);

        GoldResult::Success { new_balance }
    }

    pub fn remove_gold(&self, player_guid: ObjectGuid, amount: u32) -> GoldResult {
        tracing::debug!(
            "[INVENTORY] remove_gold: player={:?} amount={}",
            player_guid,
            amount
        );

        if !self.cache.has_player_inventory(player_guid) {
            tracing::error!(
                "[INVENTORY] remove_gold FAILED: player not loaded {:?}",
                player_guid
            );
            return GoldResult::PlayerNotLoaded;
        }

        let current = match self.cache.get_money(player_guid) {
            Some(m) => m,
            None => {
                tracing::error!(
                    "[INVENTORY] remove_gold FAILED: no money in cache for {:?}",
                    player_guid
                );
                return GoldResult::PlayerNotLoaded;
            }
        };

        if current < amount {
            tracing::error!(
                "[INVENTORY] remove_gold FAILED: insufficient funds for {:?} (has {} needs {})",
                player_guid,
                current,
                amount
            );
            return GoldResult::InsufficientFunds;
        }

        let new_balance = current - amount;

        // Queue DB op (deferred)
        self.cache.push_pending_op(
            player_guid,
            super::cache::PendingInventoryOp::UpdateMoney {
                player_guid: player_guid.low(),
                amount: new_balance,
            },
        );

        self.cache.set_money(player_guid, new_balance);
        self.send_money_update(player_guid, new_balance);

        GoldResult::Success { new_balance }
    }

    pub fn get_item_at(&self, player_guid: ObjectGuid, bag: u8, slot: u8) -> Option<ObjectGuid> {
        self.cache.get_item_at(player_guid, bag, slot)
    }

    pub fn count_items(&self, player_guid: ObjectGuid, entry_id: u32) -> u32 {
        self.cache.count_items_by_entry(player_guid, entry_id)
    }

    pub fn has_free_slots(&self, player_guid: ObjectGuid, count: u32) -> bool {
        self.cache.count_free_inventory_slots(player_guid) >= count
    }

    /// Transfer an item from one player to another, preserving all properties
    ///
    /// This is used for trading, mailing, etc. The item is removed from the source player
    /// and added to the target player with the SAME GUID (MaNGOS behavior).
    /// Keeping the same GUID avoids client cache conflicts and simplifies the transaction.
    pub async fn transfer_item(
        &self,
        from_player: ObjectGuid,
        to_player: ObjectGuid,
        item_guid: ObjectGuid,
    ) -> TransferItemResult {
        // 1. Get item from source player's cache
        let item_arc = match self.cache.get_item(from_player, item_guid) {
            Some(item) => item,
            None => {
                tracing::warn!(
                    "[TRANSFER] Item {:?} not found in source player {:?} cache",
                    item_guid,
                    from_player
                );
                return TransferItemResult::ItemNotFound;
            }
        };

        // Get item properties while holding the read lock
        let (item_entry, item_count, item_data) = {
            let item = item_arc.read();
            let data = ItemTransferData {
                entry: item.entry,
                count: item.count,
                durability: item.durability,
                max_durability: item.max_durability,
                enchantments: item.enchantments.clone(),
                random_property_id: item.random_property_id,
                creator_guid: item.creator_guid,
                gift_creator_guid: item.gift_creator_guid,
                duration: item.duration,
                spell_charges: item.spell_charges,
                flags: item.flags,
            };
            (item.entry, item.count, data)
        };

        // 2. Find free slot in target inventory
        let (target_bag, target_slot) = match self.cache.find_free_inventory_slot(to_player) {
            Some(slot) => slot,
            None => {
                tracing::warn!(
                    "[TRANSFER] No free inventory slots for target player {:?}",
                    to_player
                );
                return TransferItemResult::TargetInventoryFull;
            }
        };

        // 3. Get source item's current bag/slot before removal
        let (src_bag, src_slot) = {
            let item = item_arc.read();
            (item.bag, item.slot)
        };

        // 4. Update database: Change owner and slot
        // MaNGOS-style: Keep the same GUID, just update owner and position
        if let Err(e) = self
            .repository
            .update_item_owner(item_guid.low(), to_player.low())
            .await
        {
            tracing::error!("[TRANSFER] Failed to update item owner: {}", e);
            return TransferItemResult::DatabaseError(e.to_string());
        }

        if let Err(e) = self
            .repository
            .move_item(to_player.low(), item_guid.low(), target_bag, target_slot)
            .await
        {
            tracing::error!("[TRANSFER] Failed to move item to target slot: {}", e);
            // Attempt rollback
            let _ = self
                .repository
                .update_item_owner(item_guid.low(), from_player.low());
            return TransferItemResult::DatabaseError(e.to_string());
        }

        // 5. Send slot update to source player FIRST (before removing from cache)
        // This ensures the client receives the packet while the item still exists in cache
        // which helps the client properly clear the slot
        self.send_slot_update(from_player, src_bag, src_slot, None);

        // 6. Remove from source player's cache AFTER sending the slot update
        self.cache.set_item_at(from_player, src_bag, src_slot, None);
        self.cache.remove_item(from_player, item_guid);

        // 7. Add to target player's cache with updated owner and position
        {
            let mut item = item_arc.write();
            item.owner_guid = to_player;
            item.bag = target_bag;
            item.slot = target_slot;
        }

        self.cache.add_item(to_player, item_arc.clone());
        self.cache
            .set_item_at(to_player, target_bag, target_slot, Some(item_guid));

        // 8. Send CREATE_OBJECT2 packet to target player (they may have seen this GUID before
        // if the trade was cancelled and retried, so use CreateObject2 for safety)
        let create_block = {
            let item = item_arc.read();
            item.to_create_block()
        };
        let update_packet = SmsgUpdateObject {
            has_transport: false,
            blocks: vec![
                crate::shared::messages::update::UpdateBlockData::CreateObject2(create_block),
            ],
        };

        self.broadcast_mgr
            .send_msg_to_player(to_player, update_packet);

        // 9. Send inventory slot update packet to target
        let slot_update = SmsgInventorySlotUpdate {
            player_guid: to_player,
            bag: target_bag,
            slot: target_slot,
            item_guid: Some(item_guid),
        };

        self.broadcast_mgr
            .send_msg_to_player(to_player, slot_update);

        tracing::debug!(
            "[TRANSFER] Successfully transferred item {:?} from {:?} to {:?} (bag={}, slot={}, same GUID)",
            item_guid,
            from_player,
            to_player,
            target_bag,
            target_slot
        );

        TransferItemResult::Success {
            new_item_guid: item_guid,
        }
    }

    pub fn find_items_by_entry(&self, player_guid: ObjectGuid, entry_id: u32) -> Vec<ObjectGuid> {
        self.cache.find_items_by_entry(player_guid, entry_id)
    }

    pub async fn split_item(
        &self,
        player_guid: ObjectGuid,
        src_bag: u8,
        src_slot: u8,
        dst_bag: u8,
        dst_slot: u8,
        count: u32,
    ) -> SplitItemResult {
        tracing::debug!(
            "[INVENTORY] split_item: player={:?} from bag={} slot={} to bag={} slot={} count={}",
            player_guid,
            src_bag,
            src_slot,
            dst_bag,
            dst_slot,
            count
        );

        // === VALIDATION PHASE - Check all preconditions before making any changes ===

        if !self.cache.has_player_inventory(player_guid) {
            tracing::error!(
                "[INVENTORY] split_item FAILED: player not loaded {:?}",
                player_guid
            );
            return SplitItemResult::PlayerNotLoaded;
        }

        // Check if source and destination are the same
        if src_bag == dst_bag && src_slot == dst_slot {
            tracing::warn!("[INVENTORY] split_item FAILED: source and destination are the same");
            return SplitItemResult::InvalidCount;
        }

        // Check for zero count (invalid)
        if count == 0 {
            tracing::warn!("[INVENTORY] split_item FAILED: count is zero");
            self.send_inventory_error(
                player_guid,
                crate::shared::messages::EQUIP_ERR_COULDNT_SPLIT_ITEMS,
            );
            return SplitItemResult::InvalidCount;
        }

        let src_item_guid = match self.cache.get_item_at(player_guid, src_bag, src_slot) {
            Some(g) => g,
            None => {
                tracing::warn!(
                    "[INVENTORY] split_item FAILED: item not found at src bag={} slot={}",
                    src_bag,
                    src_slot
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                );
                return SplitItemResult::SourceNotFound;
            }
        };

        let src_item = match self.cache.get_item(player_guid, src_item_guid) {
            Some(i) => i,
            None => {
                tracing::warn!(
                    "[INVENTORY] split_item FAILED: item object not found for {:?}",
                    src_item_guid
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                );
                return SplitItemResult::SourceNotFound;
            }
        };

        let src_info = {
            let item = src_item.read();
            CachedItemInfo {
                guid: item.guid,
                entry_id: item.entry,
                count: item.count,
                bag: item.bag,
                slot: item.slot,
            }
        };

        // Validate count - cannot split all items (must leave at least 1)
        if count >= src_info.count {
            tracing::warn!(
                "[INVENTORY] split_item FAILED: count {} >= src_count {}",
                count,
                src_info.count
            );
            self.send_inventory_error_with_items(
                player_guid,
                crate::shared::messages::EQUIP_ERR_TRIED_TO_SPLIT_MORE_THAN_COUNT,
                Some(src_item_guid),
                None,
            );
            return SplitItemResult::InvalidCount;
        }

        // Get item template for stack size validation
        let max_stack = match self.item_mgr.get_template(src_info.entry_id) {
            Some(t) => t.stackable,
            None => {
                tracing::error!(
                    "[INVENTORY] split_item FAILED: template not found for item {}",
                    src_info.entry_id
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                );
                return SplitItemResult::InvalidCount;
            }
        };

        let dst_item_guid = self.cache.get_item_at(player_guid, dst_bag, dst_slot);

        // === EXECUTION PHASE - All validations passed, now execute ===

        if let Some(dst_guid) = dst_item_guid {
            // Destination has an item - check if we can merge
            let dst_item = match self.cache.get_item(player_guid, dst_guid) {
                Some(i) => i,
                None => {
                    tracing::error!(
                        "[INVENTORY] split_item FAILED: destination item {:?} not found in cache",
                        dst_guid
                    );
                    self.send_inventory_error_with_items(
                        player_guid,
                        crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                        Some(src_item_guid),
                        Some(dst_guid),
                    );
                    return SplitItemResult::DestinationOccupied;
                }
            };

            let dst_info = {
                let item = dst_item.read();
                CachedItemInfo {
                    guid: item.guid,
                    entry_id: item.entry,
                    count: item.count,
                    bag: item.bag,
                    slot: item.slot,
                }
            };

            // Check if items are the same type
            if dst_info.entry_id != src_info.entry_id {
                tracing::warn!(
                    "[INVENTORY] split_item FAILED: different item types - src={}, dst={}",
                    src_info.entry_id,
                    dst_info.entry_id
                );
                self.send_inventory_error_with_items(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_CANT_STACK,
                    Some(src_item_guid),
                    Some(dst_guid),
                );
                return SplitItemResult::DestinationOccupied;
            }

            // Check if destination has space for the split items
            if dst_info.count + count > max_stack {
                tracing::warn!("[INVENTORY] split_item FAILED: would exceed max stack - dst_count={}, adding={}, max={}", 
                    dst_info.count, count, max_stack);
                self.send_inventory_error_with_items(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_CANT_STACK,
                    Some(src_item_guid),
                    Some(dst_guid),
                );
                return SplitItemResult::DestinationOccupied;
            }

            let new_src_count = src_info.count - count;
            let new_dst_count = dst_info.count + count;

            tracing::debug!(
                "[INVENTORY] split_item MERGE: src {:?} {}→{}, dst {:?} {}→{}",
                src_item_guid,
                src_info.count,
                new_src_count,
                dst_guid,
                dst_info.count,
                new_dst_count
            );

            // Update database first
            if let Err(e) = self
                .repository
                .batch_update_counts(&[
                    (src_item_guid.low(), new_src_count),
                    (dst_guid.low(), new_dst_count),
                ])
                .await
            {
                tracing::error!("[INVENTORY] split_item MERGE FAILED: database error: {}", e);
                self.send_inventory_error_with_items(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                    Some(src_item_guid),
                    Some(dst_guid),
                );
                return SplitItemResult::DatabaseError(e.to_string());
            }

            // Update cache after successful database operation
            tracing::debug!("[INVENTORY] split_item MERGE: updating cache");
            self.cache
                .update_item_count(player_guid, src_item_guid, new_src_count);
            self.cache
                .update_item_count(player_guid, dst_guid, new_dst_count);

            // Send client updates for both items to reflect new counts
            tracing::debug!("[INVENTORY] split_item MERGE: sending item update packets");
            self.send_item_update(player_guid, src_item_guid);
            self.send_item_update(player_guid, dst_guid);

            tracing::debug!("[INVENTORY] split_item MERGE SUCCESS");
            SplitItemResult::MergedToExisting {
                source_guid: src_item_guid,
                dest_guid: dst_guid,
            }
        } else {
            // Destination is empty - create new item
            let new_src_count = src_info.count - count;

            // Get next item GUID for the new item
            let new_item_guid_raw = match self.repository.get_next_item_guid().await {
                Ok(g) => g,
                Err(e) => {
                    tracing::error!(
                        "[INVENTORY] split_item FAILED: could not get next item GUID: {}",
                        e
                    );
                    self.send_inventory_error(
                        player_guid,
                        crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                    );
                    return SplitItemResult::DatabaseError(e.to_string());
                }
            };

            let new_item_guid = ObjectGuid::new_without_entry(HighGuid::Item, new_item_guid_raw);

            let item_row = ItemInstanceRow {
                guid: new_item_guid_raw,
                item_id: src_info.entry_id,
                owner_guid: player_guid.low(),
                creator_guid: 0,
                gift_creator_guid: 0,
                count,
                duration: 0,
                charges: None,
                flags: 0,
                enchantments: String::new(),
                random_property_id: 0,
                durability: 0,
                text: 0,
                generated_loot: None,
            };

            let slot_row = InventorySlotRow {
                guid: player_guid.low(),
                bag: dst_bag,
                slot: dst_slot,
                item_guid: new_item_guid_raw,
            };

            // Update source item count first
            if let Err(e) = self
                .repository
                .update_item_count(src_item_guid.low(), new_src_count)
                .await
            {
                tracing::error!(
                    "[INVENTORY] split_item FAILED: could not update source count: {}",
                    e
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                );
                return SplitItemResult::DatabaseError(e.to_string());
            }

            // Create new item
            if let Err(e) = self.repository.create_item(&item_row, &slot_row).await {
                tracing::error!(
                    "[INVENTORY] split_item FAILED: could not create new item: {}",
                    e
                );
                // Attempt rollback - restore source count
                let _ = self
                    .repository
                    .update_item_count(src_item_guid.low(), src_info.count);
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                );
                return SplitItemResult::DatabaseError(e.to_string());
            }

            // Update cache after successful database operations
            self.cache
                .update_item_count(player_guid, src_item_guid, new_src_count);
            self.cache
                .set_item_at(player_guid, dst_bag, dst_slot, Some(new_item_guid));

            // Create and cache the new item object
            let new_item = Item::from_db_row(
                new_item_guid,
                src_info.entry_id,
                count,
                player_guid,
                dst_slot,
                dst_bag,
                0,
                0,
                0,
                Vec::new(),
                0,
                None,
                None,
                0,
                [0, 0, 0, 0, 0],
            );

            self.cache
                .add_item(player_guid, Arc::new(RwLock::new(new_item)));

            // Send item updates to client to reflect new counts
            // This is critical - the client needs to see the updated source item count
            // and the new item with its count
            self.send_item_update(player_guid, src_item_guid);
            self.send_item_update(player_guid, new_item_guid);

            // Also send slot update for the destination to ensure proper slot assignment
            self.send_slot_update(player_guid, dst_bag, dst_slot, Some(new_item_guid));

            tracing::debug!(
                "[INVENTORY] split_item SUCCESS: created new item {:?} with count {}",
                new_item_guid,
                count
            );
            SplitItemResult::Success {
                source_guid: src_item_guid,
                new_item_guid,
            }
        }
    }

    pub async fn update_durability(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        new_durability: u32,
    ) -> DurabilityResult {
        if !self.cache.has_player_inventory(player_guid) {
            return DurabilityResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return DurabilityResult::ItemNotFound,
        };

        let mut item_write = item.write();
        let max_durability = item_write.max_durability;

        if max_durability == 0 {
            return DurabilityResult::NoDurability;
        }

        let is_broken = new_durability == 0 && max_durability > 0;
        let clamped_durability = new_durability.min(max_durability);

        item_write.durability = clamped_durability;

        let bag = item_write.bag;
        let slot = item_write.slot;
        drop(item_write);

        if let Err(e) = self
            .repository
            .update_item_durability(item_guid.low(), clamped_durability as u16)
            .await
        {
            return DurabilityResult::DatabaseError(e.to_string());
        }

        self.send_slot_update(player_guid, bag, slot, Some(item_guid));

        if self.cache.is_equipment_slot(player_guid, slot) {
            self.send_visible_item_update(player_guid, slot, Some(item_guid));
        }

        DurabilityResult::Success {
            new_durability: clamped_durability,
            is_broken,
        }
    }

    pub async fn repair_item(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        _repair_cost: u32,
    ) -> DurabilityResult {
        if !self.cache.has_player_inventory(player_guid) {
            return DurabilityResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return DurabilityResult::ItemNotFound,
        };

        let mut item_write = item.write();
        let max_durability = item_write.max_durability;

        if max_durability == 0 {
            return DurabilityResult::NoDurability;
        }

        let is_broken = item_write.durability == 0;
        item_write.durability = max_durability;

        let bag = item_write.bag;
        let slot = item_write.slot;
        drop(item_write);

        if let Err(e) = self
            .repository
            .update_item_durability(item_guid.low(), max_durability as u16)
            .await
        {
            return DurabilityResult::DatabaseError(e.to_string());
        }

        self.send_slot_update(player_guid, bag, slot, Some(item_guid));

        if self.cache.is_equipment_slot(player_guid, slot) {
            self.send_visible_item_update(player_guid, slot, Some(item_guid));
        }

        DurabilityResult::Success {
            new_durability: max_durability,
            is_broken: false,
        }
    }

    pub async fn apply_enchantment(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        enchantment_id: u32,
        slot: u32,
    ) -> EnchantResult {
        if !self.cache.has_player_inventory(player_guid) {
            return EnchantResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return EnchantResult::ItemNotFound,
        };

        let mut item_write = item.write();

        if slot as usize >= item_write.enchantments.len() {
            return EnchantResult::InvalidSlot;
        }

        let enchantments = &mut item_write.enchantments;
        if enchantments.len() <= slot as usize {
            enchantments.resize(slot as usize + 1, (0, 0, 0));
        }
        enchantments[slot as usize] = (enchantment_id, 0, 0);

        let enchantments_str = Self::format_enchantments(&item_write.enchantments);
        let bag = item_write.bag;
        let slot_num = item_write.slot;
        drop(item_write);

        if let Err(e) = self
            .repository
            .update_item_enchantments(item_guid.low(), &enchantments_str)
            .await
        {
            return EnchantResult::DatabaseError(e.to_string());
        }

        self.send_slot_update(player_guid, bag, slot_num, Some(item_guid));

        if self.cache.is_equipment_slot(player_guid, slot_num) {
            self.send_visible_item_update(player_guid, slot_num, Some(item_guid));
        }

        EnchantResult::Success
    }

    pub async fn remove_enchantment(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        slot: u32,
    ) -> EnchantResult {
        if !self.cache.has_player_inventory(player_guid) {
            return EnchantResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return EnchantResult::ItemNotFound,
        };

        let mut item_write = item.write();

        if slot as usize >= item_write.enchantments.len() {
            return EnchantResult::InvalidSlot;
        }

        item_write.enchantments[slot as usize] = (0, 0, 0);

        let enchantments_str = Self::format_enchantments(&item_write.enchantments);
        let bag = item_write.bag;
        let slot_num = item_write.slot;
        drop(item_write);

        if let Err(e) = self
            .repository
            .update_item_enchantments(item_guid.low(), &enchantments_str)
            .await
        {
            return EnchantResult::DatabaseError(e.to_string());
        }

        self.send_slot_update(player_guid, bag, slot_num, Some(item_guid));

        if self.cache.is_equipment_slot(player_guid, slot_num) {
            self.send_visible_item_update(player_guid, slot_num, Some(item_guid));
        }

        EnchantResult::Success
    }

    pub async fn consume_charge(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        charge_index: u8,
    ) -> ChargeResult {
        if !self.cache.has_player_inventory(player_guid) {
            return ChargeResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return ChargeResult::ItemNotFound,
        };

        let mut item_write = item.write();

        if charge_index >= 5 {
            return ChargeResult::InvalidIndex;
        }

        let current_charges = item_write.spell_charges[charge_index as usize];
        if current_charges == 0 {
            return ChargeResult::NoCharges;
        }

        if current_charges > 0 {
            item_write.spell_charges[charge_index as usize] -= 1;
        }

        let charges_str = Self::format_spell_charges(&item_write.spell_charges);
        let new_remaining = item_write.spell_charges[charge_index as usize];
        drop(item_write);

        if let Err(e) = self
            .repository
            .update_item_charges(item_guid.low(), &charges_str)
            .await
        {
            return ChargeResult::DatabaseError(e.to_string());
        }

        ChargeResult::Success {
            remaining: new_remaining,
        }
    }

    const BUYBACK_START_SLOT: u8 = 69;
    const BUYBACK_SLOT_COUNT: u8 = 12;

    pub async fn add_to_buyback(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        sell_price: u32,
    ) -> BuybackResult {
        if !self.cache.has_player_inventory(player_guid) {
            return BuybackResult::PlayerNotLoaded;
        }

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return BuybackResult::ItemNotFound,
        };

        let item_read = item.read();
        let item_bag = item_read.bag;
        let item_slot = item_read.slot;
        let item_entry = item_read.entry;
        let item_count = item_read.count;
        drop(item_read);

        let buyback_slot = {
            let inv = match self.cache.get_player_inventory(player_guid) {
                Some(inv) => inv,
                None => return BuybackResult::PlayerNotLoaded,
            };

            let mut first_empty = None;
            for i in 0..Self::BUYBACK_SLOT_COUNT {
                let slot = Self::BUYBACK_START_SLOT + i;
                if inv.get_item_at(INVENTORY_SLOT_BAG_0, slot).is_none() {
                    first_empty = Some(slot);
                    break;
                }
            }

            match first_empty {
                Some(slot) => slot,
                None => {
                    let oldest_slot = Self::BUYBACK_START_SLOT;
                    self.cache
                        .set_item_at(player_guid, INVENTORY_SLOT_BAG_0, oldest_slot, None);
                    oldest_slot
                }
            }
        };

        self.cache
            .set_item_at(player_guid, item_bag, item_slot, None);
        self.cache.set_item_at(
            player_guid,
            INVENTORY_SLOT_BAG_0,
            buyback_slot,
            Some(item_guid),
        );

        self.send_slot_update(player_guid, item_bag, item_slot, None);
        self.send_slot_update(
            player_guid,
            INVENTORY_SLOT_BAG_0,
            buyback_slot,
            Some(item_guid),
        );

        BuybackResult::Added {
            slot: buyback_slot,
            item_guid,
        }
    }

    pub async fn retrieve_from_buyback(
        &self,
        player_guid: ObjectGuid,
        buyback_slot: u8,
    ) -> BuybackResult {
        if !self.cache.has_player_inventory(player_guid) {
            return BuybackResult::PlayerNotLoaded;
        }

        if buyback_slot < Self::BUYBACK_START_SLOT
            || buyback_slot >= Self::BUYBACK_START_SLOT + Self::BUYBACK_SLOT_COUNT
        {
            return BuybackResult::SlotNotFound;
        }

        let item_guid =
            match self
                .cache
                .get_item_at(player_guid, INVENTORY_SLOT_BAG_0, buyback_slot)
            {
                Some(g) => g,
                None => return BuybackResult::SlotNotFound,
            };

        let item = match self.cache.get_item(player_guid, item_guid) {
            Some(i) => i,
            None => return BuybackResult::ItemNotFound,
        };

        let (dst_bag, dst_slot) = match self.cache.find_free_inventory_slot(player_guid) {
            Some(pos) => pos,
            None => return BuybackResult::DatabaseError("No space".to_string()),
        };

        self.cache
            .set_item_at(player_guid, INVENTORY_SLOT_BAG_0, buyback_slot, None);
        self.cache
            .set_item_at(player_guid, dst_bag, dst_slot, Some(item_guid));

        self.send_slot_update(player_guid, INVENTORY_SLOT_BAG_0, buyback_slot, None);
        self.send_slot_update(player_guid, dst_bag, dst_slot, Some(item_guid));

        let item_read = item.read();
        let price = self
            .item_mgr
            .get_template(item_read.entry)
            .map(|t| t.sell_price * item_read.count)
            .unwrap_or(0);
        drop(item_read);

        BuybackResult::Retrieved { item_guid, price }
    }

    pub async fn equip_item(
        &self,
        player_guid: ObjectGuid,
        src_bag: u8,
        src_slot: u8,
        equip_slot: u8,
        _player_level: u8,
        _player_class: u8,
        _player_race: u8,
    ) -> EquipResult {
        tracing::debug!(
            "[EQUIP_ITEM] Player={:?} attempting to equip from bag={} slot={} to equip_slot={}",
            player_guid,
            src_bag,
            src_slot,
            equip_slot
        );

        // === VALIDATION PHASE - Check all preconditions before making any changes ===

        if !self.cache.has_player_inventory(player_guid) {
            tracing::error!("[EQUIP_ITEM] FAILED: player not loaded {:?}", player_guid);
            return EquipResult::PlayerNotLoaded;
        }

        // Validate equip slot range
        if equip_slot >= EquipmentSlotEnum::END as u8 {
            tracing::warn!(
                "[EQUIP_ITEM] FAILED: Invalid equip_slot={} (max={})",
                equip_slot,
                EquipmentSlotEnum::END as u8
            );
            self.send_inventory_error(
                player_guid,
                crate::shared::messages::EQUIP_ERR_ITEM_DOESNT_GO_TO_SLOT,
            );
            return EquipResult::WrongSlot;
        }

        // Get source item
        let src_item_guid = match self.cache.get_item_at(player_guid, src_bag, src_slot) {
            Some(g) => g,
            None => {
                tracing::warn!(
                    "[EQUIP_ITEM] FAILED: No item at src bag={} slot={}",
                    src_bag,
                    src_slot
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                );
                return EquipResult::ItemNotFound;
            }
        };

        // Get source item details
        let src_item = match self.cache.get_item(player_guid, src_item_guid) {
            Some(i) => i,
            None => {
                tracing::error!(
                    "[EQUIP_ITEM] FAILED: Item object not found for {:?}",
                    src_item_guid
                );
                self.send_inventory_error(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
                );
                return EquipResult::ItemNotFound;
            }
        };

        let (item_entry, item_name) = {
            let item_read = src_item.read();
            let name = self
                .item_mgr
                .get_template(item_read.entry)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            (item_read.entry, name)
        };

        tracing::debug!(
            "[EQUIP_ITEM] Equipping item_entry={} ({}) from bag={} slot={} to equip_slot={}",
            item_entry,
            item_name,
            src_bag,
            src_slot,
            equip_slot
        );

        // Check if there's an item already equipped in the target slot
        let existing_item_guid =
            self.cache
                .get_item_at(player_guid, INVENTORY_SLOT_BAG_0, equip_slot);

        // === EXECUTION PHASE - All validations passed, now execute ===

        if let Some(existing_guid) = existing_item_guid {
            // There's already an item equipped - need to swap

            // Find a free slot for the unequipped item
            let free_slot = match self.cache.find_free_inventory_slot(player_guid) {
                Some((bag, slot)) => (bag, slot),
                None => {
                    tracing::warn!(
                        "[EQUIP_ITEM] FAILED: No free inventory slot to unequip existing item"
                    );
                    self.send_inventory_error_with_items(
                        player_guid,
                        crate::shared::messages::EQUIP_ERR_INVENTORY_FULL,
                        Some(src_item_guid),
                        Some(existing_guid),
                    );
                    return EquipResult::InventoryFull;
                }
            };

            tracing::debug!(
                "[EQUIP_ITEM] Swapping: new item {:?} to equip_slot={}, existing item {:?} to bag={} slot={}",
                src_item_guid, equip_slot, existing_guid, free_slot.0, free_slot.1
            );

            // First database operation: swap the items in the equipment slot
            if let Err(e) = self
                .repository
                .swap_items(
                    player_guid.low(),
                    src_item_guid.low(),
                    src_bag,
                    src_slot,
                    Some(existing_guid.low()),
                    INVENTORY_SLOT_BAG_0,
                    equip_slot,
                )
                .await
            {
                tracing::error!("[EQUIP_ITEM] FAILED: Database error during swap: {}", e);
                self.send_inventory_error_with_items(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                    Some(src_item_guid),
                    Some(existing_guid),
                );
                return EquipResult::DatabaseError(e.to_string());
            }

            // Second database operation: move the unequipped item to inventory
            if let Err(e) = self
                .repository
                .move_item(
                    player_guid.low(),
                    existing_guid.low(),
                    free_slot.0,
                    free_slot.1,
                )
                .await
            {
                tracing::error!(
                    "[EQUIP_ITEM] FAILED: Database error moving unequipped item: {}",
                    e
                );
                // Note: At this point, the swap already happened, so we have a partial state
                // This is a serious error that requires manual intervention or rollback logic
                self.send_inventory_error_with_items(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                    Some(src_item_guid),
                    Some(existing_guid),
                );
                return EquipResult::DatabaseError(e.to_string());
            }

            // Update cache after successful database operations
            self.cache.set_item_at(player_guid, src_bag, src_slot, None);
            self.cache.set_item_at(
                player_guid,
                INVENTORY_SLOT_BAG_0,
                equip_slot,
                Some(src_item_guid),
            );
            self.cache
                .set_item_at(player_guid, free_slot.0, free_slot.1, Some(existing_guid));

            // Send updates to client
            self.send_slot_update(player_guid, src_bag, src_slot, None);
            self.send_slot_update(
                player_guid,
                INVENTORY_SLOT_BAG_0,
                equip_slot,
                Some(src_item_guid),
            );
            self.send_slot_update(player_guid, free_slot.0, free_slot.1, Some(existing_guid));

            self.send_visible_item_update(player_guid, equip_slot, Some(src_item_guid));

            tracing::debug!(
                "[EQUIP_ITEM] SUCCESS: Swapped items, unequipped item moved to bag={} slot={}",
                free_slot.0,
                free_slot.1
            );

            EquipResult::Swapped {
                unequipped_to_bag: free_slot.0,
                unequipped_to_slot: free_slot.1,
            }
        } else {
            // No item equipped - simple equip

            tracing::debug!(
                "[EQUIP_ITEM] Simple equip: item {:?} to equip_slot={}",
                src_item_guid,
                equip_slot
            );

            // Move item to equipment slot in database
            if let Err(e) = self
                .repository
                .move_item(
                    player_guid.low(),
                    src_item_guid.low(),
                    INVENTORY_SLOT_BAG_0,
                    equip_slot,
                )
                .await
            {
                tracing::error!("[EQUIP_ITEM] FAILED: Database error during equip: {}", e);
                self.send_inventory_error_with_items(
                    player_guid,
                    crate::shared::messages::EQUIP_ERR_INT_BAG_ERROR,
                    Some(src_item_guid),
                    None,
                );
                return EquipResult::DatabaseError(e.to_string());
            }

            // Update cache after successful database operation
            self.cache.set_item_at(player_guid, src_bag, src_slot, None);
            self.cache.set_item_at(
                player_guid,
                INVENTORY_SLOT_BAG_0,
                equip_slot,
                Some(src_item_guid),
            );

            // Send updates to client
            self.send_slot_update(player_guid, src_bag, src_slot, None);
            self.send_slot_update(
                player_guid,
                INVENTORY_SLOT_BAG_0,
                equip_slot,
                Some(src_item_guid),
            );
            self.send_visible_item_update(player_guid, equip_slot, Some(src_item_guid));

            tracing::debug!("[EQUIP_ITEM] SUCCESS: Item equipped to slot {}", equip_slot);

            EquipResult::Equipped
        }
    }

    pub async fn unequip_item(&self, player_guid: ObjectGuid, equip_slot: u8) -> EquipResult {
        if !self.cache.has_player_inventory(player_guid) {
            return EquipResult::PlayerNotLoaded;
        }

        if equip_slot >= EquipmentSlotEnum::END as u8 {
            self.send_inventory_error(player_guid, crate::shared::messages::ERR_NOT_EQUIPPABLE);
            return EquipResult::WrongSlot;
        }

        let item_guid = match self
            .cache
            .get_item_at(player_guid, INVENTORY_SLOT_BAG_0, equip_slot)
        {
            Some(g) => g,
            None => {
                self.send_inventory_error(player_guid, crate::shared::messages::ERR_ITEM_NOT_FOUND);
                return EquipResult::ItemNotFound;
            }
        };

        let free_slot = match self.cache.find_free_inventory_slot(player_guid) {
            Some((bag, slot)) => (bag, slot),
            None => {
                self.send_inventory_error(player_guid, crate::shared::messages::ERR_INV_FULL);
                return EquipResult::InventoryFull;
            }
        };

        if let Err(e) = self
            .repository
            .move_item(player_guid.low(), item_guid.low(), free_slot.0, free_slot.1)
            .await
        {
            return EquipResult::DatabaseError(e.to_string());
        }

        self.cache
            .set_item_at(player_guid, INVENTORY_SLOT_BAG_0, equip_slot, None);
        self.cache
            .set_item_at(player_guid, free_slot.0, free_slot.1, Some(item_guid));

        self.send_slot_update(player_guid, INVENTORY_SLOT_BAG_0, equip_slot, None);
        self.send_slot_update(player_guid, free_slot.0, free_slot.1, Some(item_guid));
        self.send_visible_item_update(player_guid, equip_slot, None);

        self.send_item_update(player_guid, item_guid);

        EquipResult::Unequipped
    }

    pub fn get_equipment_for_char_enum(&self, player_guid: ObjectGuid) -> [EquipmentSlot; 19] {
        let equipment = self.cache.get_equipment_slots(player_guid);
        let mut result = [EquipmentSlot {
            display_id: 0,
            inventory_type: 0,
        }; 19];

        for (slot, item_guid) in equipment {
            if let Some(item) = self.cache.get_item(player_guid, item_guid) {
                let item = item.read();
                if let Some(template) = self.item_mgr.get_template(item.entry) {
                    result[slot as usize] = EquipmentSlot {
                        display_id: template.display_id,
                        inventory_type: template.inventory_type,
                    };
                }
            }
        }

        result
    }

    /// Build CREATE_OBJECT blocks for all player items
    /// Returns the number of item blocks created
    /// Matches mangos-classic Player::BuildCreateUpdateBlockForPlayer() item sending
    pub fn build_item_create_blocks(
        &self,
        player_guid: ObjectGuid,
        blocks: &mut Vec<CreateObjectBlock>,
    ) -> usize {
        let mut count = 0;

        // Equipment slots (0-18)
        for slot in 0..19u8 {
            if let Some(item_guid) = self.cache.get_item_at(player_guid, 255, slot) {
                if let Some(item) = self.cache.get_item(player_guid, item_guid) {
                    let item_read = item.read();
                    blocks.push(item_read.to_create_block());
                    count += 1;
                }
            }
        }

        // Bag slots (19-22) - send the bags themselves
        for slot in 19..23u8 {
            if let Some(bag_guid) = self.cache.get_item_at(player_guid, 255, slot) {
                if let Some(bag) = self.cache.get_item(player_guid, bag_guid) {
                    let bag_read = bag.read();
                    blocks.push(bag_read.to_create_block());
                    count += 1;

                    // Also send items inside the bag
                    // Bag slots inside bag are typically 0-35
                    for bag_slot in 0..36u8 {
                        if let Some(item_guid) = self.cache.get_item_at(player_guid, slot, bag_slot)
                        {
                            if let Some(item) = self.cache.get_item(player_guid, item_guid) {
                                let item_read = item.read();
                                blocks.push(item_read.to_create_block());
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        // Inventory slots (23-38)
        for slot in 23..39u8 {
            if let Some(item_guid) = self.cache.get_item_at(player_guid, 255, slot) {
                if let Some(item) = self.cache.get_item(player_guid, item_guid) {
                    let item_read = item.read();
                    blocks.push(item_read.to_create_block());
                    count += 1;
                }
            }
        }

        count
    }

    pub fn send_player_inventory(&self, player_guid: ObjectGuid) {
        let items = self.cache.get_all_items(player_guid);

        if items.is_empty() {
            tracing::warn!("[INVENTORY] No items to send for player {:?}", player_guid);
            return;
        }

        let mut update = SmsgUpdateObject::new();

        for item_arc in items {
            let item = item_arc.read();
            update = update.add_block(UpdateBlockData::CreateObject2(item.to_create_block()));
        }

        self.broadcast_mgr.send_msg_to_player(player_guid, update);
    }

    pub fn send_money_update(&self, player_guid: ObjectGuid, money: u32) {
        use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;

        tracing::debug!(
            "[INVENTORY] send_money_update: player={:?} money={}",
            player_guid,
            money
        );

        // Use from_raw to preserve the full GUID (high + low parts)
        // This ensures the client receives the correct player GUID format
        let world_guid = WorldObjectGuid::from_raw(player_guid.raw());
        tracing::debug!(
            "[INVENTORY] send_money_update: converted world_guid={:?}",
            world_guid
        );

        let money_msg = SmsgPlayerMoneyUpdate {
            guid: world_guid,
            money,
        };

        tracing::debug!(
            "[INVENTORY] send_money_update: sending packet to player {:?}",
            player_guid
        );
        self.broadcast_mgr
            .send_msg_to_player(player_guid, money_msg);
        tracing::debug!("[INVENTORY] send_money_update: packet sent");
    }

    /// Populate visible item fields for player create/update block
    /// Sets PLAYER_VISIBLE_ITEM_1_0 + slot*12 = item_entry for character model rendering
    /// Also sets UNIT_VIRTUAL_ITEM_SLOT_DISPLAY for weapon slots
    pub fn populate_visible_items(&self, player_guid: ObjectGuid, fields: &mut Vec<(u32, u32)>) {
        use crate::world::core::common::HighGuid;
        use crate::world::game::common::update_fields::{
            visible_item_entry_field, UNIT_VIRTUAL_ITEM_SLOT_DISPLAY,
        };

        const EQUIPMENT_SLOT_MAINHAND: u8 = 15;
        const EQUIPMENT_SLOT_OFFHAND: u8 = 16;
        const EQUIPMENT_SLOT_RANGED: u8 = 17;

        info!("[INVENTORY] populate_visible_items for {:?}", player_guid);

        for slot in 0..19u8 {
            if let Some(item_guid) = self.cache.get_item_at(player_guid, 255, slot) {
                if let Some(item) = self.cache.get_item(player_guid, item_guid) {
                    let item_read = item.read();
                    let item_entry = item_read.entry;

                    // Set visible item entry (field 0) - item entry ID for character model
                    let visible_base = visible_item_entry_field(slot);
                    fields.push((visible_base, item_entry));

                    // Set enchantment fields (fields 1-2) - 0x40000000 as default
                    fields.push((visible_base + 1, 0x40000000));
                    fields.push((visible_base + 2, 0x40000000));

                    // Set UNIT_VIRTUAL_ITEM_SLOT_DISPLAY for weapon slots (for visual display)
                    if slot == EQUIPMENT_SLOT_MAINHAND
                        || slot == EQUIPMENT_SLOT_OFFHAND
                        || slot == EQUIPMENT_SLOT_RANGED
                    {
                        if let Some(proto) = self.item_mgr.get_template(item_entry) {
                            let virtual_slot = match slot {
                                EQUIPMENT_SLOT_MAINHAND => 0,
                                EQUIPMENT_SLOT_OFFHAND => 1,
                                EQUIPMENT_SLOT_RANGED => 2,
                                _ => unreachable!(),
                            };
                            fields.push((
                                UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + virtual_slot,
                                proto.display_id,
                            ));
                        } else {
                            info!(
                                "[INVENTORY]    No template found for item_entry={}",
                                item_entry
                            );
                        }
                    }
                } else {
                    info!(
                        "[INVENTORY]    Item not found in cache for guid={:?}",
                        item_guid
                    );
                }
            }
        }
    }

    /// Populate inventory slot fields for player create/update block
    /// Sets PLAYER_FIELD_INV_SLOT_HEAD for equipment/bag slots (0-22)
    /// Sets PLAYER_FIELD_PACK_SLOT_1 for inventory slots (23-38)
    ///
    /// Returns a list of (field_index, item_guid) tuples that should be set using set_guid_field
    pub fn populate_inventory_slots(
        &self,
        player_guid: ObjectGuid,
        guid_fields: &mut Vec<(u32, crate::world::core::common::guid::ObjectGuid)>,
    ) {
        use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
        use crate::world::game::common::update_fields::{
            PLAYER_FIELD_INV_SLOT_HEAD, PLAYER_FIELD_PACK_SLOT_1,
        };
        use crate::world::game::inventory::inventory_types::{
            INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START,
        };

        let fields_before = guid_fields.len();

        // Equipment slots (0-18) and bag slots (19-22) -> PLAYER_FIELD_INV_SLOT_HEAD
        for slot in 0..23u8 {
            if let Some(item_guid) = self.cache.get_item_at(player_guid, 255, slot) {
                let field_index = PLAYER_FIELD_INV_SLOT_HEAD + (slot as u32 * 2);
                let world_guid = WorldObjectGuid::from_raw(item_guid.raw());
                guid_fields.push((field_index, world_guid));
            }
        }

        // Inventory slots (23-38) -> PLAYER_FIELD_PACK_SLOT_1
        for slot in INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END {
            if let Some(item_guid) = self.cache.get_item_at(player_guid, 255, slot) {
                let slot_offset = (slot - INVENTORY_SLOT_ITEM_START) as u32;
                let field_index = PLAYER_FIELD_PACK_SLOT_1 + (slot_offset * 2);
                let world_guid = WorldObjectGuid::from_raw(item_guid.raw());
                guid_fields.push((field_index, world_guid));
            }
        }

        let fields_added = guid_fields.len() - fields_before;
    }

    // ========== FLUSH / LIFECYCLE METHODS ==========

    /// Flush all pending DB operations for all players.
    /// Called periodically from the world update loop.
    pub async fn flush_pending_ops(&self) -> Result<()> {
        let all_ops = self.cache.take_all_pending_ops();
        if all_ops.is_empty() {
            return Ok(());
        }

        let player_count = all_ops.len();
        let mut total_ops = 0;

        for (player_guid, ops) in all_ops {
            total_ops += ops.len();
            if let Err(e) = self.flush_player_ops(player_guid, ops).await {
                tracing::error!(
                    "[INVENTORY] Failed to flush ops for player {:?}: {}",
                    player_guid,
                    e
                );
            }
        }

        if total_ops > 0 {
            tracing::debug!(
                "[INVENTORY] Flushed {} ops for {} players",
                total_ops,
                player_count
            );
        }

        Ok(())
    }

    /// Flush pending DB operations for a single player.
    /// Collapses redundant operations before executing.
    async fn flush_player_ops(
        &self,
        _player_guid: ObjectGuid,
        ops: Vec<super::cache::PendingInventoryOp>,
    ) -> Result<()> {
        use super::cache::PendingInventoryOp;
        use std::collections::HashMap;

        // Collapse redundant operations
        let mut final_money: Option<(u32, u32)> = None; // (player_guid, amount)
        let mut final_counts: HashMap<u32, u32> = HashMap::new(); // item_guid -> count
        let mut deletes: Vec<u32> = Vec::new(); // item_guids to delete
        let mut creates: Vec<(crate::shared::database::characters::models::item::ItemInstanceRow, crate::shared::database::characters::repositories::inventory_repository_trait::InventorySlotRow)> = Vec::new();
        let mut moves: HashMap<u32, (u32, u8, u8)> = HashMap::new(); // item_guid -> (player_guid, bag, slot)
        let mut swaps: Vec<(u32, u32, u8, u8, Option<u32>, u8, u8)> = Vec::new();

        for op in ops {
            match op {
                PendingInventoryOp::UpdateMoney {
                    player_guid,
                    amount,
                } => {
                    final_money = Some((player_guid, amount));
                }
                PendingInventoryOp::UpdateCount { item_guid, count } => {
                    if !deletes.contains(&item_guid) {
                        final_counts.insert(item_guid, count);
                    }
                }
                PendingInventoryOp::DeleteItem { item_guid } => {
                    // Remove any pending count updates for this item
                    final_counts.remove(&item_guid);
                    moves.remove(&item_guid);
                    deletes.push(item_guid);
                }
                PendingInventoryOp::CreateItem { item, slot } => {
                    creates.push((item, slot));
                }
                PendingInventoryOp::MoveItem {
                    player_guid,
                    item_guid,
                    bag,
                    slot,
                } => {
                    // Later move overrides earlier move for same item
                    moves.insert(item_guid, (player_guid, bag, slot));
                }
                PendingInventoryOp::SwapItems {
                    player_guid,
                    item1_guid,
                    bag1,
                    slot1,
                    item2_guid,
                    bag2,
                    slot2,
                } => {
                    // Swaps are harder to collapse, just queue them
                    // But remove any pending moves for these items since the swap supersedes them
                    moves.remove(&item1_guid);
                    if let Some(g) = item2_guid {
                        moves.remove(&g);
                    }
                    swaps.push((
                        player_guid,
                        item1_guid,
                        bag1,
                        slot1,
                        item2_guid,
                        bag2,
                        slot2,
                    ));
                }
            }
        }

        // Execute collapsed operations

        // 1. Creates first (new items must exist before moves/swaps reference them)
        for (item, slot) in &creates {
            if let Err(e) = self.repository.create_item(item, slot).await {
                tracing::error!("[FLUSH] Failed to create item {}: {}", item.guid, e);
            }
        }

        // 2. Swaps
        for (player_guid, item1_guid, bag1, slot1, item2_guid, bag2, slot2) in &swaps {
            if let Err(e) = self
                .repository
                .swap_items(
                    *player_guid,
                    *item1_guid,
                    *bag1,
                    *slot1,
                    *item2_guid,
                    *bag2,
                    *slot2,
                )
                .await
            {
                tracing::error!("[FLUSH] Failed to swap items: {}", e);
            }
        }

        // 3. Moves
        for (item_guid, (player_guid, bag, slot)) in &moves {
            if let Err(e) = self
                .repository
                .move_item(*player_guid, *item_guid, *bag, *slot)
                .await
            {
                tracing::error!("[FLUSH] Failed to move item {}: {}", item_guid, e);
            }
        }

        // 4. Count updates (batched)
        if !final_counts.is_empty() {
            let updates: Vec<(u32, u32)> = final_counts.into_iter().collect();
            if let Err(e) = self.repository.batch_update_counts(&updates).await {
                tracing::error!("[FLUSH] Failed to batch update counts: {}", e);
            }
        }

        // 5. Deletes
        for item_guid in &deletes {
            if let Err(e) = self.repository.delete_item(*item_guid).await {
                tracing::error!("[FLUSH] Failed to delete item {}: {}", item_guid, e);
            }
        }

        // 6. Money update (just the final value)
        if let Some((player_guid, amount)) = final_money {
            if let Err(e) = self
                .repository
                .update_player_money(player_guid, amount)
                .await
            {
                tracing::error!(
                    "[FLUSH] Failed to update money for player {}: {}",
                    player_guid,
                    e
                );
            }
        }

        Ok(())
    }

    pub async fn init(&self) -> Result<()> {
        tracing::debug!("InventorySystem initialized");
        Ok(())
    }

    pub fn update(&self, _diff: Duration) -> Result<()> {
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        tracing::debug!("[InventorySystem] flushing pending ops before shutdown...");
        self.flush_pending_ops().await?;
        tracing::debug!("[InventorySystem] shutdown complete");
        Ok(())
    }

    pub fn on_player_login(&self, _guid: ObjectGuid) -> Result<()> {
        Ok(())
    }

    /// Flush pending DB ops for this player, then unload their inventory cache.
    pub async fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        let ops = self.cache.take_pending_ops(guid);
        if !ops.is_empty() {
            tracing::debug!(
                "[INVENTORY] Flushing {} pending ops for player {:?} on logout",
                ops.len(),
                guid
            );
            if let Err(e) = self.flush_player_ops(guid, ops).await {
                tracing::error!(
                    "[INVENTORY] Failed to flush ops on logout for player {:?}: {}",
                    guid,
                    e
                );
            }
        }
        self.unload_player_inventory(guid);
        Ok(())
    }
}
