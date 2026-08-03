//! Global weather chance table.

use super::repository::WeatherRepository;
use super::types::WeatherZoneChances;
use dashmap::DashMap;
use sqlx::PgPool;

/// Holds the `game_weather` chances for every zone that has them.
///
/// Zones missing from the table have no weather at all — they stay fine.
pub struct WeatherManager {
    zone_chances: DashMap<u32, WeatherZoneChances>,
}

impl WeatherManager {
    pub fn new() -> Self {
        Self {
            zone_chances: DashMap::new(),
        }
    }

    /// Load (or reload) the zone chances from the world database.
    pub async fn load(&self, pool: &PgPool) -> anyhow::Result<()> {
        let repo = WeatherRepository::new(pool.clone());
        let zones = repo.load_zone_chances().await?;

        self.zone_chances.clear();
        for (zone_id, chances) in zones {
            self.zone_chances.insert(zone_id, chances);
        }

        tracing::info!(
            "Loaded {} weather definitions from game_weather",
            self.zone_chances.len()
        );
        Ok(())
    }

    /// Chances for a zone, or `None` when the zone has no weather data.
    pub fn get_chances(&self, zone_id: u32) -> Option<WeatherZoneChances> {
        self.zone_chances.get(&zone_id).map(|entry| *entry)
    }

    /// Number of zones with weather data.
    pub fn zone_count(&self) -> usize {
        self.zone_chances.len()
    }

    /// Insert chances directly (used by tests).
    pub fn set_chances(&self, zone_id: u32, chances: WeatherZoneChances) {
        self.zone_chances.insert(zone_id, chances);
    }
}

impl Default for WeatherManager {
    fn default() -> Self {
        Self::new()
    }
}
