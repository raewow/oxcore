//! Loads per-zone weather chances from the world database (`game_weather`).

use super::types::{WeatherSeasonChances, WeatherZoneChances, WEATHER_SEASONS};
use sqlx::MySqlPool;

/// Chance values above this are treated as a DB error and reset to the default.
const MAX_CHANCE: u32 = 100;
const DEFAULT_CHANCE: u32 = 25;

pub struct WeatherRepository {
    pool: MySqlPool,
}

impl WeatherRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Load `game_weather` into (zone_id, chances) pairs.
    pub async fn load_zone_chances(&self) -> anyhow::Result<Vec<(u32, WeatherZoneChances)>> {
        let rows = sqlx::query_as::<_, WeatherRow>(
            "SELECT `zone`, \
             `spring_rain_chance`, `spring_snow_chance`, `spring_storm_chance`, \
             `summer_rain_chance`, `summer_snow_chance`, `summer_storm_chance`, \
             `fall_rain_chance`, `fall_snow_chance`, `fall_storm_chance`, \
             `winter_rain_chance`, `winter_snow_chance`, `winter_storm_chance` \
             FROM `game_weather`",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(WeatherRow::into_zone).collect())
    }
}

#[derive(sqlx::FromRow)]
struct WeatherRow {
    zone: u32,
    spring_rain_chance: u32,
    spring_snow_chance: u32,
    spring_storm_chance: u32,
    summer_rain_chance: u32,
    summer_snow_chance: u32,
    summer_storm_chance: u32,
    fall_rain_chance: u32,
    fall_snow_chance: u32,
    fall_storm_chance: u32,
    winter_rain_chance: u32,
    winter_snow_chance: u32,
    winter_storm_chance: u32,
}

impl WeatherRow {
    fn into_zone(self) -> (u32, WeatherZoneChances) {
        let zone = self.zone;
        let raw = [
            (
                self.spring_rain_chance,
                self.spring_snow_chance,
                self.spring_storm_chance,
            ),
            (
                self.summer_rain_chance,
                self.summer_snow_chance,
                self.summer_storm_chance,
            ),
            (
                self.fall_rain_chance,
                self.fall_snow_chance,
                self.fall_storm_chance,
            ),
            (
                self.winter_rain_chance,
                self.winter_snow_chance,
                self.winter_storm_chance,
            ),
        ];

        let mut chances = WeatherZoneChances::default();
        for (season, (rain, snow, storm)) in raw.into_iter().enumerate().take(WEATHER_SEASONS) {
            chances.data[season] = WeatherSeasonChances {
                rain_chance: sanitize(zone, season, "rain", rain),
                snow_chance: sanitize(zone, season, "snow", snow),
                storm_chance: sanitize(zone, season, "storm", storm),
            };
        }

        (zone, chances)
    }
}

/// Chances over 100% are a data error; fall back to 25%.
fn sanitize(zone: u32, season: usize, kind: &str, chance: u32) -> u32 {
    if chance > MAX_CHANCE {
        tracing::error!(
            "Weather for zone {} season {} has wrong {} chance > 100%",
            zone,
            season,
            kind
        );
        return DEFAULT_CHANCE;
    }
    chance
}
