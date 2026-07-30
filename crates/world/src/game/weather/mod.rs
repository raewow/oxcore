//! Zone weather system.
//!
//! Weather is rolled per zone on a timer (`change_weather_interval`) from the
//! seasonal chances in `game_weather`, and pushed to every player standing in
//! the zone with SMSG_WEATHER. GMs and scripts can force a zone's weather,
//! optionally permanently.

pub mod manager;
pub mod repository;
pub mod system;
pub mod types;
pub mod zone_weather;

pub use manager::WeatherManager;
pub use repository::WeatherRepository;
pub use system::WeatherSystem;
pub use types::{
    season_for_yday, WeatherSeasonChances, WeatherState, WeatherType, WeatherZoneChances,
};
pub use zone_weather::ZoneWeather;
