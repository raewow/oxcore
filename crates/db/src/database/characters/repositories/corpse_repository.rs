//! Corpse persistence — `corpse` table CRUD.

use super::super::models::corpse::CorpseRow;
use anyhow::{Context, Result};
use sqlx::{PgPool, Row};
use std::sync::Arc;

pub struct CorpseRepository {
    pool: Arc<PgPool>,
}

impl CorpseRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Load every persisted corpse. Called at world startup to rehydrate
    /// corpses created before a server restart.
    pub async fn load_all(&self) -> Result<Vec<CorpseRow>> {
        let rows = sqlx::query(
            r#"SELECT guid, player_guid, position_x, position_y, position_z,
                       orientation, map, time, corpse_type, instance
               FROM characters.corpse"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load corpses")?;
        Ok(rows.into_iter().map(corpse_from_row).collect())
    }

    /// Load a single corpse by its owner's character guid. Used on login to
    /// re-link a returning player with their corpse.
    pub async fn find_for_player(&self, player_guid: u32) -> Result<Option<CorpseRow>> {
        let row = sqlx::query(
            r#"SELECT guid, player_guid, position_x, position_y, position_z,
                      orientation, map, time, corpse_type, instance
               FROM characters.corpse
               WHERE player_guid = $1
               LIMIT 1"#,
        )
        .bind(i64::from(player_guid))
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find corpse for player")?;
        Ok(row.map(corpse_from_row))
    }

    /// Upsert a corpse row.
    pub async fn save(&self, row: &CorpseRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO characters.corpse
                  (guid, player_guid, position_x, position_y, position_z,
                   orientation, map, time, corpse_type, instance)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (guid) DO UPDATE SET
                    player_guid = EXCLUDED.player_guid,
                    position_x  = EXCLUDED.position_x,
                    position_y  = EXCLUDED.position_y,
                    position_z  = EXCLUDED.position_z,
                    orientation = EXCLUDED.orientation,
                    map         = EXCLUDED.map,
                    time        = EXCLUDED.time,
                    corpse_type = EXCLUDED.corpse_type,
                    instance    = EXCLUDED.instance"#,
        )
        .bind(i64::from(row.guid))
        .bind(i64::from(row.player_guid))
        .bind(row.position_x)
        .bind(row.position_y)
        .bind(row.position_z)
        .bind(row.orientation)
        .bind(i64::from(row.map))
        .bind(row.time as i64)
        .bind(i16::from(row.corpse_type))
        .bind(i64::from(row.instance))
        .execute(&*self.pool)
        .await
        .context("Failed to save corpse")?;
        Ok(())
    }

    /// Delete a corpse row by GUID.
    pub async fn delete(&self, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM characters.corpse WHERE guid = $1")
            .bind(i64::from(guid))
            .execute(&*self.pool)
            .await
            .context("Failed to delete corpse")?;
        Ok(())
    }

    /// Delete all corpses owned by a given player (used when a player is deleted).
    pub async fn delete_for_player(&self, player_guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM characters.corpse WHERE player_guid = $1")
            .bind(i64::from(player_guid))
            .execute(&*self.pool)
            .await
            .context("Failed to delete corpses for player")?;
        Ok(())
    }
}

fn corpse_from_row(row: sqlx::postgres::PgRow) -> CorpseRow {
    CorpseRow {
        guid: row.get::<i64, _>("guid") as u32,
        player_guid: row.get::<i64, _>("player_guid") as u32,
        position_x: row.get("position_x"),
        position_y: row.get("position_y"),
        position_z: row.get("position_z"),
        orientation: row.get("orientation"),
        map: row.get::<i64, _>("map") as u32,
        time: row.get::<i64, _>("time") as u64,
        corpse_type: row.get::<i16, _>("corpse_type") as u8,
        instance: row.get::<i64, _>("instance") as u32,
    }
}
