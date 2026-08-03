use anyhow::{Context, Result};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

/// Signed PostgreSQL rows for base player stats. Protocol-sized conversion belongs to the caller.
#[derive(Debug, FromRow)]
pub struct PgPlayerLevelStatRow {
    pub race: i16,
    pub class: i16,
    pub level: i16,
    pub str: i16,
    pub agi: i16,
    pub sta: i16,
    pub inte: i16,
    pub spi: i16,
}

#[derive(Debug, FromRow)]
pub struct PgPlayerClassLevelStatRow {
    pub class: i16,
    pub level: i16,
    pub basehp: i32,
    pub basemana: i32,
}

pub struct PlayerStatsRepository {
    pool: Arc<PgPool>,
}
impl PlayerStatsRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn load_level_stats(&self) -> Result<Vec<PgPlayerLevelStatRow>> {
        sqlx::query_as("SELECT race, class, level, str, agi, sta, inte, spi FROM world.player_levelstats ORDER BY race, class, level")
            .fetch_all(&*self.pool).await.context("Failed to load PostgreSQL player level stats")
    }
    pub async fn load_class_level_stats(&self) -> Result<Vec<PgPlayerClassLevelStatRow>> {
        sqlx::query_as("SELECT class, level, basehp, basemana FROM world.player_classlevelstats ORDER BY class, level")
            .fetch_all(&*self.pool).await.context("Failed to load PostgreSQL player class level stats")
    }
}
