//! Corpse persistence — `corpse` table CRUD.

use super::super::models::corpse::CorpseRow;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct CorpseRepository {
    pool: Arc<MySqlPool>,
}

impl CorpseRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load every persisted corpse. Called at world startup to rehydrate
    /// corpses created before a server restart.
    pub async fn load_all(&self) -> Result<Vec<CorpseRow>> {
        sqlx::query_as::<_, CorpseRow>(
            r#"SELECT guid, player_guid, position_x, position_y, position_z,
                      orientation, map, time, corpse_type, instance
               FROM corpse"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load corpses")
    }

    /// Load a single corpse by its owner's character guid. Used on login to
    /// re-link a returning player with their corpse.
    pub async fn find_for_player(&self, player_guid: u32) -> Result<Option<CorpseRow>> {
        sqlx::query_as::<_, CorpseRow>(
            r#"SELECT guid, player_guid, position_x, position_y, position_z,
                      orientation, map, time, corpse_type, instance
               FROM corpse
               WHERE player_guid = ?
               LIMIT 1"#,
        )
        .bind(player_guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find corpse for player")
    }

    /// Upsert a corpse row.
    pub async fn save(&self, row: &CorpseRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO corpse
                  (guid, player_guid, position_x, position_y, position_z,
                   orientation, map, time, corpse_type, instance)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE
                   player_guid = VALUES(player_guid),
                   position_x  = VALUES(position_x),
                   position_y  = VALUES(position_y),
                   position_z  = VALUES(position_z),
                   orientation = VALUES(orientation),
                   map         = VALUES(map),
                   time        = VALUES(time),
                   corpse_type = VALUES(corpse_type),
                   instance    = VALUES(instance)"#,
        )
        .bind(row.guid)
        .bind(row.player_guid)
        .bind(row.position_x)
        .bind(row.position_y)
        .bind(row.position_z)
        .bind(row.orientation)
        .bind(row.map)
        .bind(row.time)
        .bind(row.corpse_type)
        .bind(row.instance)
        .execute(&*self.pool)
        .await
        .context("Failed to save corpse")?;
        Ok(())
    }

    /// Delete a corpse row by GUID.
    pub async fn delete(&self, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM corpse WHERE guid = ?")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete corpse")?;
        Ok(())
    }

    /// Delete all corpses owned by a given player (used when a player is deleted).
    pub async fn delete_for_player(&self, player_guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM corpse WHERE player_guid = ?")
            .bind(player_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete corpses for player")?;
        Ok(())
    }
}
