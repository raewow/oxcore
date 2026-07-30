//! Per-map zone weather, hosted on `Map`.
//!
//! Weather states live in this system keyed by `(map_id, instance_id, zone_id)`.

use super::manager::WeatherManager;
use super::types::{season_for_yday, WeatherType};
use super::zone_weather::ZoneWeather;
use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::World;
use chrono::{Datelike, Local};
use dashmap::DashMap;
use oxcore_shared::messages::weather::SmsgWeather;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;
use std::time::Duration;

/// Fallback when `change_weather_interval` is unset (10 minutes).
const DEFAULT_CHANGE_INTERVAL: Duration = Duration::from_millis(10 * 60 * 1000);

/// Key of a zone weather state: map, instance and zone.
type WeatherKey = (u32, u32, u32);

pub struct WeatherSystem {
    manager: Arc<WeatherManager>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    weathers: DashMap<WeatherKey, ZoneWeather>,
}

impl WeatherSystem {
    pub fn new(
        manager: Arc<WeatherManager>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    ) -> Self {
        Self {
            manager,
            broadcast_mgr,
            weathers: DashMap::new(),
        }
    }

    pub fn manager(&self) -> &Arc<WeatherManager> {
        &self.manager
    }

    /// Tick every live zone weather; push changes to the players standing in
    /// the zone. Zones that ran dry of players drop their state.
    pub fn update(&self, diff: Duration, world: &World) -> anyhow::Result<()> {
        if !Self::enabled(world) {
            return Ok(());
        }

        let season = current_season();
        let mut empty_zones = Vec::new();

        for mut entry in self.weathers.iter_mut() {
            if !entry.value_mut().update(diff, season) {
                continue;
            }

            let (map_id, instance_id, zone_id) = *entry.key();
            let players = players_in_zone(world, map_id, instance_id, zone_id);
            if players.is_empty() {
                empty_zones.push(*entry.key());
                continue;
            }

            let weather = entry.value();
            tracing::debug!(
                "[WEATHER] Zone {} on map {}:{} changed to {} (type {}, grade {:.2})",
                zone_id,
                map_id,
                instance_id,
                weather.state().name(),
                weather.weather_type().name(),
                weather.grade()
            );

            self.send_to(&players, weather);
        }

        for key in empty_zones {
            self.weathers.remove(&key);
        }

        Ok(())
    }

    /// Send the current weather of the player's zone to that one player.
    ///
    /// Called on login and whenever the player enters a new zone.
    pub fn send_weather_to_player(&self, player_guid: ObjectGuid, world: &World) {
        if !Self::enabled(world) {
            return;
        }

        let Some((map_id, instance_id, zone_id)) = world
            .managers
            .player_mgr
            .with_player(player_guid, |p| (p.map_id, p.instance_id, p.zone_id))
        else {
            return;
        };

        if zone_id == 0 {
            return;
        }

        let entry = self.find_or_create(map_id, instance_id, zone_id, world);
        self.broadcast_mgr
            .send_msg_to_player(player_guid, weather_packet(entry.value()));
    }

    /// Force the weather of a zone (GM `.wchange`, scripts).
    ///
    /// `permanent` weather stops regenerating until it is set again.
    pub fn set_weather(
        &self,
        map_id: u32,
        instance_id: u32,
        zone_id: u32,
        weather_type: WeatherType,
        grade: f32,
        permanent: bool,
        world: &World,
    ) {
        let grade = grade.clamp(0.0, 0.9999);

        let (changed, packet) = {
            let mut entry = self.find_or_create_mut(map_id, instance_id, zone_id, world);
            let weather = entry.value_mut();
            let changed = weather.set_weather(weather_type, grade, permanent);
            (changed, weather_packet(weather))
        };

        if !changed {
            return;
        }

        let players = players_in_zone(world, map_id, instance_id, zone_id);
        for player_guid in players {
            self.broadcast_mgr
                .send_msg_to_player(player_guid, packet.clone());
        }
    }

    /// Current weather of a zone, for spell/aura and script checks.
    ///
    /// Returns `None` when the zone has no weather state yet (no player has
    /// entered it since startup), which callers should read as "fine".
    pub fn get_weather(
        &self,
        map_id: u32,
        instance_id: u32,
        zone_id: u32,
    ) -> Option<(WeatherType, f32)> {
        self.weathers
            .get(&(map_id, instance_id, zone_id))
            .map(|entry| (entry.weather_type(), entry.grade()))
    }

    /// Number of zones currently tracking weather.
    pub fn active_zone_count(&self) -> usize {
        self.weathers.len()
    }

    /// Drop all zone weather states (used when `game_weather` is reloaded).
    pub fn clear(&self) {
        self.weathers.clear();
    }

    fn find_or_create(
        &self,
        map_id: u32,
        instance_id: u32,
        zone_id: u32,
        world: &World,
    ) -> dashmap::mapref::one::Ref<'_, WeatherKey, ZoneWeather> {
        let key = (map_id, instance_id, zone_id);
        if let Some(entry) = self.weathers.get(&key) {
            return entry;
        }

        self.weathers
            .entry(key)
            .or_insert_with(|| self.new_zone_weather(zone_id, world));
        self.weathers
            .get(&key)
            .expect("zone weather was just inserted")
    }

    fn find_or_create_mut(
        &self,
        map_id: u32,
        instance_id: u32,
        zone_id: u32,
        world: &World,
    ) -> dashmap::mapref::one::RefMut<'_, WeatherKey, ZoneWeather> {
        self.weathers
            .entry((map_id, instance_id, zone_id))
            .or_insert_with(|| self.new_zone_weather(zone_id, world))
    }

    fn new_zone_weather(&self, zone_id: u32, world: &World) -> ZoneWeather {
        let interval = change_interval(world);
        tracing::debug!(
            "[WEATHER] Starting weather for zone {} (change every {} minutes)",
            zone_id,
            interval.as_secs() / 60
        );
        ZoneWeather::new(zone_id, self.manager.get_chances(zone_id), interval)
    }

    fn send_to(&self, players: &[ObjectGuid], weather: &ZoneWeather) {
        let packet = weather_packet(weather);
        for &player_guid in players {
            self.broadcast_mgr
                .send_msg_to_player(player_guid, packet.clone());
        }
    }

    fn enabled(world: &World) -> bool {
        world.config.activate_weather.unwrap_or(true)
    }
}

fn weather_packet(weather: &ZoneWeather) -> SmsgWeather {
    SmsgWeather {
        weather_type: weather.weather_type().as_u32(),
        grade: weather.grade(),
        sound_id: weather.sound(),
        // Smooth transitions.
        instant_change: false,
    }
}

fn change_interval(world: &World) -> Duration {
    world
        .config
        .change_weather_interval
        .filter(|ms| *ms > 0)
        .map(|ms| Duration::from_millis(ms as u64))
        .unwrap_or(DEFAULT_CHANGE_INTERVAL)
}

/// Players standing in one zone of one map.
fn players_in_zone(world: &World, map_id: u32, instance_id: u32, zone_id: u32) -> Vec<ObjectGuid> {
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return Vec::new();
    };

    map.player_guids()
        .into_iter()
        .filter(|guid| {
            world
                .managers
                .player_mgr
                .with_player(*guid, |p| p.zone_id == zone_id)
                .unwrap_or(false)
        })
        .collect()
}

/// Season of the current server day (0=spring .. 3=winter).
fn current_season() -> usize {
    season_for_yday(Local::now().ordinal0())
}
