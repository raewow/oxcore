use super::super::models::mail::*;
use super::super::{PgMailItemRow, PgMailRepository, PgMailRow};
use crate::database::characters::repositories::mail_repository_trait::MailRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct MailRepository {
    pool: Arc<PgPool>,
}

impl MailRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgMailRepository {
        PgMailRepository::new(Arc::clone(&self.pool))
    }
}

#[async_trait]
impl MailRepositoryTrait for MailRepository {
    async fn find_by_id(&self, id: u32, receiver: u32) -> Result<Option<MailRow>> {
        self.pg()
            .find_by_id(id.into(), receiver.into())
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
    async fn find_by_receiver(&self, receiver: u32) -> Result<Vec<MailRow>> {
        self.pg()
            .find_by_receiver(receiver.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_mail_items(&self, id: u32) -> Result<Vec<MailItemRow>> {
        self.pg()
            .find_mail_items(id.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn find_items_by_receiver(&self, receiver: u32) -> Result<Vec<MailItemRow>> {
        self.pg()
            .find_items_by_receiver(receiver.into())
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
    async fn count_by_receiver(&self, receiver: u32) -> Result<u32> {
        self.pg()
            .count_by_receiver(receiver.into())
            .await?
            .try_into()
            .map_err(Into::into)
    }
    async fn find_item_text(&self, id: u32) -> Result<Option<ItemTextRow>> {
        self.pg()
            .find_item_text(id.into())
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
    async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>> {
        self.pg()
            .find_player_guid_by_name(name)
            .await?
            .map(|value| value.try_into().map_err(Into::into))
            .transpose()
    }
    async fn find_player_race(&self, guid: u32) -> Result<Option<u8>> {
        let race: Option<i16> =
            sqlx::query_scalar("SELECT race FROM characters.characters WHERE guid = $1")
                .bind(i64::from(guid))
                .fetch_optional(&*self.pool)
                .await
                .context("Failed to find PostgreSQL player race")?;
        race.map(|value| value.try_into().map_err(Into::into))
            .transpose()
    }
    async fn create(&self, mail: &MailRow) -> Result<u32> {
        self.pg()
            .create(&PgMailRow::from(mail))
            .await?
            .try_into()
            .map_err(Into::into)
    }
    async fn add_item(
        &self,
        mail_id: u32,
        item_guid: u32,
        item_id: u32,
        receiver_guid: u32,
    ) -> Result<()> {
        self.pg()
            .add_item(&PgMailItemRow {
                mail_id: mail_id.into(),
                item_guid: item_guid.into(),
                item_id: item_id.into(),
                receiver_guid: receiver_guid.into(),
            })
            .await
    }
    async fn update_checked(&self, id: u32, receiver: u32, checked: u8) -> Result<()> {
        self.pg()
            .update_checked(id.into(), receiver.into(), checked.into())
            .await
    }
    async fn clear_money(&self, id: u32, receiver: u32) -> Result<()> {
        self.pg().clear_money(id.into(), receiver.into()).await
    }
    async fn remove_item(&self, mail_id: u32, item_guid: u32) -> Result<()> {
        self.pg()
            .remove_item(mail_id.into(), item_guid.into())
            .await
    }
    async fn update_has_items(&self, id: u32, receiver: u32, has_items: bool) -> Result<()> {
        self.pg()
            .update_has_items(id.into(), receiver.into(), has_items.into())
            .await
    }
    async fn delete(&self, id: u32) -> Result<()> {
        self.pg().delete(id.into()).await
    }
    async fn return_to_sender(&self, id: u32, receiver: u32, sender: u32) -> Result<()> {
        self.pg()
            .return_to_sender(id.into(), receiver.into(), sender.into())
            .await
    }
    async fn create_item_text(&self, text: &str) -> Result<u32> {
        self.pg()
            .create_item_text(text)
            .await?
            .try_into()
            .map_err(Into::into)
    }
    async fn delete_expired(&self, current_time: i64) -> Result<u64> {
        self.pg().delete_expired(current_time).await
    }
}

impl MailRepository {
    pub async fn save_item_text(&self, id: u32, text: &str) -> Result<()> {
        sqlx::query("INSERT INTO characters.item_text (id, text) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET text = EXCLUDED.text")
            .bind(i64::from(id)).bind(text).execute(&*self.pool).await.context("Failed to save PostgreSQL item text")?;
        Ok(())
    }
    pub async fn delete_item_text(&self, id: u32) -> Result<()> {
        sqlx::query("DELETE FROM characters.item_text WHERE id = $1")
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL item text")?;
        Ok(())
    }
}
