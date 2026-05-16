//! Vendor System - business logic for NPC vendors
//!
//! Handles sending vendor lists to players, processing buy/sell transactions,
//! reputation discounts, and stock management.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::manager::VendorManager;
use super::types::{ReputationRank, VendorItem};
use crate::shared::messages::vendor::{
    BuyError, BuyResult, SellResult, SmsgBuyFailed, SmsgBuyItem, SmsgListInventory, SmsgSellItem,
    VendorItemData,
};
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::world::game::creature::CreatureManager;
use crate::world::game::inventory::InventorySystem;
use crate::world::game::items::ItemManager;
use crate::world::game::player::PlayerManager;
use crate::world::World;

/// Vendor system - handles vendor business logic
pub struct VendorSystem {
    manager: Arc<VendorManager>,
    broadcast_mgr: Arc<BroadcastManager>,
    creature_mgr: Arc<CreatureManager>,
    player_mgr: Arc<PlayerManager>,
    item_mgr: Arc<ItemManager>,
    inventory: Arc<InventorySystem>,
}

impl VendorSystem {
    /// Create a new vendor system
    pub fn new(
        manager: Arc<VendorManager>,
        broadcast_mgr: Arc<BroadcastManager>,
        creature_mgr: Arc<CreatureManager>,
        player_mgr: Arc<PlayerManager>,
        item_mgr: Arc<ItemManager>,
        inventory: Arc<InventorySystem>,
    ) -> Self {
        Self {
            manager,
            broadcast_mgr,
            creature_mgr,
            player_mgr,
            item_mgr,
            inventory,
        }
    }

    /// Initialize the vendor system
    pub async fn init(&self) -> Result<()> {
        // Nothing to initialize yet

        Ok(())
    }

    /// Shutdown the vendor system
    pub async fn shutdown(&self) -> Result<()> {
        // Nothing to shutdown yet
        Ok(())
    }

    /// Send vendor list to player
    pub async fn send_vendor_list(
        &self,
        player_guid: ObjectGuid,
        vendor_guid: ObjectGuid,
    ) -> Result<()> {
        debug!(
            "Sending vendor list to player {:?} from vendor {:?}",
            player_guid, vendor_guid
        );

        // Get creature entry
        let entry = {
            let creature = self
                .creature_mgr
                .get_creature(vendor_guid)
                .ok_or_else(|| anyhow::anyhow!("Vendor {:?} not found", vendor_guid))?;
            creature.entry
        };

        // Get vendor items (includes template items)
        let items = self.manager.get_vendor_items(entry);

        // Filter by conditions
        let visible_items: Vec<VendorItem> = items
            .into_iter()
            .filter(|item| {
                if item.condition_id > 0 {
                    // TODO: Check condition via ConditionSystem
                    // For now, assume condition passes
                    true
                } else {
                    true
                }
            })
            .collect();

        // Initialize stock if needed
        self.manager.initialize_stock(vendor_guid, &visible_items);

        // Get current stock
        let stock = self.manager.get_stock(vendor_guid);

        // Get reputation discount
        let discount = self.calculate_reputation_discount(player_guid, entry);

        // Build item data
        let item_data: Vec<VendorItemData> = visible_items
            .iter()
            .enumerate()
            .filter_map(|(index, vendor_item)| {
                let template = self.item_mgr.get_template(vendor_item.item_entry)?;

                // Get current stock (0xFFFFFFFF = unlimited)
                let current_stock = if vendor_item.max_count == 0 {
                    0xFFFFFFFFu32 // Unlimited
                } else {
                    stock
                        .iter()
                        .find(|s| s.item_entry == vendor_item.item_entry)
                        .map(|s| s.count)
                        .unwrap_or(vendor_item.max_count as u32)
                };

                // Apply discount to price
                let discounted_price = (template.buy_price as f32 * discount + 0.5) as u32;

                Some(VendorItemData {
                    index: (index + 1) as u32,
                    item_id: vendor_item.item_entry,
                    display_id: template.display_id,
                    max_count: current_stock,
                    price: discounted_price,
                    max_durability: template.max_durability,
                    buy_count: 1, // Default stack size
                })
            })
            .collect();

        // Build and send message
        let msg = SmsgListInventory {
            vendor_guid,
            items: item_data,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        info!(
            "Sent vendor list to player {:?} from vendor {:?} (entry {})",
            player_guid, vendor_guid, entry
        );

        Ok(())
    }

    /// Handle buy item request
    pub async fn handle_buy_item(
        &self,
        player_guid: ObjectGuid,
        vendor_guid: ObjectGuid,
        item_id: u32,
        count: u8,
    ) -> Result<()> {
        info!(
            "Player {:?} buying item {} x{} from vendor {:?}",
            player_guid, item_id, count, vendor_guid
        );

        // Get vendor entry
        let entry = {
            let creature = self
                .creature_mgr
                .get_creature(vendor_guid)
                .ok_or_else(|| anyhow::anyhow!("Vendor {:?} not found", vendor_guid))?;
            creature.entry
        };

        debug!("Buy item: vendor entry={}, item_id={}", entry, item_id);

        // Get item template
        let template = match self.item_mgr.get_template(item_id) {
            Some(t) => t,
            None => {
                warn!(
                    "Buy failed: item template {} not found in item_mgr",
                    item_id
                );
                self.send_buy_failed(player_guid, vendor_guid, item_id, BuyError::CantFind);
                return Ok(());
            }
        };

        // Get vendor items and find the specific item
        let items = self.manager.get_vendor_items(entry);
        let vendor_item = match items.iter().find(|item| item.item_entry == item_id) {
            Some(item) => item.clone(),
            None => {
                warn!(
                    "Buy failed: item {} not in vendor entry {} item list ({} items: {:?})",
                    item_id,
                    entry,
                    items.len(),
                    items.iter().map(|i| i.item_entry).collect::<Vec<_>>()
                );
                self.send_buy_failed(player_guid, vendor_guid, item_id, BuyError::CantFind);
                return Ok(());
            }
        };

        // Calculate price with discount
        let discount = self.calculate_reputation_discount(player_guid, entry);
        let unit_price = (template.buy_price as f32 * discount + 0.5) as u32;
        let total_price = unit_price.saturating_mul(count as u32);

        // Check player money (from inventory system, the authoritative source)
        let player_money = self.inventory.get_money(player_guid).unwrap_or(0);

        if player_money < total_price {
            self.send_buy_failed(player_guid, vendor_guid, item_id, BuyError::NotEnoughMoney);
            return Ok(());
        }

        // Check stock for limited items
        if vendor_item.max_count > 0 {
            let current_stock = self
                .manager
                .get_item_stock(vendor_guid, item_id)
                .unwrap_or(vendor_item.max_count as u32);

            if current_stock < count as u32 {
                self.send_buy_failed(player_guid, vendor_guid, item_id, BuyError::ItemSoldOut);
                return Ok(());
            }
        }

        // Try to add items to inventory
        let add_result = self
            .inventory
            .add_item(player_guid, item_id, count as u32)
            .await;

        match add_result {
            crate::world::game::inventory::AddItemResult::Success { .. } => {
                // Success - deduct money (updates cache, DB, and sends client update)
                self.inventory.remove_gold(player_guid, total_price);

                // Reduce stock for limited items
                if vendor_item.max_count > 0 {
                    for _ in 0..count {
                        self.manager.reduce_stock(vendor_guid, item_id);
                    }
                }

                // Find vendor slot (1-based index)
                let vendor_slot = items
                    .iter()
                    .position(|item| item.item_entry == item_id)
                    .map(|i| (i + 1) as u32)
                    .unwrap_or(1);

                // Calculate remaining stock
                let remaining_stock = if vendor_item.max_count == 0 {
                    0xFFFFFFFF // Unlimited
                } else {
                    self.manager
                        .get_item_stock(vendor_guid, item_id)
                        .unwrap_or(0)
                };

                // Send success message
                let msg = SmsgBuyItem {
                    vendor_guid,
                    vendor_slot,
                    remaining_stock,
                    count: count as u32,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                info!(
                    "Player {:?} bought item {} x{} for {} copper",
                    player_guid, item_id, count, total_price
                );
            }
            _ => {
                // Failed to add item (bag full, etc.)
                self.send_buy_failed(player_guid, vendor_guid, item_id, BuyError::CantCarryMore);
            }
        }

        Ok(())
    }

    /// Handle sell item request
    pub async fn handle_sell_item(
        &self,
        player_guid: ObjectGuid,
        vendor_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) -> Result<()> {
        info!(
            "Player {:?} selling item {:?} to vendor {:?}",
            player_guid, item_guid, vendor_guid
        );

        // If vendor_guid is empty (0), try to get from player's current selection
        let vendor_guid = if vendor_guid.is_empty() {
            self.player_mgr
                .get_selection(player_guid)
                .unwrap_or(ObjectGuid::empty())
        } else {
            vendor_guid
        };

        // Validate vendor GUID is not empty
        if vendor_guid.is_empty() {
            warn!("Sell item failed: no vendor GUID and no selection");
            let msg = SmsgSellItem {
                vendor_guid,
                item_guid,
                result: SellResult::CantFindVendor,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Verify vendor exists
        if self.creature_mgr.get_creature(vendor_guid).is_none() {
            let msg = SmsgSellItem {
                vendor_guid,
                item_guid,
                result: SellResult::CantFindVendor,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Validate item_guid is not empty
        if item_guid.is_empty() {
            let msg = SmsgSellItem {
                vendor_guid,
                item_guid,
                result: SellResult::CantFindItem,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Get item from inventory cache
        let item = match self.inventory.cache().get_item(player_guid, item_guid) {
            Some(item) => item,
            None => {
                warn!(
                    "Sell item failed: item {:?} not found in inventory",
                    item_guid
                );
                let msg = SmsgSellItem {
                    vendor_guid,
                    item_guid,
                    result: SellResult::CantFindItem,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        };

        // Get item entry and count
        let (entry_id, count) = {
            let item_read = item.read();
            (item_read.entry, item_read.count)
        };

        // Get item template
        let template = match self.item_mgr.get_template(entry_id) {
            Some(t) => t,
            None => {
                warn!("Sell item failed: template {} not found", entry_id);
                let msg = SmsgSellItem {
                    vendor_guid,
                    item_guid,
                    result: SellResult::CantFindItem,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        };

        // Validate item can be sold (sell_price > 0)
        if template.sell_price == 0 {
            warn!("Sell item failed: item {} has sell_price=0", entry_id);
            let msg = SmsgSellItem {
                vendor_guid,
                item_guid,
                result: SellResult::CantSellItem,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Calculate total sell price
        let total_price = template.sell_price.saturating_mul(count);

        // Remove item from inventory (sends SMSG_UPDATE_OBJECT slot clear + SMSG_DESTROY_OBJECT)
        let remove_result = self.inventory.remove_item(player_guid, item_guid, count);
        match remove_result {
            crate::world::game::inventory::RemoveItemResult::ItemRemoved { .. }
            | crate::world::game::inventory::RemoveItemResult::CountReduced { .. } => {
                // Success - add money (updates cache, DB, and sends client update)
                self.inventory.add_gold(player_guid, total_price);

                // Send success message
                let msg = SmsgSellItem {
                    vendor_guid,
                    item_guid,
                    result: SellResult::Ok,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                info!(
                    "Player {:?} sold item {} (x{}) to vendor {:?} for {} copper",
                    player_guid, entry_id, count, vendor_guid, total_price
                );
            }
            _ => {
                warn!(
                    "Sell item failed: couldn't remove item {:?} from inventory",
                    item_guid
                );
                let msg = SmsgSellItem {
                    vendor_guid,
                    item_guid,
                    result: SellResult::CantFindItem,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            }
        }

        Ok(())
    }

    /// Calculate reputation discount for a player at a vendor
    fn calculate_reputation_discount(&self, player_guid: ObjectGuid, vendor_entry: u32) -> f32 {
        // Get vendor faction from creature template
        let faction = self
            .creature_mgr
            .get_template(vendor_entry)
            .map(|t| t.faction)
            .unwrap_or(0);

        if faction == 0 {
            return 1.0; // No discount
        }

        // Get player reputation standing
        // TODO: Get actual reputation from player
        // For now, return no discount
        let standing = ReputationRank::Neutral;

        standing.discount_multiplier()
    }

    /// Send buy failed message
    fn send_buy_failed(
        &self,
        player_guid: ObjectGuid,
        vendor_guid: ObjectGuid,
        item_id: u32,
        error: BuyError,
    ) {
        let msg = SmsgBuyFailed {
            vendor_guid,
            item_id,
            error,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }
}
