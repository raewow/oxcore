//! SpellManager - owns spell template data loaded from SQL

use crate::shared::database::world::repositories::SpellRepository;
use crate::world::dbc::structures::SpellEntry;
use anyhow::Result;
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::info;

/// Destination coordinates loaded from spell_target_position table
#[derive(Debug, Clone)]
pub struct SpellTargetPosition {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

pub struct SpellManager {
    spells: DashMap<u32, Arc<SpellEntry>>,
    target_positions: DashMap<u32, SpellTargetPosition>,
}

impl SpellManager {
    pub fn new() -> Self {
        Self {
            spells: DashMap::new(),
            target_positions: DashMap::new(),
        }
    }

    /// Load all spells from the spell_template SQL table
    pub async fn load(&self, world_db: &MySqlPool) -> Result<()> {
        let repo = SpellRepository::new(Arc::new(world_db.clone()));
        let entries = repo.load_all().await?;
        let count = entries.len();

        for entry in entries {
            let id = entry.id;
            self.spells.insert(id, Arc::new(entry));
        }

        info!("Loaded {} spells from spell_template", count);

        // Load spell_target_position
        self.load_target_positions(world_db).await?;

        Ok(())
    }

    /// Load spell_target_position table (coordinates for teleport spells)
    async fn load_target_positions(&self, world_db: &MySqlPool) -> Result<()> {
        let rows = sqlx::query(
            "SELECT CAST(id AS UNSIGNED) AS id, \
                    CAST(target_map AS UNSIGNED) AS target_map, \
                    target_position_x, target_position_y, target_position_z, target_orientation \
             FROM spell_target_position",
        )
        .fetch_all(world_db)
        .await?;

        let count = rows.len();
        for row in rows {
            use sqlx::Row;
            let id: u32 = row.try_get::<u64, _>("id").unwrap_or(0) as u32;
            let map_id: u32 = row.try_get::<u64, _>("target_map").unwrap_or(0) as u32;
            let x: f32 = row.try_get("target_position_x").unwrap_or(0.0);
            let y: f32 = row.try_get("target_position_y").unwrap_or(0.0);
            let z: f32 = row.try_get("target_position_z").unwrap_or(0.0);
            let orientation: f32 = row.try_get("target_orientation").unwrap_or(0.0);

            self.target_positions.insert(
                id,
                SpellTargetPosition {
                    map_id,
                    x,
                    y,
                    z,
                    orientation,
                },
            );
        }

        info!("Loaded {} spell_target_position entries", count);
        Ok(())
    }

    /// Get a spell entry by ID
    pub fn get(&self, spell_id: u32) -> Option<Arc<SpellEntry>> {
        self.spells.get(&spell_id).map(|r| Arc::clone(&r))
    }

    /// Get spell target position (for teleport spells using TARGET_LOCATION_DATABASE)
    pub fn get_spell_target_position(&self, spell_id: u32) -> Option<SpellTargetPosition> {
        self.target_positions.get(&spell_id).map(|r| r.clone())
    }

    /// Get spell count
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    /// Search spells by name (case-insensitive substring match)
    pub fn search_by_name(&self, search: &str) -> Vec<Arc<SpellEntry>> {
        let search_lower = search.to_lowercase();
        let mut results: Vec<Arc<SpellEntry>> = self
            .spells
            .iter()
            .filter(|entry| entry.value().name.to_lowercase().contains(&search_lower))
            .map(|entry| Arc::clone(entry.value()))
            .collect();
        results.sort_by_key(|s| s.id);
        results
    }
}
