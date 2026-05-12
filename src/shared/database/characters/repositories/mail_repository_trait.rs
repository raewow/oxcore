use super::super::models::mail::*;
use anyhow::Result;
use async_trait::async_trait;

/// Trait abstraction for mail repository operations.
/// Enables dependency injection and mocking for tests.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MailRepositoryTrait: Send + Sync {
    // ========== QUERY METHODS ==========

    /// Find mail by ID and receiver GUID
    async fn find_by_id(&self, mail_id: u32, receiver_guid: u32) -> Result<Option<MailRow>>;

    /// Load all mail for a receiver (excluding deleted)
    async fn find_by_receiver(&self, receiver_guid: u32) -> Result<Vec<MailRow>>;

    /// Load items attached to a specific mail
    async fn find_mail_items(&self, mail_id: u32) -> Result<Vec<MailItemRow>>;

    /// Load all items for a receiver
    async fn find_items_by_receiver(&self, receiver_guid: u32) -> Result<Vec<MailItemRow>>;

    /// Get mail count for receiver
    async fn count_by_receiver(&self, receiver_guid: u32) -> Result<u32>;

    /// Load item text by ID
    async fn find_item_text(&self, text_id: u32) -> Result<Option<ItemTextRow>>;

    /// Find player GUID by name
    async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>>;

    /// Find player race by GUID
    async fn find_player_race(&self, guid: u32) -> Result<Option<u8>>;

    // ========== COMMAND METHODS ==========

    /// Create a new mail message, returns mail ID
    async fn create(&self, mail: &MailRow) -> Result<u32>;

    /// Add item attachment to mail
    async fn add_item(
        &self,
        mail_id: u32,
        item_guid: u32,
        item_id: u32,
        receiver_guid: u32,
    ) -> Result<()>;

    /// Update mail checked status
    async fn update_checked(&self, mail_id: u32, receiver_guid: u32, checked: u8) -> Result<()>;

    /// Clear money from mail
    async fn clear_money(&self, mail_id: u32, receiver_guid: u32) -> Result<()>;

    /// Remove item from mail
    async fn remove_item(&self, mail_id: u32, item_guid: u32) -> Result<()>;

    /// Update has_items flag
    async fn update_has_items(
        &self,
        mail_id: u32,
        receiver_guid: u32,
        has_items: bool,
    ) -> Result<()>;

    /// Delete mail (mark as deleted)
    async fn delete(&self, mail_id: u32) -> Result<()>;

    /// Return mail to sender (swap receiver/sender)
    async fn return_to_sender(
        &self,
        mail_id: u32,
        receiver_guid: u32,
        sender_guid: u32,
    ) -> Result<()>;

    /// Create new item text, returns ID
    async fn create_item_text(&self, text: &str) -> Result<u32>;

    /// Delete expired mails
    async fn delete_expired(&self, current_time: i64) -> Result<u64>;
}
