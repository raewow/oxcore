//! Weather state for a single zone (MaNGOS `Weather`).

use super::types::{
    season_name, sounds, WeatherState, WeatherType, WeatherZoneChances, WEATHER_SEASONS,
};
use rand::Rng;
use std::time::Duration;

/// One third of the grade range — the step used when weather gets better/worse.
const GRADE_STEP: f32 = 0.333_333_34;

/// Weather in one zone of one map.
#[derive(Debug, Clone)]
pub struct ZoneWeather {
    zone_id: u32,
    weather_type: WeatherType,
    grade: f32,
    /// Time left before the weather is rolled again.
    timer: Duration,
    /// Interval the timer resets to (config `change_weather_interval`).
    interval: Duration,
    /// Per-season chances from `game_weather`; `None` means the zone always
    /// stays fine.
    chances: Option<WeatherZoneChances>,
    /// Weather set by a GM/script: never regenerates on its own.
    permanent: bool,
}

impl ZoneWeather {
    pub fn new(zone_id: u32, chances: Option<WeatherZoneChances>, interval: Duration) -> Self {
        Self {
            zone_id,
            weather_type: WeatherType::Fine,
            grade: 0.0,
            timer: interval,
            interval,
            chances,
            permanent: false,
        }
    }

    pub fn zone_id(&self) -> u32 {
        self.zone_id
    }

    pub fn weather_type(&self) -> WeatherType {
        self.weather_type
    }

    pub fn grade(&self) -> f32 {
        self.grade
    }

    pub fn is_permanent(&self) -> bool {
        self.permanent
    }

    /// Tick the change timer. Returns true when the weather changed and has to
    /// be pushed to the players in this zone.
    pub fn update(&mut self, diff: Duration, season: usize) -> bool {
        self.timer = self.timer.saturating_sub(diff);
        if !self.timer.is_zero() {
            return false;
        }

        self.timer = self.interval;
        self.regenerate(season)
    }

    /// Force a weather. Returns true when it actually differs from the current
    /// one (and therefore has to be sent to players).
    pub fn set_weather(&mut self, weather_type: WeatherType, grade: f32, permanent: bool) -> bool {
        self.permanent = permanent;

        if self.weather_type == weather_type && self.grade == grade {
            return false;
        }

        self.weather_type = weather_type;
        self.grade = grade;
        true
    }

    /// Roll the next weather. Returns true if and only if the weather changed.
    ///
    /// Distribution (MaNGOS `Weather::ReGenerate`):
    ///   30% no change, 30% better / type change, 30% worse, 10% radical change.
    pub fn regenerate(&mut self, season: usize) -> bool {
        self.regenerate_with(season, &mut rand::thread_rng())
    }

    fn regenerate_with<R: Rng>(&mut self, season: usize, rng: &mut R) -> bool {
        if self.permanent {
            return false;
        }

        let old_type = self.weather_type;
        let old_grade = self.grade;

        let Some(chances) = self.chances else {
            // No chance data for this zone: it is always fine.
            self.weather_type = WeatherType::Fine;
            self.grade = 0.0;
            return old_type != self.weather_type || old_grade != self.grade;
        };

        let u = rng.gen_range(0..100u32);
        if u < 30 {
            return false;
        }

        let season = season % WEATHER_SEASONS;
        tracing::debug!(
            "[WEATHER] Generating a change in {} weather for zone {}",
            season_name(season),
            self.zone_id
        );

        if u < 60 && self.grade < GRADE_STEP {
            // Get fair
            self.weather_type = WeatherType::Fine;
            self.grade = 0.0;
        }

        if u < 60 && self.weather_type != WeatherType::Fine {
            // Get better
            self.grade -= GRADE_STEP;
            self.normalize_grade();
            return true;
        }

        if u < 90 && self.weather_type != WeatherType::Fine {
            // Get worse
            self.grade += GRADE_STEP;
            self.normalize_grade();
            return true;
        }

        if self.weather_type != WeatherType::Fine {
            // Radical change:
            //   light  -> heavy
            //   medium -> change weather type
            //   heavy  -> 50% light, 50% change weather type
            if self.grade < GRADE_STEP {
                self.grade = 0.9999; // go nuts
                return true;
            }

            if self.grade > 0.666_666_7 && rng.gen_range(0..100u32) < 50 {
                self.grade -= 0.666_666_7;
                self.normalize_grade();
                return true;
            }

            self.weather_type = WeatherType::Fine; // clear up
            self.grade = 0.0;
        }

        // Only fine weather remains at this point: roll a new type from the
        // zone's seasonal chances.
        let season_chances = chances.data[season];
        let chance1 = season_chances.rain_chance;
        let chance2 = chance1 + season_chances.snow_chance;
        let chance3 = chance2 + season_chances.storm_chance;

        let roll = rng.gen_range(1..=100u32);
        self.weather_type = if roll <= chance1 {
            WeatherType::Rain
        } else if roll <= chance2 {
            WeatherType::Snow
        } else if roll <= chance3 {
            WeatherType::Storm
        } else {
            WeatherType::Fine
        };

        // New weather intensity (if not fine): 85% light, 7% medium, 7% heavy.
        if self.weather_type == WeatherType::Fine {
            self.grade = 0.0;
        } else if u < 90 {
            self.grade = rng.gen::<f32>() * 0.3333;
        } else if rng.gen_range(0..100u32) < 50 {
            self.grade = rng.gen::<f32>() * 0.3333 + 0.3334;
        } else {
            self.grade = rng.gen::<f32>() * 0.3333 + 0.6667;
        }

        self.normalize_grade();

        self.weather_type != old_type || self.grade != old_grade
    }

    /// Visual state for the current type/grade combination.
    pub fn state(&self) -> WeatherState {
        if self.grade < 0.27 {
            return WeatherState::Fine;
        }

        match self.weather_type {
            WeatherType::Rain => {
                if self.grade < 0.40 {
                    WeatherState::LightRain
                } else if self.grade < 0.70 {
                    WeatherState::MediumRain
                } else {
                    WeatherState::HeavyRain
                }
            }
            WeatherType::Snow => {
                if self.grade < 0.40 {
                    WeatherState::LightSnow
                } else if self.grade < 0.70 {
                    WeatherState::MediumSnow
                } else {
                    WeatherState::HeavySnow
                }
            }
            WeatherType::Storm => {
                if self.grade < 0.40 {
                    WeatherState::LightSandstorm
                } else if self.grade < 0.70 {
                    WeatherState::MediumSandstorm
                } else {
                    WeatherState::HeavySandstorm
                }
            }
            WeatherType::Fine => WeatherState::Fine,
        }
    }

    /// Ambience sound for the current type/grade combination.
    pub fn sound(&self) -> u32 {
        let (light, medium, heavy) = match self.weather_type {
            WeatherType::Rain => (sounds::RAIN_LIGHT, sounds::RAIN_MEDIUM, sounds::RAIN_HEAVY),
            WeatherType::Snow => (sounds::SNOW_LIGHT, sounds::SNOW_MEDIUM, sounds::SNOW_HEAVY),
            WeatherType::Storm => (
                sounds::SANDSTORM_LIGHT,
                sounds::SANDSTORM_MEDIUM,
                sounds::SANDSTORM_HEAVY,
            ),
            WeatherType::Fine => return sounds::NO_SOUND,
        };

        if self.grade < 0.3 {
            sounds::NO_SOUND
        } else if self.grade < 0.6 {
            light
        } else if self.grade < 0.9 {
            medium
        } else {
            heavy
        }
    }

    /// Clamp the grade into the 0..1 range the client accepts.
    pub fn normalize_grade(&mut self) {
        if self.grade >= 1.0 {
            self.grade = 0.9999;
        } else if self.grade < 0.0 {
            self.grade = 0.0001;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::weather::types::{WeatherSeasonChances, WeatherZoneChances};

    fn always_rain() -> WeatherZoneChances {
        WeatherZoneChances {
            data: [WeatherSeasonChances {
                rain_chance: 100,
                snow_chance: 0,
                storm_chance: 0,
            }; WEATHER_SEASONS],
        }
    }

    #[test]
    fn timer_only_fires_after_the_interval() {
        let mut weather = ZoneWeather::new(1, Some(always_rain()), Duration::from_secs(10));
        assert!(!weather.update(Duration::from_secs(9), 0));
        // Not expired yet, so nothing rolled and nothing changed.
        assert_eq!(weather.weather_type(), WeatherType::Fine);

        // Expiring resets the timer regardless of whether the roll changed
        // anything, so the next tick must not fire again immediately.
        weather.update(Duration::from_secs(1), 0);
        assert!(!weather.update(Duration::from_secs(1), 0));
    }

    #[test]
    fn zone_without_chances_stays_fine() {
        let mut weather = ZoneWeather::new(1, None, Duration::from_secs(1));
        assert!(!weather.regenerate(0));
        assert_eq!(weather.weather_type(), WeatherType::Fine);
        assert_eq!(weather.grade(), 0.0);
    }

    #[test]
    fn permanent_weather_never_regenerates() {
        let mut weather = ZoneWeather::new(1, Some(always_rain()), Duration::from_secs(1));
        assert!(weather.set_weather(WeatherType::Snow, 0.9, true));

        for _ in 0..50 {
            assert!(!weather.regenerate(0));
        }
        assert_eq!(weather.weather_type(), WeatherType::Snow);
        assert_eq!(weather.grade(), 0.9);
    }

    #[test]
    fn set_weather_reports_only_real_changes() {
        let mut weather = ZoneWeather::new(1, Some(always_rain()), Duration::from_secs(1));
        assert!(weather.set_weather(WeatherType::Rain, 0.5, false));
        assert!(!weather.set_weather(WeatherType::Rain, 0.5, false));
        assert!(weather.set_weather(WeatherType::Rain, 0.8, false));
    }

    #[test]
    fn grade_stays_in_client_range() {
        let mut weather = ZoneWeather::new(1, Some(always_rain()), Duration::from_secs(1));
        for _ in 0..2000 {
            weather.regenerate(0);
            assert!(
                weather.grade() >= 0.0 && weather.grade() < 1.0,
                "grade out of range: {}",
                weather.grade()
            );
        }
    }

    #[test]
    fn state_and_sound_follow_grade() {
        let mut weather = ZoneWeather::new(1, Some(always_rain()), Duration::from_secs(1));

        weather.set_weather(WeatherType::Rain, 0.1, false);
        assert_eq!(weather.state(), WeatherState::Fine);
        assert_eq!(weather.sound(), sounds::NO_SOUND);

        weather.set_weather(WeatherType::Rain, 0.35, false);
        assert_eq!(weather.state(), WeatherState::LightRain);
        assert_eq!(weather.sound(), sounds::RAIN_LIGHT);

        weather.set_weather(WeatherType::Snow, 0.65, false);
        assert_eq!(weather.state(), WeatherState::MediumSnow);
        assert_eq!(weather.sound(), sounds::SNOW_MEDIUM);

        weather.set_weather(WeatherType::Storm, 0.95, false);
        assert_eq!(weather.state(), WeatherState::HeavySandstorm);
        assert_eq!(weather.sound(), sounds::SANDSTORM_HEAVY);

        weather.set_weather(WeatherType::Fine, 0.0, false);
        assert_eq!(weather.state(), WeatherState::Fine);
        assert_eq!(weather.sound(), sounds::NO_SOUND);
    }

    #[test]
    fn rain_zone_eventually_rains() {
        let mut weather = ZoneWeather::new(1, Some(always_rain()), Duration::from_secs(1));
        let mut saw_rain = false;
        for _ in 0..500 {
            weather.regenerate(0);
            if weather.weather_type() == WeatherType::Rain {
                saw_rain = true;
                break;
            }
        }
        assert!(saw_rain, "a zone with 100% rain chance never rained");
    }
}
