use super::super::models::social::*;
use anyhow::Result;
use async_trait::async_trait;

/// Trait abstraction for social repository operations.
/// Enables dependency injection and mocking for tests.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SocialRepositoryTrait: Send + Sync {
    // ========== QUERY METHODS (Read Operations) ==========

    /// Load all social entries for a character (friends and ignore list).
    /// Returns list of (friend_guid, flags) pairs.
    async fn find_by_guid(&self, guid: u32) -> Result<Vec<CharacterSocialRow>>;

    /// Lookup a player GUID by character name (for friend/ignore operations)
    /// Returns None if player not found
    async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>>;

    /// Check if a specific friend/ignore relationship exists.
    async fn exists(&self, guid: u32, friend_guid: u32) -> Result<bool>;

    /// Get character name by GUID (for friend list display).
    /// Returns None if character not found
    async fn get_character_name(&self, character_guid: u32) -> Result<Option<String>>;

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Add or update a social entry (friend or ignore).
    /// Uses ON DUPLICATE KEY UPDATE to handle the case where the entry already exists.
    /// If the entry exists, the flags will be OR'd with the new flags.
    async fn add_or_update(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()>;

    /// Set flags for an existing social entry (replaces flags entirely).
    async fn update_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()>;

    /// Add flags to an existing social entry (bitwise OR).
    async fn add_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()>;

    /// Remove flags from an existing social entry (bitwise AND NOT).
    async fn remove_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()>;

    /// Remove a social entry.
    async fn remove(&self, guid: u32, friend_guid: u32) -> Result<()>;

    /// Delete all social entries for a character (both as owner and as friend).
    /// Used when deleting a character.
    async fn delete_all_for_character(&self, guid: u32) -> Result<()>;
}
