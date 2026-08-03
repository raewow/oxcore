//! Loads per-zone weather chances from the world database (`game_weather`).

use super::types::{WeatherSeasonChances, WeatherZoneChances, WEATHER_SEASONS};
use sqlx::PgPool;

/// Chance values above this are treated as a DB error and reset to the default.
const MAX_CHANCE: u32 = 100;
const DEFAULT_CHANCE: u32 = 25;

pub struct WeatherRepository {
    pool: PgPool,
}

impl WeatherRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load `game_weather` into (zone_id, chances) pairs.
    pub async fn load_zone_chances(&self) -> anyhow::Result<Vec<(u32, WeatherZoneChances)>> {
        let rows = sqlx::query_as::<_, WeatherRow>(
            "SELECT zone, spring_rain_chance, spring_snow_chance, spring_storm_chance, \
              summer_rain_chance, summer_snow_chance, summer_storm_chance, \
              fall_rain_chance, fall_snow_chance, fall_storm_chance, \
              winter_rain_chance, winter_snow_chance, winter_storm_chance \
              FROM world.game_weather",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(WeatherRow::into_zone).collect()
    }
}

#[derive(sqlx::FromRow)]
struct WeatherRow {
    zone: i64,
    spring_rain_chance: i64,
    spring_snow_chance: i64,
    spring_storm_chance: i64,
    summer_rain_chance: i64,
    summer_snow_chance: i64,
    summer_storm_chance: i64,
    fall_rain_chance: i64,
    fall_snow_chance: i64,
    fall_storm_chance: i64,
    winter_rain_chance: i64,
    winter_snow_chance: i64,
    winter_storm_chance: i64,
}

impl WeatherRow {
    fn into_zone(self) -> anyhow::Result<(u32, WeatherZoneChances)> {
        let zone = u32::try_from(self.zone)
            .map_err(|_| anyhow::anyhow!("game_weather.zone must fit u32"))?;
        let raw = [
            (
                checked_chance(self.spring_rain_chance, "spring_rain_chance")?,
                checked_chance(self.spring_snow_chance, "spring_snow_chance")?,
                checked_chance(self.spring_storm_chance, "spring_storm_chance")?,
            ),
            (
                checked_chance(self.summer_rain_chance, "summer_rain_chance")?,
                checked_chance(self.summer_snow_chance, "summer_snow_chance")?,
                checked_chance(self.summer_storm_chance, "summer_storm_chance")?,
            ),
            (
                checked_chance(self.fall_rain_chance, "fall_rain_chance")?,
                checked_chance(self.fall_snow_chance, "fall_snow_chance")?,
                checked_chance(self.fall_storm_chance, "fall_storm_chance")?,
            ),
            (
                checked_chance(self.winter_rain_chance, "winter_rain_chance")?,
                checked_chance(self.winter_snow_chance, "winter_snow_chance")?,
                checked_chance(self.winter_storm_chance, "winter_storm_chance")?,
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

        Ok((zone, chances))
    }
}

fn checked_chance(value: i64, field: &str) -> anyhow::Result<u32> {
    u32::try_from(value).map_err(|_| anyhow::anyhow!("game_weather.{field} must fit u32"))
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
