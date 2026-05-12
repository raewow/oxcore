use super::super::models::social::*;
use super::social_repository_trait::SocialRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct SocialRepository {
    pool: Arc<MySqlPool>,
}

impl SocialRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Load all social entries for a character (friends and ignore list).
    /// Returns list of (friend_guid, flags) pairs.
    pub async fn find_by_guid(&self, guid: u32) -> Result<Vec<CharacterSocialRow>> {
        sqlx::query_as::<_, CharacterSocialRow>(
            r#"SELECT guid, friend, flags
               FROM character_social
               WHERE guid = ?
               LIMIT 255"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch social entries")
    }

    /// Lookup a player GUID by character name (for friend/ignore operations)
    pub async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>> {
        sqlx::query_scalar(r#"SELECT guid FROM characters WHERE name = ? LIMIT 1"#)
            .bind(name)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to lookup player by name")
    }

    /// Check if a specific friend/ignore relationship exists.
    pub async fn exists(&self, guid: u32, friend_guid: u32) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_social WHERE guid = ? AND friend = ?",
        )
        .bind(guid)
        .bind(friend_guid)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to check social entry existence")?;

        Ok(count > 0)
    }

    /// Get character name by GUID (for friend list display).
    pub async fn get_character_name(&self, character_guid: u32) -> Result<Option<String>> {
        sqlx::query_scalar(r#"SELECT name FROM characters WHERE guid = ? LIMIT 1"#)
            .bind(character_guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to lookup character name")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Add or update a social entry (friend or ignore).
    /// Uses ON DUPLICATE KEY UPDATE to handle the case where the entry already exists.
    /// If the entry exists, the flags will be OR'd with the new flags.
    pub async fn add_or_update(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO character_social (guid, friend, flags)
               VALUES (?, ?, ?)
               ON DUPLICATE KEY UPDATE flags = flags | VALUES(flags)"#,
        )
        .bind(guid)
        .bind(friend_guid)
        .bind(flags)
        .execute(&*self.pool)
        .await
        .context("Failed to add or update social entry")?;

        Ok(())
    }

    /// Set flags for an existing social entry (replaces flags entirely).
    pub async fn update_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        sqlx::query("UPDATE character_social SET flags = ? WHERE guid = ? AND friend = ?")
            .bind(flags)
            .bind(guid)
            .bind(friend_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update social entry flags")?;

        Ok(())
    }

    /// Add flags to an existing social entry (bitwise OR).
    pub async fn add_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        sqlx::query(
            "UPDATE character_social SET flags = (flags | ?) WHERE guid = ? AND friend = ?",
        )
        .bind(flags)
        .bind(guid)
        .bind(friend_guid)
        .execute(&*self.pool)
        .await
        .context("Failed to add flags to social entry")?;

        Ok(())
    }

    /// Remove flags from an existing social entry (bitwise AND NOT).
    pub async fn remove_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        sqlx::query(
            "UPDATE character_social SET flags = (flags & ~?) WHERE guid = ? AND friend = ?",
        )
        .bind(flags)
        .bind(guid)
        .bind(friend_guid)
        .execute(&*self.pool)
        .await
        .context("Failed to remove flags from social entry")?;

        Ok(())
    }

    /// Remove a social entry.
    pub async fn remove(&self, guid: u32, friend_guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM character_social WHERE guid = ? AND friend = ?")
            .bind(guid)
            .bind(friend_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete social entry")?;

        Ok(())
    }

    /// Delete all social entries for a character (both as owner and as friend).
    /// Used when deleting a character.
    pub async fn delete_all_for_character(&self, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM character_social WHERE guid = ? OR friend = ?")
            .bind(guid)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete all social entries for character")?;

        Ok(())
    }
}

// ========== TRAIT IMPLEMENTATION ==========

#[async_trait]
impl SocialRepositoryTrait for SocialRepository {
    async fn find_by_guid(&self, guid: u32) -> Result<Vec<CharacterSocialRow>> {
        self.find_by_guid(guid).await
    }

    async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<u32>> {
        self.find_player_guid_by_name(name).await
    }

    async fn exists(&self, guid: u32, friend_guid: u32) -> Result<bool> {
        self.exists(guid, friend_guid).await
    }

    async fn get_character_name(&self, character_guid: u32) -> Result<Option<String>> {
        self.get_character_name(character_guid).await
    }

    async fn add_or_update(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        self.add_or_update(guid, friend_guid, flags).await
    }

    async fn update_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        self.update_flags(guid, friend_guid, flags).await
    }

    async fn add_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        self.add_flags(guid, friend_guid, flags).await
    }

    async fn remove_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> Result<()> {
        self.remove_flags(guid, friend_guid, flags).await
    }

    async fn remove(&self, guid: u32, friend_guid: u32) -> Result<()> {
        self.remove(guid, friend_guid).await
    }

    async fn delete_all_for_character(&self, guid: u32) -> Result<()> {
        self.delete_all_for_character(guid).await
    }
}
