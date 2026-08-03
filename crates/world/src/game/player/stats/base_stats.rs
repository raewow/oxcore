//! Base stat data loader
//!
//! Loads player_levelstats and player_classlevelstats from the world database
//! at server startup. Provides base stats for race/class/level combinations.

use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn};

/// Base stats for a specific race/class/level combination
#[derive(Debug, Clone, Default)]
pub struct BaseLevelStats {
    pub strength: u32,
    pub agility: u32,
    pub stamina: u32,
    pub intellect: u32,
    pub spirit: u32,
}

/// Base health/mana for a specific class/level combination
#[derive(Debug, Clone, Default)]
pub struct BaseClassLevelStats {
    pub base_health: u32,
    pub base_mana: u32,
}

/// Holds all base stat data loaded from the world database
#[derive(Debug)]
pub struct BaseStatsData {
    /// (race, class, level) -> base stats (STR, AGI, STA, INT, SPI)
    level_stats: HashMap<(u8, u8, u8), BaseLevelStats>,
    /// (class, level) -> base health/mana
    class_level_stats: HashMap<(u8, u8), BaseClassLevelStats>,
}

impl BaseStatsData {
    pub fn new() -> Self {
        Self {
            level_stats: HashMap::new(),
            class_level_stats: HashMap::new(),
        }
    }

    /// Load base stats from the world database
    pub async fn load(world_pool: &sqlx::PgPool) -> Result<Self> {
        let mut data = Self::new();

        let repository = oxcore_db::database::world::PlayerStatsRepository::new(
            std::sync::Arc::new(world_pool.clone()),
        );
        match repository.load_level_stats().await {
            Ok(rows) => {
                for row in rows {
                    let Ok(race) = u32::try_from(row.race) else {
                        continue;
                    };
                    let Ok(class) = u32::try_from(row.class) else {
                        continue;
                    };
                    let Ok(level) = u32::try_from(row.level) else {
                        continue;
                    };
                    let Ok(str_val) = u32::try_from(row.str) else {
                        continue;
                    };
                    let Ok(agi) = u32::try_from(row.agi) else {
                        continue;
                    };
                    let Ok(sta) = u32::try_from(row.sta) else {
                        continue;
                    };
                    let Ok(inte) = u32::try_from(row.inte) else {
                        continue;
                    };
                    let Ok(spi) = u32::try_from(row.spi) else {
                        continue;
                    };

                    if race == 0
                        || race > 11
                        || class == 0
                        || class > 11
                        || level == 0
                        || level > 60
                    {
                        continue;
                    }

                    data.level_stats.insert(
                        (race as u8, class as u8, level as u8),
                        BaseLevelStats {
                            strength: str_val,
                            agility: agi,
                            stamina: sta,
                            intellect: inte,
                            spirit: spi,
                        },
                    );
                }
                info!(
                    "Loaded {} player level stat entries",
                    data.level_stats.len()
                );
            }
            Err(e) => {
                warn!(
                    "Failed to load player_levelstats: {}. Stats will use fallback values.",
                    e
                );
            }
        }

        match repository.load_class_level_stats().await {
            Ok(rows) => {
                for row in rows {
                    let Ok(class) = u32::try_from(row.class) else {
                        continue;
                    };
                    let Ok(level) = u32::try_from(row.level) else {
                        continue;
                    };
                    let Ok(basehp) = u32::try_from(row.basehp) else {
                        continue;
                    };
                    let Ok(basemana) = u32::try_from(row.basemana) else {
                        continue;
                    };

                    if class == 0 || class > 11 || level == 0 || level > 60 {
                        continue;
                    }

                    data.class_level_stats.insert(
                        (class as u8, level as u8),
                        BaseClassLevelStats {
                            base_health: basehp,
                            base_mana: basemana,
                        },
                    );
                }
                info!(
                    "Loaded {} player class level stat entries",
                    data.class_level_stats.len()
                );
            }
            Err(e) => {
                warn!(
                    "Failed to load player_classlevelstats: {}. Base HP/mana will use fallback.",
                    e
                );
            }
        }

        Ok(data)
    }

    /// Get base stats for a race/class/level. Falls back to level 1 or defaults.
    pub fn get_level_stats(&self, race: u8, class: u8, level: u8) -> BaseLevelStats {
        if let Some(stats) = self.level_stats.get(&(race, class, level)) {
            return stats.clone();
        }
        // Fallback: try level 1
        if let Some(stats) = self.level_stats.get(&(race, class, 1)) {
            return stats.clone();
        }
        // Ultimate fallback
        BaseLevelStats {
            strength: 20,
            agility: 20,
            stamina: 20,
            intellect: 20,
            spirit: 20,
        }
    }

    /// Get base health/mana for a class/level. Falls back to level 1 or defaults.
    pub fn get_class_level_stats(&self, class: u8, level: u8) -> BaseClassLevelStats {
        if let Some(stats) = self.class_level_stats.get(&(class, level)) {
            return stats.clone();
        }
        // Fallback: try level 1
        if let Some(stats) = self.class_level_stats.get(&(class, 1)) {
            return stats.clone();
        }
        // Ultimate fallback
        BaseClassLevelStats {
            base_health: 20,
            base_mana: 0,
        }
    }
}

impl Default for BaseStatsData {
    fn default() -> Self {
        Self::new()
    }
}
