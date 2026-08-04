use crate::database::characters::models::item::*;
use anyhow::Result;
use async_trait::async_trait;

/// Inventory slot row from character_inventory table
#[derive(Debug, Clone)]
pub struct InventorySlotRow {
    pub guid: u32,      // Player GUID (owner)
    pub bag: u8,        // Bag slot (255 for main inventory)
    pub slot: u8,       // Slot within bag
    pub item_guid: u32, // Item instance GUID
}

/// Trait abstraction for inventory repository operations.
/// Enables dependency injection and mocking for tests.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait InventoryRepositoryTrait: Send + Sync {
    // ========== QUERY METHODS (Read Operations) ==========

    /// Load all inventory slots for a player
    /// Returns list of (bag, slot, item_guid) entries
    async fn load_player_inventory(&self, player_guid: u32) -> Result<Vec<InventorySlotRow>>;

    /// Get player's current money
    async fn get_player_money(&self, player_guid: u32) -> Result<u32>;

    /// Find item instance by GUID
    async fn find_item(&self, item_guid: u32) -> Result<Option<ItemInstanceRow>>;

    /// Find all items owned by a player
    async fn find_items_by_owner(&self, player_guid: u32) -> Result<Vec<ItemInstanceRow>>;

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new item instance and add to inventory
    async fn create_item(&self, item: &ItemInstanceRow, slot: &InventorySlotRow) -> Result<()>;

    /// Update item instance count
    async fn update_item_count(&self, item_guid: u32, count: u32) -> Result<()>;

    /// Move item to a different slot
    async fn move_item(&self, player_guid: u32, item_guid: u32, bag: u8, slot: u8) -> Result<()>;

    /// Swap two items (atomic operation)
    /// If item2_guid is None, it's a simple move (item1 to slot2, slot1 becomes empty)
    async fn swap_items(
        &self,
        player_guid: u32,
        item1_guid: u32,
        bag1: u8,
        slot1: u8,
        item2_guid: Option<u32>,
        bag2: u8,
        slot2: u8,
    ) -> Result<()>;

    /// Delete item instance and remove from inventory
    async fn delete_item(&self, item_guid: u32) -> Result<()>;

    /// Delete item from all related tables (item_instance, character_inventory, auction, mail_items, character_gifts)
    async fn delete_item_all(&self, item_guid: u32) -> Result<()>;

    /// Remove item from inventory slot (but don't delete item instance)
    async fn remove_from_slot(&self, player_guid: u32, bag: u8, slot: u8) -> Result<()>;

    /// Add existing item to inventory slot
    async fn add_to_slot(&self, player_guid: u32, item_guid: u32, bag: u8, slot: u8) -> Result<()>;

    /// Update player money
    async fn update_player_money(&self, player_guid: u32, money: u32) -> Result<()>;

    /// Update item owner
    async fn update_item_owner(&self, item_guid: u32, new_owner_guid: u32) -> Result<()>;

    /// Update item durability
    async fn update_item_durability(&self, item_guid: u32, durability: u16) -> Result<()>;

    /// Update item enchantments
    async fn update_item_enchantments(&self, item_guid: u32, enchantments: &str) -> Result<()>;

    /// Update item flags
    async fn update_item_flags(&self, item_guid: u32, flags: u32) -> Result<()>;

    /// Update item spell charges
    async fn update_item_charges(&self, item_guid: u32, charges: &str) -> Result<()>;

    /// Batch update inventory slot positions (deferred persistence)
    /// moves: &[(item_guid, bag, slot)]
    async fn batch_move_items(&self, player_guid: u32, moves: &[(u32, u8, u8)]) -> Result<()>;

    /// Batch update item counts (for multi-stack operations)
    async fn batch_update_counts(&self, updates: &[(u32, u32)]) -> Result<()>;

    /// Update item duration
    async fn update_item_duration(&self, item_guid: u32, duration: u32) -> Result<()>;

    /// Batch update item durability (for death penalty)
    async fn batch_update_durability(&self, updates: &[(u32, u16)]) -> Result<()>;

    /// Get next available item GUID
    async fn get_next_item_guid(&self) -> Result<u32>;

    /// Reserve item GUIDs (for creating multiple items)
    async fn reserve_item_guids(&self, count: u32) -> Result<u32>;
}
