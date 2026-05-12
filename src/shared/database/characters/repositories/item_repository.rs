use super::super::models::item::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct ItemRepository {
    pool: Arc<MySqlPool>,
}

impl ItemRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Find an item instance by GUID.
    pub async fn find_by_guid(&self, guid: u32) -> Result<Option<ItemInstanceRow>> {
        sqlx::query_as::<_, ItemInstanceRow>(
            r#"SELECT guid, item_id, owner_guid, creator_guid, gift_creator_guid,
                      count, duration, charges, flags, enchantments, random_property_id,
                      durability, text, generated_loot
               FROM item_instance
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch item instance by GUID")
    }

    /// Find all item instances owned by a player.
    pub async fn find_by_owner(&self, owner_guid: u32) -> Result<Vec<ItemInstanceRow>> {
        sqlx::query_as::<_, ItemInstanceRow>(
            r#"SELECT guid, item_id, owner_guid, creator_guid, gift_creator_guid,
                      count, duration, charges, flags, enchantments, random_property_id,
                      durability, text, generated_loot
               FROM item_instance
               WHERE owner_guid = ?"#,
        )
        .bind(owner_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch item instances by owner")
    }

    /// Get all distinct item IDs currently in item_instance (for template loading).
    pub async fn find_distinct_item_ids(&self) -> Result<Vec<u32>> {
        sqlx::query_scalar::<_, u32>("SELECT DISTINCT item_id FROM item_instance")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch distinct item IDs")
    }

    /// Find loot contents for a container item.
    pub async fn find_loot(&self, guid: u32) -> Result<Vec<ItemLootRow>> {
        sqlx::query_as::<_, ItemLootRow>(
            r#"SELECT guid, owner_guid, item_id, amount, property
               FROM item_loot
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch item loot")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new item instance.
    pub async fn create(&self, item: &ItemInstanceRow) -> Result<()> {
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
        .execute(&*self.pool)
        .await
        .context("Failed to create item instance")?;

        Ok(())
    }

    /// Update item instance count.
    pub async fn update_count(&self, guid: u32, count: u32) -> Result<()> {
        sqlx::query("UPDATE item_instance SET count = ? WHERE guid = ?")
            .bind(count)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item count")?;

        Ok(())
    }

    /// Update item durability.
    pub async fn update_durability(&self, guid: u32, durability: u16) -> Result<()> {
        sqlx::query("UPDATE item_instance SET durability = ? WHERE guid = ?")
            .bind(durability)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item durability")?;

        Ok(())
    }

    /// Update item owner.
    pub async fn update_owner(&self, guid: u32, owner_guid: u32) -> Result<()> {
        sqlx::query("UPDATE item_instance SET owner_guid = ? WHERE guid = ?")
            .bind(owner_guid)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item owner")?;

        Ok(())
    }

    /// Update item enchantments.
    pub async fn update_enchantments(&self, guid: u32, enchantments: &str) -> Result<()> {
        sqlx::query("UPDATE item_instance SET enchantments = ? WHERE guid = ?")
            .bind(enchantments)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update item enchantments")?;

        Ok(())
    }

    /// Update full item instance.
    pub async fn update(&self, item: &ItemInstanceRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE item_instance
               SET item_id = ?, owner_guid = ?, creator_guid = ?, gift_creator_guid = ?,
                   count = ?, duration = ?, charges = ?, flags = ?, enchantments = ?,
                   random_property_id = ?, durability = ?, text = ?, generated_loot = ?
               WHERE guid = ?"#,
        )
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
        .bind(item.guid)
        .execute(&*self.pool)
        .await
        .context("Failed to update item instance")?;

        Ok(())
    }

    /// Add loot to a container item.
    pub async fn add_loot(&self, loot: &ItemLootRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO item_loot (guid, owner_guid, item_id, amount, property)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(loot.guid)
        .bind(loot.owner_guid)
        .bind(loot.item_id)
        .bind(loot.amount)
        .bind(loot.property)
        .execute(&*self.pool)
        .await
        .context("Failed to add item loot")?;

        Ok(())
    }

    /// Delete all loot for a container item.
    pub async fn delete_loot(&self, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM item_loot WHERE guid = ?")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete item loot")?;

        Ok(())
    }

    // ========== DELETE OPERATIONS ==========

    /// Delete an item instance.
    pub async fn delete(&self, guid: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete loot first (if any)
        sqlx::query("DELETE FROM item_loot WHERE guid = ?")
            .bind(guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete item loot")?;

        // Delete item instance
        sqlx::query("DELETE FROM item_instance WHERE guid = ?")
            .bind(guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete item instance")?;

        tx.commit()
            .await
            .context("Failed to commit item deletion")?;
        Ok(())
    }

    /// Delete all items owned by a player (used when deleting character).
    pub async fn delete_all_for_owner(&self, owner_guid: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Get all item guids for the owner
        let item_guids: Vec<u32> =
            sqlx::query_scalar("SELECT guid FROM item_instance WHERE owner_guid = ?")
                .bind(owner_guid)
                .fetch_all(&mut *tx)
                .await
                .context("Failed to fetch item guids for owner")?;

        // Delete loot for all items
        for guid in &item_guids {
            sqlx::query("DELETE FROM item_loot WHERE guid = ?")
                .bind(guid)
                .execute(&mut *tx)
                .await
                .context("Failed to delete item loot")?;
        }

        // Delete all item instances
        sqlx::query("DELETE FROM item_instance WHERE owner_guid = ?")
            .bind(owner_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to delete item instances for owner")?;

        tx.commit()
            .await
            .context("Failed to commit item deletion for owner")?;
        Ok(())
    }
}
