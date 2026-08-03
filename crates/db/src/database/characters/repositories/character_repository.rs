use super::super::models::character::CharacterRow;
use super::super::{PgCharacterRepository, PgCharacterRow};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDeleteMode {
    Hard,
    Soft,
}

/// Compatibility facade for callers that still use unsigned protocol-sized identifiers.
/// PostgreSQL persistence remains signed internally; every row returned from it is range checked.
pub struct CharacterRepository {
    pool: Arc<PgPool>,
}

impl CharacterRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgCharacterRepository {
        PgCharacterRepository::new(Arc::clone(&self.pool))
    }

    pub async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterRow>> {
        self.pg()
            .find_by_guid(guid.into())
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
    pub async fn find_by_account(&self, account: u32) -> Result<Vec<CharacterRow>> {
        self.pg()
            .find_by_account(account.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    pub async fn find_by_name(&self, name: &str) -> Result<Option<CharacterRow>> {
        sqlx::query_as::<_, PgCharacterRow>("SELECT * FROM characters.characters WHERE name = $1")
            .bind(name)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL character by name")
            .and_then(|row| row.map(TryInto::try_into).transpose())
    }
    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        self.pg().exists_by_name(name).await
    }
    pub async fn find_empty_inventory_slot(&self, guid: u32) -> Result<Option<u8>> {
        let slot: Option<i16> = sqlx::query_scalar("SELECT slot FROM generate_series(23, 38) AS slot WHERE NOT EXISTS (SELECT 1 FROM characters.character_inventory WHERE guid = $1 AND bag = 0 AND character_inventory.slot = slot) ORDER BY slot LIMIT 1")
            .bind(i64::from(guid)).fetch_optional(&*self.pool).await.context("Failed to find PostgreSQL inventory slot")?;
        slot.map(|value| value.try_into().map_err(Into::into))
            .transpose()
    }
    pub async fn add_item_to_inventory(
        &self,
        guid: u32,
        bag: u32,
        slot: u8,
        item_guid: u32,
        item_id: u32,
    ) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_inventory (guid, bag, slot, item_guid, item_id) VALUES ($1, $2, $3, $4, $5)")
            .bind(i64::from(guid)).bind(i64::from(bag)).bind(i16::from(slot)).bind(i64::from(item_guid)).bind(i64::from(item_id)).execute(&*self.pool).await.context("Failed to add PostgreSQL item to inventory")?;
        Ok(())
    }
    pub async fn create_item_instance(
        &self,
        guid: u32,
        item_id: u32,
        owner_guid: u32,
        creator_guid: u32,
        count: u32,
    ) -> Result<()> {
        sqlx::query("INSERT INTO characters.item_instance (guid, item_id, owner_guid, creator_guid, count, enchantments) VALUES ($1, $2, $3, $4, $5, '')")
            .bind(i64::from(guid)).bind(i64::from(item_id)).bind(i64::from(owner_guid)).bind(i64::from(creator_guid)).bind(i64::from(count)).execute(&*self.pool).await.context("Failed to create PostgreSQL item instance")?;
        Ok(())
    }
}

#[async_trait]
pub trait CharacterRepositoryTrait: Send + Sync {
    async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterRow>>;
    async fn find_by_account(&self, account_id: u32) -> Result<Vec<CharacterRow>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<CharacterRow>>;
    async fn exists_by_name(&self, name: &str) -> Result<bool>;
    async fn find_empty_inventory_slot(&self, guid: u32) -> Result<Option<u8>>;
    async fn add_item_to_inventory(
        &self,
        guid: u32,
        bag: u32,
        slot: u8,
        item_guid: u32,
        item_id: u32,
    ) -> Result<()>;
    async fn create_item_instance(
        &self,
        guid: u32,
        item_id: u32,
        owner_guid: u32,
        creator_guid: u32,
        count: u32,
    ) -> Result<()>;
}

#[async_trait]
impl CharacterRepositoryTrait for CharacterRepository {
    async fn find_by_guid(&self, guid: u32) -> Result<Option<CharacterRow>> {
        self.find_by_guid(guid).await
    }
    async fn find_by_account(&self, account: u32) -> Result<Vec<CharacterRow>> {
        self.find_by_account(account).await
    }
    async fn find_by_name(&self, name: &str) -> Result<Option<CharacterRow>> {
        self.find_by_name(name).await
    }
    async fn exists_by_name(&self, name: &str) -> Result<bool> {
        self.exists_by_name(name).await
    }
    async fn find_empty_inventory_slot(&self, guid: u32) -> Result<Option<u8>> {
        self.find_empty_inventory_slot(guid).await
    }
    async fn add_item_to_inventory(
        &self,
        guid: u32,
        bag: u32,
        slot: u8,
        item_guid: u32,
        item_id: u32,
    ) -> Result<()> {
        self.add_item_to_inventory(guid, bag, slot, item_guid, item_id)
            .await
    }
    async fn create_item_instance(
        &self,
        guid: u32,
        item_id: u32,
        owner_guid: u32,
        creator_guid: u32,
        count: u32,
    ) -> Result<()> {
        self.create_item_instance(guid, item_id, owner_guid, creator_guid, count)
            .await
    }
}
