use super::super::models::honor::*;
use anyhow::{Context, Result};
use sqlx::{PgPool, Row};
use std::sync::Arc;

pub struct HonorRepository {
    pool: Arc<PgPool>,
}

impl HonorRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Find all honor CP entries for a specific character.
    pub async fn find_honor_cp(&self, guid: u32) -> Result<Vec<HonorCPRow>> {
        let rows = sqlx::query(
            r#"SELECT guid, victim_type, victim_id, cp, date, type FROM characters.character_honor_cp WHERE guid = $1"#,
        )
        .bind(i64::from(guid))
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch honor CP entries")?;
        Ok(rows.into_iter().map(honor_cp_from_row).collect())
    }

    /// Find stored honor data for a specific character.
    pub async fn find_stored_data(&self, guid: u32) -> Result<Option<HonorStoredRow>> {
        let row = sqlx::query(
            r#"SELECT guid, honor_rank_points, honor_standing, honor_highest_rank, honor_last_week_hk, honor_last_week_cp, honor_stored_hk, honor_stored_dk FROM characters.characters WHERE guid = $1"#,
        )
        .bind(i64::from(guid))
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch stored honor data")?;
        Ok(row.map(honor_stored_from_row))
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Save honor CP entry.
    pub async fn save_honor_cp(&self, cp: &HonorCPRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO characters.character_honor_cp (guid, victim_type, victim_id, cp, date, type) VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(i64::from(cp.guid))
        .bind(i16::from(cp.victim_type))
        .bind(i64::from(cp.victim_id))
        .bind(cp.cp)
        .bind(i64::from(cp.date))
        .bind(i16::from(cp.r#type))
        .execute(&*self.pool)
        .await
        .context("Failed to save honor CP entry")?;

        Ok(())
    }

    /// Save stored honor data.
    pub async fn save_stored_data(&self, data: &HonorStoredRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE characters.characters SET honor_rank_points = $1, honor_standing = $2, honor_highest_rank = $3, honor_last_week_hk = $4, honor_last_week_cp = $5, honor_stored_hk = $6, honor_stored_dk = $7 WHERE guid = $8"#,
        )
        .bind(data.honor_rank_points)
        .bind(i64::from(data.honor_standing))
        .bind(i16::from(data.honor_highest_rank))
        .bind(i64::from(data.honor_last_week_hk))
        .bind(data.honor_last_week_cp)
        .bind(i64::from(data.honor_stored_hk))
        .bind(i64::from(data.honor_stored_dk))
        .bind(i64::from(data.guid))
        .execute(&*self.pool)
        .await
        .context("Failed to save stored honor data")?;

        Ok(())
    }

    /// Delete all honor CP entries for a specific character.
    pub async fn delete_honor_cp(&self, guid: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM characters.character_honor_cp WHERE guid = $1"#)
            .bind(i64::from(guid))
            .execute(&*self.pool)
            .await
            .context("Failed to delete honor CP entries")?;

        Ok(())
    }
}

fn honor_cp_from_row(row: sqlx::postgres::PgRow) -> HonorCPRow {
    HonorCPRow {
        guid: row.get::<i64, _>("guid") as u32,
        victim_type: row.get::<i16, _>("victim_type") as u8,
        victim_id: row.get::<i64, _>("victim_id") as u32,
        cp: row.get("cp"),
        date: row.get::<i64, _>("date") as u32,
        r#type: row.get::<i16, _>("type") as u8,
    }
}

fn honor_stored_from_row(row: sqlx::postgres::PgRow) -> HonorStoredRow {
    HonorStoredRow {
        guid: row.get::<i64, _>("guid") as u32,
        honor_rank_points: row.get("honor_rank_points"),
        honor_standing: row.get::<i64, _>("honor_standing") as u32,
        honor_highest_rank: row.get::<i16, _>("honor_highest_rank") as u8,
        honor_last_week_hk: row.get::<i64, _>("honor_last_week_hk") as u32,
        honor_last_week_cp: row.get("honor_last_week_cp"),
        honor_stored_hk: row.get::<i64, _>("honor_stored_hk") as u32,
        honor_stored_dk: row.get::<i64, _>("honor_stored_dk") as u32,
    }
}
