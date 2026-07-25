//! Weather types, states and chance tables (MaNGOS Weather.h / SharedDefines.h)

/// Weather kind sent to the client in SMSG_WEATHER.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WeatherType {
    Fine = 0,
    Rain = 1,
    Snow = 2,
    Storm = 3,
}

impl WeatherType {
    /// Parse a raw type as used by GM commands and the DB.
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(WeatherType::Fine),
            1 => Some(WeatherType::Rain),
            2 => Some(WeatherType::Snow),
            3 => Some(WeatherType::Storm),
            _ => None,
        }
    }

    pub fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn name(self) -> &'static str {
        match self {
            WeatherType::Fine => "fine",
            WeatherType::Rain => "rain",
            WeatherType::Snow => "snow",
            WeatherType::Storm => "sandstorm",
        }
    }
}

/// Visual state derived from type + grade. Only used for logging in 1.12,
/// but kept because it is the value later clients send on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WeatherState {
    Fine = 0,
    LightRain = 3,
    MediumRain = 4,
    HeavyRain = 5,
    LightSnow = 6,
    MediumSnow = 7,
    HeavySnow = 8,
    LightSandstorm = 22,
    MediumSandstorm = 41,
    HeavySandstorm = 42,
}

impl WeatherState {
    pub fn name(self) -> &'static str {
        match self {
            WeatherState::Fine => "fine",
            WeatherState::LightRain => "light rain",
            WeatherState::MediumRain => "medium rain",
            WeatherState::HeavyRain => "heavy rain",
            WeatherState::LightSnow => "light snow",
            WeatherState::MediumSnow => "medium snow",
            WeatherState::HeavySnow => "heavy snow",
            WeatherState::LightSandstorm => "light sandstorm",
            WeatherState::MediumSandstorm => "medium sandstorm",
            WeatherState::HeavySandstorm => "heavy sandstorm",
        }
    }
}

/// Ambience sound ids (1.12 only).
pub mod sounds {
    pub const NO_SOUND: u32 = 0;
    pub const RAIN_LIGHT: u32 = 8533;
    pub const RAIN_MEDIUM: u32 = 8534;
    pub const RAIN_HEAVY: u32 = 8535;
    pub const SNOW_LIGHT: u32 = 8536;
    pub const SNOW_MEDIUM: u32 = 8537;
    pub const SNOW_HEAVY: u32 = 8538;
    pub const SANDSTORM_LIGHT: u32 = 8556;
    pub const SANDSTORM_MEDIUM: u32 = 8557;
    pub const SANDSTORM_HEAVY: u32 = 8558;
}

/// Number of seasons in `game_weather`.
pub const WEATHER_SEASONS: usize = 4;

/// Per-season chances for one zone (percent, 0-100).
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherSeasonChances {
    pub rain_chance: u32,
    pub snow_chance: u32,
    pub storm_chance: u32,
}

/// Chances for a zone across all four seasons (spring, summer, fall, winter).
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherZoneChances {
    pub data: [WeatherSeasonChances; WEATHER_SEASONS],
}

/// Season index for a day of the year (0=spring, 1=summer, 2=fall, 3=winter).
///
/// 78 days between January 1st and March 20th; 365/4 = 91 days per season.
pub fn season_for_yday(yday: u32) -> usize {
    (((yday as i64 - 78 + 365) / 91) % 4) as usize
}

/// Season name for logging.
pub fn season_name(season: usize) -> &'static str {
    match season {
        0 => "spring",
        1 => "summer",
        2 => "fall",
        _ => "winter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_boundaries_match_mangos() {
        // Seasons are 91-day blocks counted from day 77 (mid March).
        assert_eq!(season_for_yday(77), 0); // spring
        assert_eq!(season_for_yday(167), 0);
        assert_eq!(season_for_yday(168), 1); // summer
        assert_eq!(season_for_yday(258), 1);
        assert_eq!(season_for_yday(259), 2); // fall
        assert_eq!(season_for_yday(349), 2);
        assert_eq!(season_for_yday(350), 3); // winter
                                             // January wraps back into winter.
        assert_eq!(season_for_yday(0), 3);
        assert_eq!(season_for_yday(76), 3);
    }

    #[test]
    fn weather_type_round_trips() {
        for raw in 0..4u32 {
            assert_eq!(WeatherType::from_u32(raw).unwrap().as_u32(), raw);
        }
        assert!(WeatherType::from_u32(4).is_none());
    }
}
