use super::super::models::reputation::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct ReputationRepository {
    pool: Arc<MySqlPool>,
}

impl ReputationRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Find all reputation entries for a specific character.
    pub async fn find_reputations(&self, guid: u32) -> Result<Vec<ReputationRow>> {
        sqlx::query_as::<_, ReputationRow>(
            r#"SELECT guid, faction, standing, flags FROM character_reputation WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch reputation entries")
    }

    /// Find a specific reputation entry for a character and faction.
    pub async fn find_reputation(&self, guid: u32, faction: u32) -> Result<Option<ReputationRow>> {
        sqlx::query_as::<_, ReputationRow>(r#"SELECT guid, faction, standing, flags FROM character_reputation WHERE guid = ? AND faction = ?"#)
            .bind(guid)
            .bind(faction)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch reputation entry")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Save or update a reputation entry.
    pub async fn save_reputation(&self, reputation: &ReputationRow) -> Result<()> {
        sqlx::query(r#"REPLACE INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ?, ?)"#)
            .bind(reputation.guid)
            .bind(reputation.faction)
            .bind(reputation.standing)
            .bind(reputation.flags)
            .execute(&*self.pool)
            .await
            .context("Failed to save reputation entry")?;

        Ok(())
    }

    /// Update reputation standing and flags for a specific entry.
    pub async fn update_reputation(
        &self,
        guid: u32,
        faction: u32,
        standing: i32,
        flags: i32,
    ) -> Result<()> {
        sqlx::query(r#"UPDATE character_reputation SET standing = ?, flags = ? WHERE guid = ? AND faction = ?"#)
            .bind(standing)
            .bind(flags)
            .bind(guid)
            .bind(faction)
            .execute(&*self.pool)
            .await
            .context("Failed to update reputation entry")?;

        Ok(())
    }

    /// Delete all reputation entries for a specific character.
    pub async fn delete_reputations(&self, guid: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM character_reputation WHERE guid = ?"#)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete reputation entries")?;

        Ok(())
    }
}
