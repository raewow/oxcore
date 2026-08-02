//! Database repository for graveyard zone data

use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

/// Raw row from game_graveyard_zone table
#[derive(Debug, sqlx::FromRow)]
pub struct GraveyardZoneRow {
    pub id: u32,
    pub ghost_zone: u32,
    pub faction: u16,
}

/// Repository for graveyard-related database operations
pub struct GraveyardRepository {
    pool: Arc<MySqlPool>,
}

impl GraveyardRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load all graveyard zone mappings from the database
    pub async fn load_graveyard_zones(&self) -> Result<Vec<GraveyardZoneRow>> {
        sqlx::query_as::<_, GraveyardZoneRow>(
            "SELECT id, ghost_zone, faction FROM game_graveyard_zone WHERE patch_min <= 10 AND patch_max >= 10"
        )
        .fetch_all(self.pool.as_ref())
        .await
        .context("Failed to load game_graveyard_zone")
    }
}
