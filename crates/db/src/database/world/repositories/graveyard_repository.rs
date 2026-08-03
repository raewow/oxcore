//! Database repository for graveyard zone data

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;

/// Raw row from game_graveyard_zone table
#[derive(Debug, sqlx::FromRow)]
pub struct GraveyardZoneRow {
    pub id: i64,
    pub ghost_zone: i64,
    pub faction: i32,
}

/// Repository for graveyard-related database operations
pub struct GraveyardRepository {
    pool: Arc<PgPool>,
}

impl GraveyardRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Load all graveyard zone mappings from the database
    pub async fn load_graveyard_zones(&self) -> Result<Vec<GraveyardZoneRow>> {
        sqlx::query_as::<_, GraveyardZoneRow>(
            "SELECT id, ghost_zone, faction FROM world.game_graveyard_zone WHERE patch_min <= 10 AND patch_max >= 10"
        )
        .fetch_all(self.pool.as_ref())
        .await
        .context("Failed to load game_graveyard_zone")
    }
}
