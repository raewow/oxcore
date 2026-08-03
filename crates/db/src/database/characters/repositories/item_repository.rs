use super::super::postgres::{PgItemInstanceRow, PgItemLootRow};
use super::item_repository_trait::ItemRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct ItemRepository {
    pool: Arc<PgPool>,
}

impl ItemRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Find an item instance by GUID.
    pub async fn find_by_guid(&self, guid: u32) -> Result<Option<PgItemInstanceRow>> {
        sqlx::query_as::<_, PgItemInstanceRow>(
            r#"SELECT guid, item_id, owner_guid, creator_guid, gift_creator_guid,
                       count, duration, charges, flags, enchantments, random_property_id,
                       durability, text, generated_loot
               FROM characters.item_instance
               WHERE guid = $1"#,
        )
        .bind(i64::from(guid))
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch item instance by GUID")
    }

    /// Find all item instances owned by a player.
    pub async fn find_by_owner(&self, owner_guid: u32) -> Result<Vec<PgItemInstanceRow>> {
        sqlx::query_as::<_, PgItemInstanceRow>(
            r#"SELECT guid, item_id, owner_guid, creator_guid, gift_creator_guid,
                      count, duration, charges, flags, enchantments, random_property_id,
                      durability, text, generated_loot
               FROM characters.item_instance
               WHERE owner_guid = $1"#,
        )
        .bind(i64::from(owner_guid))
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch item instances by owner")
    }

    /// Get all distinct item IDs currently in item_instance (for template loading).
    pub async fn find_distinct_item_ids(&self) -> Result<Vec<u32>> {
        let ids =
            sqlx::query_scalar::<_, i64>("SELECT DISTINCT item_id FROM characters.item_instance")
                .fetch_all(&*self.pool)
                .await
                .context("Failed to fetch distinct item IDs")?;
        ids.into_iter()
            .map(|id| {
                u32::try_from(id).context("PostgreSQL item ID exceeds the game protocol range")
            })
            .collect()
    }

    /// Find loot contents for a container item.
    pub async fn find_loot(&self, guid: u32) -> Result<Vec<PgItemLootRow>> {
        sqlx::query_as::<_, PgItemLootRow>(
            r#"SELECT guid, owner_guid, item_id, amount, property
               FROM characters.item_loot
               WHERE guid = $1"#,
        )
        .bind(i64::from(guid))
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch item loot")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new item instance.
    pub async fn create(&self, item: &PgItemInstanceRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO characters.item_instance
               (guid, item_id, owner_guid, creator_guid, gift_creator_guid, count, duration,
                charges, flags, enchantments, random_property_id, durability, text, generated_loot)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"#,
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
        sqlx::query("UPDATE characters.item_instance SET count = $1 WHERE guid = $2")
            .bind(i64::from(count))
            .bind(i64::from(guid))
            .execute(&*self.pool)
            .await
            .context("Failed to update item count")?;

        Ok(())
    }

    /// Update item durability.
    pub async fn update_durability(&self, guid: u32, durability: u16) -> Result<()> {
        sqlx::query("UPDATE characters.item_instance SET durability = $1 WHERE guid = $2")
            .bind(i32::from(durability))
            .bind(i64::from(guid))
            .execute(&*self.pool)
            .await
            .context("Failed to update item durability")?;

        Ok(())
    }

    /// Update item owner.
    pub async fn update_owner(&self, guid: u32, owner_guid: u32) -> Result<()> {
        sqlx::query("UPDATE characters.item_instance SET owner_guid = $1 WHERE guid = $2")
            .bind(i64::from(owner_guid))
            .bind(i64::from(guid))
            .execute(&*self.pool)
            .await
            .context("Failed to update item owner")?;

        Ok(())
    }

    /// Update item enchantments.
    pub async fn update_enchantments(&self, guid: u32, enchantments: &str) -> Result<()> {
        sqlx::query("UPDATE characters.item_instance SET enchantments = $1 WHERE guid = $2")
            .bind(enchantments)
            .bind(i64::from(guid))
            .execute(&*self.pool)
            .await
            .context("Failed to update item enchantments")?;

        Ok(())
    }

    /// Update full item instance.
    pub async fn update(&self, item: &PgItemInstanceRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE characters.item_instance
               SET item_id = $1, owner_guid = $2, creator_guid = $3, gift_creator_guid = $4,
                   count = $5, duration = $6, charges = $7, flags = $8, enchantments = $9,
                   random_property_id = $10, durability = $11, text = $12, generated_loot = $13
               WHERE guid = $14"#,
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
    pub async fn add_loot(&self, loot: &PgItemLootRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO characters.item_loot (guid, owner_guid, item_id, amount, property)
                VALUES ($1, $2, $3, $4, $5)"#,
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
        sqlx::query("DELETE FROM characters.item_loot WHERE guid = $1")
            .bind(i64::from(guid))
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
        sqlx::query("DELETE FROM characters.item_loot WHERE guid = $1")
            .bind(i64::from(guid))
            .execute(&mut *tx)
            .await
            .context("Failed to delete item loot")?;

        // Delete item instance
        sqlx::query("DELETE FROM characters.item_instance WHERE guid = $1")
            .bind(i64::from(guid))
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
        let item_guids: Vec<i64> =
            sqlx::query_scalar("SELECT guid FROM characters.item_instance WHERE owner_guid = $1")
                .bind(i64::from(owner_guid))
                .fetch_all(&mut *tx)
                .await
                .context("Failed to fetch item guids for owner")?;

        // Delete loot for all items
        for guid in &item_guids {
            sqlx::query("DELETE FROM characters.item_loot WHERE guid = $1")
                .bind(guid)
                .execute(&mut *tx)
                .await
                .context("Failed to delete item loot")?;
        }

        // Delete all item instances
        sqlx::query("DELETE FROM characters.item_instance WHERE owner_guid = $1")
            .bind(i64::from(owner_guid))
            .execute(&mut *tx)
            .await
            .context("Failed to delete item instances for owner")?;

        tx.commit()
            .await
            .context("Failed to commit item deletion for owner")?;
        Ok(())
    }
}

#[async_trait]
impl ItemRepositoryTrait for ItemRepository {
    async fn update_owner(&self, guid: u32, owner_guid: u32) -> Result<()> {
        Self::update_owner(self, guid, owner_guid).await
    }

    async fn delete(&self, guid: u32) -> Result<()> {
        Self::delete(self, guid).await
    }
}
