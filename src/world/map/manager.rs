//! Map Manager - manages all map instances

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::world::map::Map;

/// Manages all map instances
pub struct MapManager {
    /// Maps by (map_id, instance_id)
    maps: DashMap<(u32, u32), Arc<Map>>,
    /// Current tick counter for visibility throttling
    current_tick: AtomicU32,
}

impl MapManager {
    pub fn new() -> Self {
        Self {
            maps: DashMap::new(),
            current_tick: AtomicU32::new(0),
        }
    }

    /// Get current tick (for visibility throttling)
    pub fn current_tick(&self) -> u32 {
        self.current_tick.load(Ordering::Relaxed)
    }

    /// Increment and get current tick
    fn increment_tick(&self) -> u32 {
        self.current_tick.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Get or create a map
    pub fn get_or_create_map(&self, map_id: u32, instance_id: u32) -> Arc<Map> {
        let key = (map_id, instance_id);

        if let Some(map) = self.maps.get(&key) {
            return Arc::clone(&map);
        }

        let map = Arc::new(Map::new(map_id, instance_id));
        self.maps.insert(key, Arc::clone(&map));
        map
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

    /// Update all active maps (async for packet sending)
    pub async fn update_all(
        &self,
        diff: std::time::Duration,
        world: &crate::world::World,
    ) -> anyhow::Result<()> {
        // Increment tick counter for visibility throttling
        let current_tick = self.increment_tick();

        // Collect map references to avoid holding iterator across await
        let maps: Vec<Arc<Map>> = self.maps.iter().map(|r| Arc::clone(r.value())).collect();

        // Update all active maps
        for map in maps {
            map.update(diff, current_tick, world).await?;
        }

        Ok(())
    }
}

impl Default for MapManager {
    fn default() -> Self {
        Self::new()
    }
}
