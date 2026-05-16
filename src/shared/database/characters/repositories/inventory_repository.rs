use super::super::models::character::CharacterInventoryRow;
use super::super::models::item::*;
use super::inventory_repository_trait::{
    InventoryRepositoryTrait, InventorySlotRow, StackableSlotInfo,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{MySqlPool, Row};
use std::sync::Arc;

pub struct InventoryRepository {
    pool: Arc<MySqlPool>,
}

impl InventoryRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Load all inventory slots for a player
    pub async fn load_player_inventory(&self, player_guid: u32) -> Result<Vec<InventorySlotRow>> {
        let rows = sqlx::query_as::<_, CharacterInventoryRow>(
            r#"SELECT guid, bag, slot, item_guid, item_id
               FROM character_inventory
               WHERE guid = ?"#,
        )
        .bind(player_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load player inventory")?;

        Ok(rows
            .into_iter()
            .map(|r| InventorySlotRow {
                guid: r.guid,
                bag: r.bag as u8,
                slot: r.slot,
                item_guid: r.item_guid,
            })
            .collect())
    }

    /// Load equipment slots (0-18) for a single character
    /// Used for character enumeration (character select screen)
    pub async fn load_equipment_for_character(
        &self,
        character_guid: u32,
    ) -> Result<Vec<(u8, u32)>> {
        let rows = sqlx::query(
            r#"SELECT slot, item_id
               FROM character_inventory
               WHERE guid = ? AND slot >= 0 AND slot < 19
               ORDER BY slot"#,
        )
        .bind(character_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load equipment for character")?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let slot: u8 = row.try_get("slot").ok()?;
                let item_id: u32 = row.try_get("item_id").ok()?;
                Some((slot, item_id))
            })
            .collect())
    }

    /// Find stackable slots for an item
    pub async fn find_stackable_slots(
        &self,
        player_guid: u32,
        item_id: u32,
        max_stack: u32,
    ) -> Result<Vec<StackableSlotInfo>> {
        let rows = sqlx::query_as::<_, (u32, u32, u8, u32)>(
            r#"SELECT ci.item_guid, ci.bag, ci.slot, ii.count
               FROM character_inventory ci
               JOIN item_instance ii ON ci.item_guid = ii.guid
               WHERE ci.guid = ? AND ii.item_id = ? AND ii.count < ?
               ORDER BY ii.count DESC"#,
        )
        .bind(player_guid)
        .bind(item_id)
        .bind(max_stack)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to find stackable slots")?;

        Ok(rows
            .into_iter()
            .map(|(item_guid, bag, slot, count)| StackableSlotInfo {
                item_guid,
                bag: bag as u8,
                slot,
                current_count: count,
            })
            .collect())
    }

    /// Get player's current money
    pub async fn get_player_money(&self, player_guid: u32) -> Result<u32> {
        let money: u32 = sqlx::query_scalar("SELECT money FROM characters WHERE guid = ?")
            .bind(player_guid)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to get player money")?;

        Ok(money)
    }

    /// Find item instance by GUID
    pub async fn find_item(&self, item_guid: u32) -> Result<Option<ItemInstanceRow>> {
        sqlx::query_as::<_, ItemInstanceRow>(
            r#"SELECT guid, item_id, owner_guid, creator_guid, gift_creator_guid,
                      count, duration, charges, flags, enchantments, random_property_id,
                      durability, text, generated_loot
               FROM item_instance
               WHERE guid = ?"#,
        )
        .bind(item_guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find item by GUID")
    }

    /// Find all items owned by a player
    pub async fn find_items_by_owner(&self, player_guid: u32) -> Result<Vec<ItemInstanceRow>> {
        sqlx::query_as::<_, ItemInstanceRow>(
            r#"SELECT guid, item_id, owner_guid, creator_guid, gift_creator_guid,
                      count, duration, charges, flags, enchantments, random_property_id,
                      durability, text, generated_loot
               FROM item_instance
               WHERE owner_guid = ?"#,
        )
        .bind(player_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to find items by owner")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new item instance and add to inventory
    pub async fn create_item(&self, item: &ItemInstanceRow, slot: &InventorySlotRow) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Insert item instance
        sqlx::query(
            r#"INSERT INTO item_instance
               (guid, item_id, owner_guid, creator_guid, gift_creator_guid, count, duration,
                charges, flags, enchantments, random_property_id, durability, text, generated_loot)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(item.guid)
        .bind(item.item_id)
        .bind(item.owner_guid)
        .bind(item.creator_guid)
        .bind(item.gift_creator_guid)
        .bind(item.count)
        .bind(item.duration)
        .bind(&item.charges)
        .bind(item.flags)
        .bind(&item.enchantments)
        .bind(item.random_property_id)
        .bind(item.durability)
        .bind(item.text)
        .bind(item.generated_loot)
        .execute(&mut *tx)
        .await
        .context("Failed to create item instance")?;

        // Insert inventory slot
        sqlx::query(
            r#"INSERT INTO character_inventory (guid, bag, slot, item_guid, item_id)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(slot.guid)
        .bind(slot.bag as u32)
        .bind(slot.slot)
        .bind(item.guid)
        .bind(item.item_id)
        .execute(&mut *tx)
        .await
        .context("Failed to insert inventory slot")?;

        tx.commit().await.context("Failed to commit create item")?;
        Ok(())
    }

    /// Update item instance count
    pub async fn update_item_count(&self, item_guid: u32, count: u32) -> Result<()> {
        sqlx::query("UPDATE item_instance SET count = ? WHERE guid = ?")
            .bind(count)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item count")?;

        Ok(())
    }

    /// Move item to a different slot
    pub async fn move_item(
        &self,
        player_guid: u32,
        item_guid: u32,
        bag: u8,
        slot: u8,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE character_inventory SET bag = ?, slot = ? WHERE guid = ? AND item_guid = ?",
        )
        .bind(bag as u32)
        .bind(slot)
        .bind(player_guid)
        .bind(item_guid)
        .execute(&*self.pool)
        .await
        .context("Failed to move item")?;

        Ok(())
    }

    /// Swap two items (atomic operation)
    pub async fn swap_items(
        &self,
        player_guid: u32,
        item1_guid: u32,
        bag1: u8,
        slot1: u8,
        item2_guid: Option<u32>,
        bag2: u8,
        slot2: u8,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Move item1 to slot2
        sqlx::query(
            "UPDATE character_inventory SET bag = ?, slot = ? WHERE guid = ? AND item_guid = ?",
        )
        .bind(bag2 as u32)
        .bind(slot2)
        .bind(player_guid)
        .bind(item1_guid)
        .execute(&mut *tx)
        .await
        .context("Failed to move item1")?;

        // If item2 exists, move it to slot1
        if let Some(item2) = item2_guid {
            sqlx::query(
                "UPDATE character_inventory SET bag = ?, slot = ? WHERE guid = ? AND item_guid = ?",
            )
            .bind(bag1 as u32)
            .bind(slot1)
            .bind(player_guid)
            .bind(item2)
            .execute(&mut *tx)
            .await
            .context("Failed to move item2")?;
        }

        tx.commit().await.context("Failed to commit swap")?;
        Ok(())
    }

    /// Delete item instance and remove from inventory
    pub async fn delete_item(&self, item_guid: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete from inventory
        sqlx::query("DELETE FROM character_inventory WHERE item_guid = ?")
            .bind(item_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete from inventory")?;

        // Delete loot first (if any)
        sqlx::query("DELETE FROM item_loot WHERE guid = ?")
            .bind(item_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete item loot")?;

        // Delete item instance
        sqlx::query("DELETE FROM item_instance WHERE guid = ?")
            .bind(item_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete item instance")?;

        tx.commit().await.context("Failed to commit delete")?;
        Ok(())
    }

    /// Remove item from inventory slot (but don't delete item instance)
    pub async fn remove_from_slot(&self, player_guid: u32, bag: u8, slot: u8) -> Result<()> {
        sqlx::query("DELETE FROM character_inventory WHERE guid = ? AND bag = ? AND slot = ?")
            .bind(player_guid)
            .bind(bag as u32)
            .bind(slot)
            .execute(&*self.pool)
            .await
            .context("Failed to remove from slot")?;

        Ok(())
    }

    /// Add existing item to inventory slot
    pub async fn add_to_slot(
        &self,
        player_guid: u32,
        item_guid: u32,
        bag: u8,
        slot: u8,
    ) -> Result<()> {
        // Get item_id from item_instance
        let item_id: u32 = sqlx::query_scalar("SELECT item_id FROM item_instance WHERE guid = ?")
            .bind(item_guid)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to get item_id")?;

        sqlx::query(
            r#"INSERT INTO character_inventory (guid, bag, slot, item_guid, item_id)
               VALUES (?, ?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE item_guid = ?, item_id = ?"#,
        )
        .bind(player_guid)
        .bind(bag as u32)
        .bind(slot)
        .bind(item_guid)
        .bind(item_id)
        .bind(item_guid)
        .bind(item_id)
        .execute(&*self.pool)
        .await
        .context("Failed to add to slot")?;

        Ok(())
    }

    /// Update player money
    pub async fn update_player_money(&self, player_guid: u32, money: u32) -> Result<()> {
        sqlx::query("UPDATE characters SET money = ? WHERE guid = ?")
            .bind(money)
            .bind(player_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update player money")?;

        Ok(())
    }

    /// Update item owner
    pub async fn update_item_owner(&self, item_guid: u32, new_owner_guid: u32) -> Result<()> {
        sqlx::query("UPDATE item_instance SET owner_guid = ? WHERE guid = ?")
            .bind(new_owner_guid)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item owner")?;

        Ok(())
    }

    /// Update item durability
    pub async fn update_item_durability(&self, item_guid: u32, durability: u16) -> Result<()> {
        sqlx::query("UPDATE item_instance SET durability = ? WHERE guid = ?")
            .bind(durability)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item durability")?;

        Ok(())
    }

    /// Update item enchantments
    pub async fn update_item_enchantments(&self, item_guid: u32, enchantments: &str) -> Result<()> {
        sqlx::query("UPDATE item_instance SET enchantments = ? WHERE guid = ?")
            .bind(enchantments)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item enchantments")?;

        Ok(())
    }

    /// Update item flags
    pub async fn update_item_flags(&self, item_guid: u32, flags: u32) -> Result<()> {
        sqlx::query("UPDATE item_instance SET flags = ? WHERE guid = ?")
            .bind(flags)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item flags")?;

        Ok(())
    }

    /// Batch update inventory slot positions (deferred persistence)
    pub async fn batch_move_items(&self, player_guid: u32, moves: &[(u32, u8, u8)]) -> Result<()> {
        if moves.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for &(item_guid, bag, slot) in moves {
            sqlx::query(
                "UPDATE character_inventory SET bag = ?, slot = ? WHERE guid = ? AND item_guid = ?",
            )
            .bind(bag as u32)
            .bind(slot)
            .bind(player_guid)
            .bind(item_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to batch move item")?;
        }

        tx.commit().await.context("Failed to commit batch move")?;
        Ok(())
    }

    /// Batch update item counts
    pub async fn batch_update_counts(&self, updates: &[(u32, u32)]) -> Result<()> {
        if updates.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for (item_guid, count) in updates {
            sqlx::query("UPDATE item_instance SET count = ? WHERE guid = ?")
                .bind(count)
                .bind(item_guid)
                .execute(&mut *tx)
                .await
                .context("Failed to update item count in batch")?;
        }

        tx.commit().await.context("Failed to commit batch update")?;
        Ok(())
    }

    /// Update item spell charges
    pub async fn update_item_charges(&self, item_guid: u32, charges: &str) -> Result<()> {
        sqlx::query("UPDATE item_instance SET charges = ? WHERE guid = ?")
            .bind(charges)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item charges")?;

        Ok(())
    }

    /// Batch update item durability (for death penalty)
    pub async fn batch_update_durability(&self, updates: &[(u32, u16)]) -> Result<()> {
        if updates.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for (item_guid, durability) in updates {
            sqlx::query("UPDATE item_instance SET durability = ? WHERE guid = ?")
                .bind(durability)
                .bind(item_guid)
                .execute(&mut *tx)
                .await
                .context("Failed to update item durability in batch")?;
        }

        tx.commit()
            .await
            .context("Failed to commit batch durability update")?;
        Ok(())
    }

    /// Get next available item GUID
    pub async fn get_next_item_guid(&self) -> Result<u32> {
        let max_guid: Option<u32> = sqlx::query_scalar("SELECT MAX(guid) FROM item_instance")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to get max item GUID")?;

        Ok(max_guid.unwrap_or(0) + 1)
    }

    /// Reserve item GUIDs (for creating multiple items)
    pub async fn reserve_item_guids(&self, count: u32) -> Result<u32> {
        let start_guid = self.get_next_item_guid().await?;
        // In a production system, you'd want to use a sequence or atomic counter
        // For now, just return the starting GUID and trust caller to use correctly
        Ok(start_guid)
    }
}

// ========== TRAIT IMPLEMENTATION ==========

#[async_trait]
impl InventoryRepositoryTrait for InventoryRepository {
    async fn load_player_inventory(&self, player_guid: u32) -> Result<Vec<InventorySlotRow>> {
        self.load_player_inventory(player_guid).await
    }

    async fn find_stackable_slots(
        &self,
        player_guid: u32,
        item_id: u32,
        max_stack: u32,
    ) -> Result<Vec<StackableSlotInfo>> {
        self.find_stackable_slots(player_guid, item_id, max_stack)
            .await
    }

    async fn get_player_money(&self, player_guid: u32) -> Result<u32> {
        self.get_player_money(player_guid).await
    }

    async fn find_item(&self, item_guid: u32) -> Result<Option<ItemInstanceRow>> {
        self.find_item(item_guid).await
    }

    async fn find_items_by_owner(&self, player_guid: u32) -> Result<Vec<ItemInstanceRow>> {
        self.find_items_by_owner(player_guid).await
    }

    async fn create_item(&self, item: &ItemInstanceRow, slot: &InventorySlotRow) -> Result<()> {
        self.create_item(item, slot).await
    }

    async fn update_item_count(&self, item_guid: u32, count: u32) -> Result<()> {
        self.update_item_count(item_guid, count).await
    }

    async fn move_item(&self, player_guid: u32, item_guid: u32, bag: u8, slot: u8) -> Result<()> {
        self.move_item(player_guid, item_guid, bag, slot).await
    }

    async fn swap_items(
        &self,
        player_guid: u32,
        item1_guid: u32,
        bag1: u8,
        slot1: u8,
        item2_guid: Option<u32>,
        bag2: u8,
        slot2: u8,
    ) -> Result<()> {
        self.swap_items(
            player_guid,
            item1_guid,
            bag1,
            slot1,
            item2_guid,
            bag2,
            slot2,
        )
        .await
    }

    async fn delete_item(&self, item_guid: u32) -> Result<()> {
        self.delete_item(item_guid).await
    }

    async fn remove_from_slot(&self, player_guid: u32, bag: u8, slot: u8) -> Result<()> {
        self.remove_from_slot(player_guid, bag, slot).await
    }

    async fn add_to_slot(&self, player_guid: u32, item_guid: u32, bag: u8, slot: u8) -> Result<()> {
        self.add_to_slot(player_guid, item_guid, bag, slot).await
    }

    async fn update_player_money(&self, player_guid: u32, money: u32) -> Result<()> {
        self.update_player_money(player_guid, money).await
    }

    async fn update_item_owner(&self, item_guid: u32, new_owner_guid: u32) -> Result<()> {
        self.update_item_owner(item_guid, new_owner_guid).await
    }

    async fn update_item_durability(&self, item_guid: u32, durability: u16) -> Result<()> {
        self.update_item_durability(item_guid, durability).await
    }

    async fn update_item_enchantments(&self, item_guid: u32, enchantments: &str) -> Result<()> {
        self.update_item_enchantments(item_guid, enchantments).await
    }

    async fn update_item_flags(&self, item_guid: u32, flags: u32) -> Result<()> {
        self.update_item_flags(item_guid, flags).await
    }

    async fn update_item_charges(&self, item_guid: u32, charges: &str) -> Result<()> {
        self.update_item_charges(item_guid, charges).await
    }

    async fn batch_move_items(&self, player_guid: u32, moves: &[(u32, u8, u8)]) -> Result<()> {
        self.batch_move_items(player_guid, moves).await
    }

    async fn batch_update_counts(&self, updates: &[(u32, u32)]) -> Result<()> {
        self.batch_update_counts(updates).await
    }

    async fn batch_update_durability(&self, updates: &[(u32, u16)]) -> Result<()> {
        self.batch_update_durability(updates).await
    }

    async fn get_next_item_guid(&self) -> Result<u32> {
        self.get_next_item_guid().await
    }

    async fn reserve_item_guids(&self, count: u32) -> Result<u32> {
        self.reserve_item_guids(count).await
    }
}
