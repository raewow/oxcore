//! Vendor Manager - state storage and database loading
//!
//! Manages vendor item data loaded from the database.
//! Provides thread-safe access using DashMap.

use anyhow::Result;
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::info;

use super::types::{ItemExtendedCost, VendorItem, VendorItemCount};
use crate::shared::database::world::repositories::VendorRepository;
use crate::shared::protocol::ObjectGuid;

/// Manages vendor data (state storage + database loading)
pub struct VendorManager {
    /// Database pool for loading
    world_db: Arc<MySqlPool>,
    /// Direct vendor items (from npc_vendor table): entry -> items
    vendor_items: DashMap<u32, Vec<VendorItem>>,
    /// Template vendor items (from npc_vendor_template table): template_id -> items
    vendor_template_items: DashMap<u32, Vec<VendorItem>>,
    /// Extended costs (from DBC): id -> cost
    extended_costs: DashMap<u32, ItemExtendedCost>,
    /// Runtime stock state per creature GUID
    vendor_stock: DashMap<ObjectGuid, Vec<VendorItemCount>>,
    /// Vendor template ID per creature entry
    creature_vendor_templates: DashMap<u32, u32>,
}

impl VendorManager {
    /// Create a new vendor manager with database pool
    pub fn new(world_db: Arc<MySqlPool>) -> Self {
        Self {
            world_db,
            vendor_items: DashMap::new(),
            vendor_template_items: DashMap::new(),
            extended_costs: DashMap::new(),
            vendor_stock: DashMap::new(),
            creature_vendor_templates: DashMap::new(),
        }
    }

    /// Load all vendor data from the database
    pub async fn load(&self) -> Result<()> {
        let repo = VendorRepository::new(Arc::clone(&self.world_db));
        let data = repo.load_all().await?;

        // Load direct vendor items
        for row in &data.vendor_items {
            let item = VendorItem {
                item_entry: row.item_entry,
                max_count: row.max_count,
                incr_time: row.incr_time,
                condition_id: row.condition_id as u32,
                itemflags: row.itemflags as u32,
            };
            self.add_vendor_item(row.entry, item);
        }

        // Load template vendor items
        for row in &data.template_items {
            let item = VendorItem {
                item_entry: row.item_entry,
                max_count: row.max_count,
                incr_time: row.incr_time,
                condition_id: row.condition_id as u32,
                itemflags: row.itemflags as u32,
            };
            self.add_template_item(row.entry, item);
        }

        // Note: creature vendor template IDs are loaded from creature_template
        // and should be registered via register_creature_vendor_template()

        info!(
            "VendorManager loaded: {} vendor entries, {} template entries",
            self.vendor_items.len(),
            self.vendor_template_items.len()
        );

        Ok(())
    }

    /// Get vendor items (combines direct items + template items)
    pub fn get_vendor_items(&self, entry: u32) -> Vec<VendorItem> {
        let mut items = Vec::new();

        // Add direct items from npc_vendor
        if let Some(direct) = self.vendor_items.get(&entry) {
            items.extend(direct.iter().cloned());
        }

        // Add template items if vendor_template_id > 0
        if let Some(template_id) = self.creature_vendor_templates.get(&entry) {
            if let Some(template) = self.vendor_template_items.get(&*template_id) {
                items.extend(template.iter().cloned());
            }
        }

        items
    }

    /// Check if a creature is a vendor (has items)
    pub fn is_vendor(&self, entry: u32) -> bool {
        self.vendor_items.contains_key(&entry)
            || self.creature_vendor_templates.contains_key(&entry)
    }

    /// Get extended cost by ID
    pub fn get_extended_cost(&self, id: u32) -> Option<ItemExtendedCost> {
        self.extended_costs.get(&id).map(|c| c.clone())
    }

    /// Get current stock for a vendor
    pub fn get_stock(&self, vendor_guid: ObjectGuid) -> Vec<VendorItemCount> {
        self.vendor_stock
            .get(&vendor_guid)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Initialize stock for a vendor (call when vendor spawns or first accessed)
    pub fn initialize_stock(&self, vendor_guid: ObjectGuid, items: &[VendorItem]) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let stock: Vec<_> = items
            .iter()
            .filter(|item| item.max_count > 0) // Only track limited stock
            .map(|item| VendorItemCount {
                item_entry: item.item_entry,
                count: item.max_count as u32,
                last_increment: now,
                restock_delay: item.incr_time,
            })
            .collect();

        if !stock.is_empty() {
            self.vendor_stock.insert(vendor_guid, stock);
        }
    }

    /// Update stock (called periodically)
    pub fn update_stock(&self, _diff_ms: u32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for mut stock_ref in self.vendor_stock.iter_mut() {
            for item_stock in stock_ref.value_mut().iter_mut() {
                // Check if stock is below max and restock timer has elapsed
                let max_count = self.get_item_max_count(item_stock.item_entry);
                if item_stock.count < max_count {
                    let elapsed = now.saturating_sub(item_stock.last_increment);
                    if elapsed >= item_stock.restock_delay as u64 {
                        item_stock.count += 1;
                        item_stock.last_increment = now;
                    }
                }
            }
        }
    }

    /// Reduce stock after purchase
    pub fn reduce_stock(&self, vendor_guid: ObjectGuid, item_entry: u32) -> bool {
        if let Some(mut stock_ref) = self.vendor_stock.get_mut(&vendor_guid) {
            if let Some(item) = stock_ref.iter_mut().find(|s| s.item_entry == item_entry) {
                if item.count > 0 {
                    item.count -= 1;
                    return true;
                }
            }
        }
        false
    }

    /// Get current stock count for an item
    pub fn get_item_stock(&self, vendor_guid: ObjectGuid, item_entry: u32) -> Option<u32> {
        self.vendor_stock.get(&vendor_guid).and_then(|stock| {
            stock
                .iter()
                .find(|s| s.item_entry == item_entry)
                .map(|s| s.count)
        })
    }

    /// Helper to get max count for an item (for restocking)
    fn get_item_max_count(&self, item_entry: u32) -> u32 {
        // Search through all vendor items to find max count
        // This is a simplified version - in practice, you'd want to track per-vendor
        for entry in self.vendor_items.iter() {
            for item in entry.value().iter() {
                if item.item_entry == item_entry {
                    return item.max_count as u32;
                }
            }
        }
        0
    }

    /// Add a vendor item
    fn add_vendor_item(&self, entry: u32, item: VendorItem) {
        self.vendor_items
            .entry(entry)
            .or_insert_with(Vec::new)
            .push(item);
    }

    /// Add a template item
    fn add_template_item(&self, template_id: u32, item: VendorItem) {
        self.vendor_template_items
            .entry(template_id)
            .or_insert_with(Vec::new)
            .push(item);
    }

    /// Register a creature's vendor template ID (called when creature templates are loaded)
    pub fn register_creature_vendor_template(&self, creature_entry: u32, vendor_template_id: u32) {
        if vendor_template_id > 0 {
            self.creature_vendor_templates
                .insert(creature_entry, vendor_template_id);
        }
    }

    /// Check if a creature has a vendor template
    pub fn has_vendor_template(&self, creature_entry: u32) -> bool {
        self.creature_vendor_templates.contains_key(&creature_entry)
    }
}
