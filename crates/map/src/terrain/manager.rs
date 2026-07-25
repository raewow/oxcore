//! Per-map terrain access: lazily loaded `GridMap`s plus the combined
//! terrain/VMap height and liquid queries.
//!
//! Ported from MaNGOS `TerrainInfo` / `TerrainManager` (GridMap.cpp).

use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, warn};

use super::defines::*;
use super::grid_map::{classify_depth, GridMap};
use crate::pathfinding::vmap::VMapManager;

/// Terrain for one map: the grid cache and the queries built on it.
pub struct TerrainInfo {
    map_id: u32,
    maps_dir: PathBuf,
    /// Loaded grids by `(gx, gy)`. `None` records a grid with no `.map` file so
    /// we do not retry the read on every query.
    grids: DashMap<(usize, usize), Option<Arc<GridMap>>>,
}

impl TerrainInfo {
    pub fn new(map_id: u32, maps_dir: PathBuf) -> Self {
        Self {
            map_id,
            maps_dir,
            grids: DashMap::new(),
        }
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    /// Get the grid covering a position, loading it on first use.
    pub fn get_grid(&self, x: f32, y: f32) -> Option<Arc<GridMap>> {
        let (gx, gy) = terrain_grid_coords(x, y)?;

        if let Some(entry) = self.grids.get(&(gx, gy)) {
            return entry.clone();
        }

        // File name convention is `{map:03}{gy:02}{gx:02}.map` — note the order.
        let path = self
            .maps_dir
            .join(format!("{:03}{:02}{:02}.map", self.map_id, gy, gx));

        let loaded = match GridMap::load(&path) {
            Ok(Some(grid)) => Some(Arc::new(grid)),
            Ok(None) => {
                debug!(
                    "TerrainInfo: no terrain file for map {} grid ({}, {})",
                    self.map_id, gx, gy
                );
                None
            }
            Err(e) => {
                warn!("TerrainInfo: failed to load {:?}: {}", path, e);
                None
            }
        };

        self.grids.insert((gx, gy), loaded.clone());
        loaded
    }

    /// Unload every cached grid for this map.
    pub fn unload(&self) {
        self.grids.clear();
    }

    /// AreaTable *flag* at a position, or 0 when there is no terrain data.
    pub fn get_area_flag(&self, x: f32, y: f32) -> u16 {
        self.get_grid(x, y)
            .map(|g| g.get_area_flag(x, y))
            .unwrap_or(0)
    }

    /// Raw ADT liquid type flags at a position.
    pub fn get_terrain_type(&self, x: f32, y: f32) -> u8 {
        self.get_grid(x, y)
            .map(|g| g.get_terrain_type(x, y))
            .unwrap_or(0)
    }

    /// Ground height combining the ADT height mesh with VMap model surfaces.
    ///
    /// Returns `INVALID_HEIGHT_VALUE` when neither source yields a height.
    ///
    /// Note: unlike the reference this does not perform the upward-looking
    /// retry, because the VMap height query has no signed search direction.
    pub fn get_height_static(
        &self,
        x: f32,
        y: f32,
        z: f32,
        vmap: Option<&VMapManager>,
        max_search_dist: f32,
    ) -> f32 {
        let map_height = self
            .get_grid(x, y)
            .map(|g| g.get_height(x, y))
            .unwrap_or(INVALID_HEIGHT_VALUE);

        // Look from slightly above so a position resting exactly on a surface
        // still finds the floor below it.
        let z2 = z + 2.0;
        let mut vmap_height = INVALID_HEIGHT_VALUE;

        if let Some(vmap) = vmap {
            // If the ADT surface is far below, widen the search so we do not
            // miss a model floor that sits between z and the terrain.
            let mut search_dist = max_search_dist;
            if map_height > INVALID_HEIGHT && z2 - map_height > search_dist {
                search_dist = z2 - map_height + 1.0;
            }

            vmap_height = vmap
                .get_height_within(self.map_id, x, y, z2, search_dist)
                .unwrap_or(INVALID_HEIGHT_VALUE);

            // Not found nearby: retry unbounded for the case of standing far
            // above a model floor but still below the terrain surface.
            if vmap_height <= INVALID_HEIGHT {
                vmap_height = vmap
                    .get_height_within(self.map_id, x, y, z2, 10000.0)
                    .unwrap_or(INVALID_HEIGHT_VALUE);
            }

            // Still nothing: try again from just above the terrain surface.
            if vmap_height <= INVALID_HEIGHT && map_height > INVALID_HEIGHT && z2 < map_height {
                vmap_height = vmap
                    .get_height_within(self.map_id, x, y, map_height + 2.0, DEFAULT_HEIGHT_SEARCH)
                    .unwrap_or(INVALID_HEIGHT_VALUE);
            }
        }

        if vmap_height > INVALID_HEIGHT {
            if map_height > INVALID_HEIGHT {
                // Both available: prefer the model floor when we are already
                // below the terrain, or when it sits above the terrain.
                if z < map_height || vmap_height > map_height {
                    return vmap_height;
                }
                return map_height;
            }
            return vmap_height;
        }

        map_height
    }

    /// Classify a position against the liquid at that point.
    ///
    /// Checks WMO liquid volumes first (they take priority inside buildings and
    /// caves), then falls back to the ADT liquid layer.
    ///
    /// `req_liquid_type` filters by `MAP_LIQUID_TYPE_*`; pass `MAP_ALL_LIQUIDS`
    /// to accept any kind.
    pub fn get_liquid_status(
        &self,
        x: f32,
        y: f32,
        z: f32,
        req_liquid_type: u32,
        vmap: Option<&VMapManager>,
        mut data: Option<&mut LiquidData>,
    ) -> LiquidStatusFlags {
        let ground_level = self.get_height_static(x, y, z, vmap, DEFAULT_WATER_SEARCH);

        // ---- WMO liquid volumes ----
        if let Some(vmap) = vmap {
            if let Some(wmo) = vmap.get_liquid_level(self.map_id, x, y, z, req_liquid_type) {
                // A WMO floor above the liquid surface means the volume does not
                // apply here. The 2 yard slack lets a player standing in a
                // shallow pool still register.
                if wmo.level > wmo.floor && z > wmo.floor - 2.0 {
                    if let Some(ref mut data) = data {
                        data.level = wmo.level;
                        data.depth_level = wmo.floor;
                        data.entry = wmo.liquid_type;
                        data.type_flags = wmo.type_flags | MAP_LIQUID_TYPE_WMO_WATER;
                    }

                    let status = classify_depth(wmo.level, z);
                    if !status.is_empty() {
                        return status;
                    }
                }
            }
        }

        // ---- ADT liquid layer ----
        if let Some(grid) = self.get_grid(x, y) {
            let mut map_data = LiquidData::default();
            let map_status = grid.get_liquid_status(x, y, z, req_liquid_type, Some(&mut map_data));

            // Only accept the ADT answer when its surface is above the floor we
            // computed; do not let a miss here erase an ABOVE_WATER result.
            if !map_status.is_empty() && map_data.level > ground_level {
                if let Some(data) = data {
                    *data = map_data;
                }
                return map_status;
            }
        }

        LiquidStatusFlags::NO_WATER
    }

    /// Whether a position is on or below a liquid surface.
    pub fn is_in_water(&self, x: f32, y: f32, z: f32, vmap: Option<&VMapManager>) -> bool {
        self.get_liquid_status(x, y, z, MAP_ALL_LIQUIDS, vmap, None)
            .intersects(LiquidStatusFlags::MASK_SWIMMING)
    }

    /// Whether a position is fully submerged in water or ocean.
    pub fn is_underwater(&self, x: f32, y: f32, z: f32, vmap: Option<&VMapManager>) -> bool {
        let req = MAP_LIQUID_TYPE_WATER | MAP_LIQUID_TYPE_OCEAN;
        self.get_liquid_status(x, y, z, req, vmap, None)
            .intersects(LiquidStatusFlags::UNDER_WATER)
    }

    /// Surface height of the liquid at a position, or `INVALID_HEIGHT_VALUE`
    /// when there is none.
    pub fn get_water_level(&self, x: f32, y: f32, z: f32, vmap: Option<&VMapManager>) -> f32 {
        let mut data = LiquidData::default();
        let status = self.get_liquid_status(x, y, z, MAP_ALL_LIQUIDS, vmap, Some(&mut data));

        if status.is_empty() {
            INVALID_HEIGHT_VALUE
        } else {
            data.level
        }
    }
}

/// Owns one `TerrainInfo` per map id.
pub struct TerrainManager {
    maps_dir: PathBuf,
    terrains: DashMap<u32, Arc<TerrainInfo>>,
}

impl TerrainManager {
    /// `data_dir` is the server data directory; terrain is read from
    /// `{data_dir}/maps`.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let maps_dir = data_dir.into().join("maps");

        if !maps_dir.exists() {
            warn!(
                "TerrainManager: maps directory not found at {:?}; liquid and \
                 terrain height queries will return no data",
                maps_dir
            );
        }

        Self {
            maps_dir,
            terrains: DashMap::new(),
        }
    }

    /// Get (or create) the terrain for a map.
    pub fn get(&self, map_id: u32) -> Arc<TerrainInfo> {
        if let Some(existing) = self.terrains.get(&map_id) {
            return Arc::clone(&existing);
        }

        let terrain = Arc::new(TerrainInfo::new(map_id, self.maps_dir.clone()));
        self.terrains.insert(map_id, Arc::clone(&terrain));
        terrain
    }

    /// Drop all cached grids for a map.
    pub fn unload_map(&self, map_id: u32) {
        if let Some((_, terrain)) = self.terrains.remove(&map_id) {
            terrain.unload();
        }
    }

    pub fn maps_dir(&self) -> &std::path::Path {
        &self.maps_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_grid_is_cached_as_absent() {
        let terrain = TerrainInfo::new(999, PathBuf::from("/nonexistent/maps"));

        assert!(terrain.get_grid(0.0, 0.0).is_none());
        // The negative result is cached rather than retried.
        assert!(terrain.get_grid(0.0, 0.0).is_none());
        assert_eq!(terrain.grids.len(), 1);
    }

    #[test]
    fn queries_are_inert_without_terrain_data() {
        let terrain = TerrainInfo::new(999, PathBuf::from("/nonexistent/maps"));

        assert_eq!(terrain.get_area_flag(0.0, 0.0), 0);
        assert_eq!(terrain.get_terrain_type(0.0, 0.0), 0);
        assert_eq!(
            terrain.get_height_static(0.0, 0.0, 10.0, None, DEFAULT_HEIGHT_SEARCH),
            INVALID_HEIGHT_VALUE
        );
        assert!(terrain
            .get_liquid_status(0.0, 0.0, 10.0, MAP_ALL_LIQUIDS, None, None)
            .is_empty());
        assert!(!terrain.is_in_water(0.0, 0.0, 10.0, None));
        assert!(!terrain.is_underwater(0.0, 0.0, 10.0, None));
        assert_eq!(
            terrain.get_water_level(0.0, 0.0, 10.0, None),
            INVALID_HEIGHT_VALUE
        );
    }

    #[test]
    fn out_of_bounds_position_has_no_grid() {
        let terrain = TerrainInfo::new(0, PathBuf::from("/nonexistent/maps"));
        assert!(terrain.get_grid(60000.0, 0.0).is_none());
        // Out-of-bounds coordinates are rejected before any caching happens.
        assert_eq!(terrain.grids.len(), 0);
    }

    #[test]
    fn manager_reuses_terrain_per_map() {
        let mgr = TerrainManager::new("/nonexistent");

        let a = mgr.get(0);
        let b = mgr.get(0);
        assert!(Arc::ptr_eq(&a, &b));

        let other = mgr.get(1);
        assert!(!Arc::ptr_eq(&a, &other));

        mgr.unload_map(0);
        let c = mgr.get(0);
        assert!(!Arc::ptr_eq(&a, &c));
    }
}
