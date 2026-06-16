use super::super::models::mail::*;
use crate::shared::database::characters::repositories::mail_repository_trait::MailRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct MailRepository {
    pool: Arc<MySqlPool>,
}

impl MailRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MailRepositoryTrait for MailRepository {
    // ========== QUERY METHODS ==========

    /// Find mail by ID and receiver GUID
    async fn find_by_id(&self, mail_id: u32, receiver_guid: u32) -> Result<Option<MailRow>> {
        sqlx::query_as::<_, MailRow>(
            r#"SELECT id, message_type, stationery, mail_template_id, sender_guid, receiver_guid,
                      subject, item_text_id, has_items, expire_time, deliver_time,
                      money, cod, checked
               FROM mail
               WHERE id = ? AND receiver_guid = ?"#,
        )
        .bind(mail_id)
        .bind(receiver_guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch mail by ID")
    }

    /// Load all mail for a receiver (excluding deleted mail where checked >= 2)
    async fn find_by_receiver(&self, receiver_guid: u32) -> Result<Vec<MailRow>> {
        sqlx::query_as::<_, MailRow>(
            r#"SELECT id, message_type, stationery, mail_template_id, sender_guid, receiver_guid,
                      subject, item_text_id, has_items, expire_time, deliver_time,
                      money, cod, checked
               FROM mail
               WHERE receiver_guid = ? AND checked < 2
               ORDER BY id DESC"#,
        )
        .bind(receiver_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch mail for receiver")
    }

    /// Load items attached to a specific mail
    async fn find_mail_items(&self, mail_id: u32) -> Result<Vec<MailItemRow>> {
        sqlx::query_as::<_, MailItemRow>(
            r#"SELECT mail_id, item_guid, item_id, receiver_guid
               FROM mail_items
               WHERE mail_id = ?"#,
        )
        .bind(mail_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch mail items")
    }

    /// Load all items for a receiver (across all their mail)
    async fn find_items_by_receiver(&self, receiver_guid: u32) -> Result<Vec<MailItemRow>> {
        sqlx::query_as::<_, MailItemRow>(
            r#"SELECT mail_id, item_guid, item_id, receiver_guid
               FROM mail_items
               WHERE receiver_guid = ?"#,
        )
        .bind(receiver_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch mail items for receiver")
    }

    /// Get mail count for receiver
    async fn count_by_receiver(&self, receiver_guid: u32) -> Result<u32> {
        let result = sqlx::query_scalar::<_, u32>(
            "SELECT COUNT(*) FROM mail WHERE receiver_guid = ? AND checked < 2",
        )
        .bind(receiver_guid)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to count mail for receiver")?;

        Ok(result)
    }

    /// Load item text by ID
    async fn find_item_text(&self, text_id: u32) -> Result<Option<ItemTextRow>> {
        sqlx::query_as::<_, ItemTextRow>("SELECT id, text FROM item_text WHERE id = ?")
            .bind(text_id)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch item text")
    }

    /// Find player GUID by name
    async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>> {
        let result = sqlx::query_scalar::<_, u32>("SELECT guid FROM characters WHERE name = ?")
            .bind(name)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find player by name")?;

        Ok(result)
    }

    /// Find player race by GUID
    async fn find_player_race(&self, guid: u32) -> Result<Option<u8>> {
        let result = sqlx::query_scalar::<_, u8>("SELECT race FROM characters WHERE guid = ?")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find player race")?;

        Ok(result)
    }

    // ========== COMMAND METHODS ==========

    /// Create a new mail message, returns mail ID
    async fn create(&self, mail: &MailRow) -> Result<u32> {
        let result = sqlx::query(
            r#"INSERT INTO mail
               (message_type, stationery, mail_template_id, sender_guid, receiver_guid,
                subject, item_text_id, has_items, expire_time, deliver_time, money, cod, checked)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(mail.message_type)
        .bind(mail.stationery)
        .bind(mail.mail_template_id)
        .bind(mail.sender_guid)
        .bind(mail.receiver_guid)
        .bind(&mail.subject)
        .bind(mail.item_text_id)
        .bind(mail.has_items)
        .bind(mail.expire_time)
        .bind(mail.deliver_time)
        .bind(mail.money)
        .bind(mail.cod)
        .bind(mail.checked)
        .execute(&*self.pool)
        .await
        .context("Failed to create mail")?;

        Ok(result.last_insert_id() as u32)
    }

    /// Add item attachment to mail
    async fn add_item(
        &self,
        mail_id: u32,
        item_guid: u32,
        item_id: u32,
        receiver_guid: u32,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO mail_items (mail_id, item_guid, item_id, receiver_guid)
               VALUES (?, ?, ?, ?)"#,
        )
        .bind(mail_id)
        .bind(item_guid)
        .bind(item_id)
        .bind(receiver_guid)
        .execute(&*self.pool)
        .await
        .context("Failed to add mail item")?;

        Ok(())
    }

    /// Update mail checked status
    async fn update_checked(&self, mail_id: u32, receiver_guid: u32, checked: u8) -> Result<()> {
        sqlx::query("UPDATE mail SET checked = ? WHERE id = ? AND receiver_guid = ?")
            .bind(checked)
            .bind(mail_id)
            .bind(receiver_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update mail checked status")?;

        Ok(())
    }

    /// Clear money from mail
    async fn clear_money(&self, mail_id: u32, receiver_guid: u32) -> Result<()> {
        sqlx::query("UPDATE mail SET money = 0, checked = 1 WHERE id = ? AND receiver_guid = ?")
            .bind(mail_id)
            .bind(receiver_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to clear mail money")?;

        Ok(())
    }

    /// Remove item from mail
    async fn remove_item(&self, mail_id: u32, item_guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM mail_items WHERE mail_id = ? AND item_guid = ?")
            .bind(mail_id)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to remove mail item")?;

        Ok(())
    }

    /// Update has_items flag
    async fn update_has_items(
        &self,
        mail_id: u32,
        receiver_guid: u32,
        has_items: bool,
    ) -> Result<()> {
        sqlx::query("UPDATE mail SET has_items = ? WHERE id = ? AND receiver_guid = ?")
            .bind(has_items as u8)
            .bind(mail_id)
            .bind(receiver_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update mail has_items")?;

        Ok(())
    }

    /// Delete mail (mark as deleted)
    async fn delete(&self, mail_id: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete items first
        sqlx::query("DELETE FROM mail_items WHERE mail_id = ?")
            .bind(mail_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete mail items")?;

        // Delete mail
        sqlx::query("DELETE FROM mail WHERE id = ?")
            .bind(mail_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete mail")?;

        tx.commit()
            .await
            .context("Failed to commit mail deletion")?;
        Ok(())
    }

    /// Return mail to sender (swap receiver/sender)
    async fn return_to_sender(
        &self,
        mail_id: u32,
        receiver_guid: u32,
        sender_guid: u32,
    ) -> Result<()> {
        sqlx::query("UPDATE mail SET receiver_guid = ?, sender_guid = ?, checked = 0 WHERE id = ? AND receiver_guid = ?")
            .bind(sender_guid)
            .bind(receiver_guid)
            .bind(mail_id)
            .bind(receiver_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to return mail to sender")?;

        Ok(())
    }

    /// Create new item text, returns ID
    async fn create_item_text(&self, text: &str) -> Result<u32> {
        let result = sqlx::query("INSERT INTO item_text (text) VALUES (?)")
            .bind(text)
            .execute(&*self.pool)
            .await
            .context("Failed to create item text")?;

        Ok(result.last_insert_id() as u32)
    }

    /// Delete expired mails
    async fn delete_expired(&self, current_time: i64) -> Result<u64> {
        let mut tx = self.pool.begin().await?;

        // Delete items from expired mail
        sqlx::query(
            "DELETE mi FROM mail_items mi INNER JOIN mail m ON mi.mail_id = m.id WHERE m.expire_time > 0 AND m.expire_time < ?"
        )
        .bind(current_time)
        .execute(&mut *tx)
        .await
        .context("Failed to delete items from expired mail")?;

        // Delete expired mail
        let mail_result = sqlx::query("DELETE FROM mail WHERE expire_time > 0 AND expire_time < ?")
            .bind(current_time)
            .execute(&mut *tx)
            .await
            .context("Failed to delete expired mail")?;

        tx.commit()
            .await
            .context("Failed to commit expired mail deletion")?;

        Ok(mail_result.rows_affected())
    }
}

// Legacy methods for backward compatibility - these are deprecated but kept for existing code
impl MailRepository {
    /// Create or update item text (legacy method)
    pub async fn save_item_text(&self, text_id: u32, text: &str) -> Result<()> {
        sqlx::query("REPLACE INTO item_text (id, text) VALUES (?, ?)")
            .bind(text_id)
            .bind(text)
            .execute(&*self.pool)
            .await
            .context("Failed to save item text")?;

        Ok(())
    }

    /// Delete item text (legacy method)
    pub async fn delete_item_text(&self, text_id: u32) -> Result<()> {
        sqlx::query("DELETE FROM item_text WHERE id = ?")
            .bind(text_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete item text")?;

        Ok(())
    }
}
