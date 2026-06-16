use super::super::models::honor::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct HonorRepository {
    pool: Arc<MySqlPool>,
}

impl HonorRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Find all honor CP entries for a specific character.
    pub async fn find_honor_cp(&self, guid: u32) -> Result<Vec<HonorCPRow>> {
        sqlx::query_as::<_, HonorCPRow>(
            r#"SELECT guid, victim_type, victim_id, cp, date, type FROM character_honor_cp WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch honor CP entries")
    }

    /// Find stored honor data for a specific character.
    pub async fn find_stored_data(&self, guid: u32) -> Result<Option<HonorStoredRow>> {
        sqlx::query_as::<_, HonorStoredRow>(
            r#"SELECT guid, honor_rank_points, honor_standing, honor_highest_rank, honor_last_week_hk, honor_last_week_cp, honor_stored_hk, honor_stored_dk FROM characters WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch stored honor data")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Save honor CP entry.
    pub async fn save_honor_cp(&self, cp: &HonorCPRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO character_honor_cp (guid, victim_type, victim_id, cp, date, type) VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(cp.guid)
        .bind(cp.victim_type)
        .bind(cp.victim_id)
        .bind(cp.cp)
        .bind(cp.date)
        .bind(cp.r#type)
        .execute(&*self.pool)
        .await
        .context("Failed to save honor CP entry")?;

        Ok(())
    }

    /// Save stored honor data.
    pub async fn save_stored_data(&self, data: &HonorStoredRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE characters SET honor_rank_points = ?, honor_standing = ?, honor_highest_rank = ?, honor_last_week_hk = ?, honor_last_week_cp = ?, honor_stored_hk = ?, honor_stored_dk = ? WHERE guid = ?"#,
        )
        .bind(data.honor_rank_points)
        .bind(data.honor_standing)
        .bind(data.honor_highest_rank)
        .bind(data.honor_last_week_hk)
        .bind(data.honor_last_week_cp)
        .bind(data.honor_stored_hk)
        .bind(data.honor_stored_dk)
        .bind(data.guid)
        .execute(&*self.pool)
        .await
        .context("Failed to save stored honor data")?;

        Ok(())
    }

    /// Delete all honor CP entries for a specific character.
    pub async fn delete_honor_cp(&self, guid: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM character_honor_cp WHERE guid = ?"#)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete honor CP entries")?;

        Ok(())
    }
}
