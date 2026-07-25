//! Map Manager - manages all map instances

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::map::MapConfig;
use crate::Map;

/// Supplies the tuning for a map the first time it is created.
///
/// Maps are created lazily from dozens of call sites that have no access to DBC
/// or server config, so rather than threading a `MapConfig` through all of them
/// the world layer installs a resolver once at startup.
pub trait MapConfigProvider: Send + Sync {
    fn config_for(&self, map_id: u32, instance_id: u32) -> MapConfig;
}

impl<F> MapConfigProvider for F
where
    F: Fn(u32, u32) -> MapConfig + Send + Sync,
{
    fn config_for(&self, map_id: u32, instance_id: u32) -> MapConfig {
        self(map_id, instance_id)
    }
}

/// Manages all map instances
pub struct MapManager {
    /// Maps by (map_id, instance_id)
    maps: DashMap<(u32, u32), Arc<Map>>,
    /// Current tick counter for visibility throttling
    current_tick: AtomicU32,
    /// Resolves the config for a map on first creation.
    config_provider: RwLock<Option<Arc<dyn MapConfigProvider>>>,
}

impl MapManager {
    pub fn new() -> Self {
        Self {
            maps: DashMap::new(),
            current_tick: AtomicU32::new(0),
            config_provider: RwLock::new(None),
        }
    }

    /// Install the resolver used for maps created from here on.
    pub fn set_config_provider(&self, provider: Arc<dyn MapConfigProvider>) {
        *self.config_provider.write() = Some(provider);
    }

    /// Tuning for a map, or the default when no resolver is installed.
    fn config_for(&self, map_id: u32, instance_id: u32) -> MapConfig {
        match self.config_provider.read().as_ref() {
            Some(provider) => provider.config_for(map_id, instance_id),
            None => {
                tracing::debug!(
                    "MapManager: no config provider installed; map {}:{} uses defaults",
                    map_id,
                    instance_id
                );
                MapConfig::default()
            }
        }
    }

    /// Get current tick (for visibility throttling)
    pub fn current_tick(&self) -> u32 {
        self.current_tick.load(Ordering::Relaxed)
    }

    /// Increment and get current tick
    pub fn increment_tick(&self) -> u32 {
        self.current_tick.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Get or create a map
    pub fn get_or_create_map(&self, map_id: u32, instance_id: u32) -> Arc<Map> {
        let key = (map_id, instance_id);

        if let Some(map) = self.maps.get(&key) {
            return Arc::clone(&map);
        }

        // Resolve the config outside the entry lock: the provider is world-side
        // code and must not run while a map shard is held.
        let config = self.config_for(map_id, instance_id);

        Arc::clone(
            self.maps
                .entry(key)
                .or_insert_with(|| Arc::new(Map::with_config(map_id, instance_id, config)))
                .value(),
        )
    }

    /// Get a map if it exists
    pub fn get_map(&self, map_id: u32, instance_id: u32) -> Option<Arc<Map>> {
        self.maps
            .get(&(map_id, instance_id))
            .map(|r| Arc::clone(&r))
    }

    /// Get a continent (instance_id = 0)
    pub fn get_continent(&self, map_id: u32) -> Arc<Map> {
        self.get_or_create_map(map_id, 0)
    }

    /// Number of active maps
    pub fn map_count(&self) -> usize {
        self.maps.len()
    }

    /// Get all active map keys (map_id, instance_id)
    pub fn get_active_map_keys(&self) -> Vec<(u32, u32)> {
        self.maps.iter().map(|r| *r.key()).collect()
    }

    /// Snapshot of all active maps.
    pub fn active_maps(&self) -> Vec<Arc<Map>> {
        self.maps.iter().map(|r| Arc::clone(r.value())).collect()
    }
}

impl Default for MapManager {
    fn default() -> Self {
        Self::new()
    }
}
