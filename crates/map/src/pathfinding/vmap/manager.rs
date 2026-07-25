//! VMapManager - manages VMap loading and spatial queries for world
//! Simplified from the old world/ VMapManager (no instance_id, Position-based API)

use super::bsp_tree::{BSPModelInstance, BSPTree};
use super::dynamic_tree::DynamicMapTree;
use super::file_loader::{MapTileData, VMapFileLoader};
use super::go_model_list::{load_gameobject_model_list, GameObjectModelData};
use super::model_transform::place_world_model;
use super::types::{
    wmo_liquid_type_mask, BoundingBox, ModelType, VMapConfig, VMapLoadResult, WmoLiquidInfo,
    VMAP_INVALID_HEIGHT_VALUE,
};
use oxcore_shared::protocol::Position;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// World-space bounds covering a whole map, used as the root of every BSP tree.
fn map_bounds() -> BoundingBox {
    BoundingBox {
        min: Position::new(-17066.0, -17066.0, -1000.0, 0.0),
        max: Position::new(17066.0, 17066.0, 1000.0, 0.0),
    }
}

/// A gameobject's collision model to place in the dynamic tree.
#[derive(Debug, Clone, Copy)]
pub struct GameObjectModelSpawn {
    /// GameObjectDisplayInfo display id, used to look up the model file.
    pub display_id: u32,
    /// World position; `o` supplies the rotation about Z.
    pub position: Position,
    /// Object scale.
    pub scale: f32,
    /// Whether the object blocks line of sight even for M2 doodads (doors and
    /// generic objects do). When false, `.m2` models are treated as M2 geometry
    /// and skipped by LoS checks that ignore doodads.
    pub always_break_los: bool,
}

/// VMap manager - handles VMap loading and spatial queries
pub struct VMapManager {
    config: VMapConfig,
    base_path: PathBuf,
    file_loader: VMapFileLoader,
    /// Whether vmaps directory exists
    loaded: bool,
    /// Loaded map trees (map_id -> loaded flag)
    loaded_trees: RwLock<HashMap<u32, bool>>,
    /// Loaded tiles (map_id -> (tile_x, tile_y) -> tile data)
    loaded_tiles: RwLock<HashMap<u32, HashMap<(u32, u32), MapTileData>>>,
    /// BSP trees per map (map_id -> tree)
    bsp_trees: RwLock<HashMap<u32, Arc<BSPTree>>>,
    /// Runtime gameobject collision, per map (map_id -> tree)
    dynamic_trees: RwLock<HashMap<u32, Arc<DynamicMapTree>>>,
    /// Collision model file + bounds per GameObjectDisplayInfo display id
    gameobject_models: HashMap<u32, GameObjectModelData>,
}

impl VMapManager {
    pub fn new(data_dir: impl Into<PathBuf>, config: VMapConfig) -> Self {
        let data_dir = data_dir.into();
        let vmap_path = data_dir.join("vmaps");
        let loaded = vmap_path.exists();

        if loaded {
            info!("VMapManager: vmaps directory found at {:?}", vmap_path);
        } else {
            warn!(
                "VMapManager: vmaps directory not found at {:?}, LOS/height checks will use fallback",
                vmap_path
            );
        }

        let gameobject_models = if loaded {
            load_gameobject_model_list(&vmap_path).unwrap_or_else(|e| {
                warn!("VMapManager: failed to load gameobject model list: {}", e);
                HashMap::new()
            })
        } else {
            HashMap::new()
        };

        Self {
            config,
            base_path: vmap_path.clone(),
            file_loader: VMapFileLoader::new(&vmap_path),
            loaded,
            loaded_trees: RwLock::new(HashMap::new()),
            loaded_tiles: RwLock::new(HashMap::new()),
            bsp_trees: RwLock::new(HashMap::new()),
            dynamic_trees: RwLock::new(HashMap::new()),
            gameobject_models,
        }
    }

    /// Load VMap data for a map tile
    pub fn load_map(&self, map_id: u32, x: i32, y: i32) -> VMapLoadResult {
        if !self.loaded {
            return VMapLoadResult::Ignored;
        }

        if !self.config.enable_los && !self.config.enable_height {
            return VMapLoadResult::Ignored;
        }

        // Load map tree if not loaded
        if !self.loaded_trees.read().contains_key(&map_id) {
            match self.file_loader.load_map_tree(map_id) {
                Ok(_tree_data) => {
                    self.loaded_trees.write().insert(map_id, true);
                }
                Err(e) => {
                    warn!("Failed to load VMap tree for map {}: {}", map_id, e);
                    return VMapLoadResult::Error;
                }
            }
        }

        // Load tile
        let tile_x = x as u32;
        let tile_y = y as u32;
        let tile_key = (tile_x, tile_y);

        let mut tiles = self.loaded_tiles.write();
        let map_tiles = tiles.entry(map_id).or_insert_with(HashMap::new);

        if !map_tiles.contains_key(&tile_key) {
            match self.file_loader.load_map_tile(map_id, tile_x, tile_y) {
                Ok(Some(tile_data)) => {
                    map_tiles.insert(tile_key, tile_data);
                }
                Ok(None) => {
                    // Tile doesn't exist - normal for tiles without VMap data
                    map_tiles.insert(
                        tile_key,
                        MapTileData {
                            map_id,
                            tile_x,
                            tile_y,
                            model_instances: Vec::new(),
                        },
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to load VMap tile {} ({}, {}): {}",
                        map_id, tile_x, tile_y, e
                    );
                    return VMapLoadResult::Ignored;
                }
            }
        }
        drop(tiles);

        // Rebuild BSP tree for this map from all loaded tiles
        self.rebuild_bsp_tree(map_id);

        VMapLoadResult::Ok
    }

    /// Rebuild the BSP tree for a map from all loaded tiles
    fn rebuild_bsp_tree(&self, map_id: u32) {
        let mut tree = BSPTree::new(map_bounds());

        let mut bsp_models = Vec::new();
        let tiles = self.loaded_tiles.read();
        if let Some(map_tiles) = tiles.get(&map_id) {
            for (_tile_key, tile_data) in map_tiles.iter() {
                for model_instance in &tile_data.model_instances {
                    match self
                        .file_loader
                        .load_world_model(&model_instance.model_name)
                    {
                        Ok(world_model) => {
                            bsp_models.extend(place_world_model(
                                &world_model,
                                model_instance.position,
                                model_instance.scale,
                                model_instance.model_id,
                                model_instance.model_type,
                            ));
                        }
                        Err(e) => {
                            debug!(
                                "Failed to load world model '{}': {}",
                                model_instance.model_name, e
                            );
                        }
                    }
                }
            }
        }
        drop(tiles);

        if !bsp_models.is_empty() {
            debug!(
                "Building BSP tree for map {} from {} models",
                map_id,
                bsp_models.len()
            );
            tree.build(bsp_models);
        }

        self.bsp_trees.write().insert(map_id, Arc::new(tree));
    }

    /// Check line of sight between two points.
    /// Returns true if there is a clear line of sight.
    pub fn is_in_line_of_sight(&self, map_id: u32, from: Position, to: Position) -> bool {
        self.is_in_line_of_sight_filtered(map_id, from, to, false)
    }

    /// Check line of sight, optionally ignoring M2 doodad geometry.
    ///
    /// Both static world geometry and spawned gameobject models are consulted.
    pub fn is_in_line_of_sight_filtered(
        &self,
        map_id: u32,
        from: Position,
        to: Position,
        ignore_m2: bool,
    ) -> bool {
        if !self.config.enable_los {
            return true;
        }

        // Static world geometry. No VMap loaded means we assume a clear view.
        if let Some(tree) = self.bsp_trees.read().get(&map_id) {
            if tree.raycast_with_filter(&from, &to, ignore_m2) {
                return false;
            }
        }

        // Spawned gameobjects (closed doors, drawbridges, ...).
        if let Some(dyn_tree) = self.dynamic_tree(map_id) {
            if dyn_tree.raycast(&from, &to, ignore_m2) {
                return false;
            }
        }

        true
    }

    /// Get ground height at a position.
    /// Returns None if no valid height found.
    pub fn get_height(&self, map_id: u32, x: f32, y: f32, z: f32) -> Option<f32> {
        self.get_height_within(map_id, x, y, z, 50.0)
    }

    /// Get ground height at a position, searching at most `max_search_dist`.
    ///
    /// Considers both static geometry and spawned gameobject models, returning
    /// whichever surface is nearer to `z`.
    pub fn get_height_within(
        &self,
        map_id: u32,
        x: f32,
        y: f32,
        z: f32,
        max_search_dist: f32,
    ) -> Option<f32> {
        if !self.config.enable_height {
            return None;
        }

        let pos = Position::new(x, y, z, 0.0);

        let static_height = self
            .bsp_trees
            .read()
            .get(&map_id)
            .and_then(|tree| tree.get_height(&pos, max_search_dist));

        let dynamic_height = self
            .dynamic_tree(map_id)
            .and_then(|tree| tree.get_height(&pos, max_search_dist));

        match (static_height, dynamic_height) {
            (Some(s), Some(d)) => {
                // Standing on an open drawbridge should report the bridge, not
                // the canyon floor: prefer the surface closest to the query z.
                Some(if (z - d).abs() < (z - s).abs() { d } else { s })
            }
            (Some(h), None) | (None, Some(h)) => Some(h),
            (None, None) => None,
        }
    }

    /// Check if position is inside a building/cave
    pub fn is_indoors(&self, map_id: u32, pos: Position) -> bool {
        if !self.config.enable_indoor_check {
            return false;
        }

        let trees = self.bsp_trees.read();
        if let Some(tree) = trees.get(&map_id) {
            tree.get_area_info(&pos).is_some()
        } else {
            false
        }
    }

    /// Get the liquid volume at a position from WMO geometry.
    ///
    /// `req_liquid_type` is a `MAP_LIQUID_TYPE_*` bitmask; pass 0 to accept any
    /// liquid kind. Returns `None` when the position is not inside a matching
    /// WMO liquid volume.
    pub fn get_liquid_level(
        &self,
        map_id: u32,
        x: f32,
        y: f32,
        z: f32,
        req_liquid_type: u32,
    ) -> Option<WmoLiquidInfo> {
        let trees = self.bsp_trees.read();
        let tree = trees.get(&map_id)?;
        let pos = Position::new(x, y, z, 0.0);
        let liquid = tree.get_liquid_level(&pos, req_liquid_type)?;

        Some(WmoLiquidInfo {
            level: liquid.level,
            floor: liquid.floor,
            liquid_type: liquid.liquid_type,
            type_flags: wmo_liquid_type_mask(liquid.liquid_type),
        })
    }

    // ==================== Dynamic gameobject models ====================

    /// Look up the collision model for a display id.
    pub fn gameobject_model(&self, display_id: u32) -> Option<&GameObjectModelData> {
        self.gameobject_models.get(&display_id)
    }

    /// Number of gameobject collision models available.
    pub fn gameobject_model_count(&self) -> usize {
        self.gameobject_models.len()
    }

    /// Add a spawned gameobject's collision model to a map's dynamic tree.
    ///
    /// `id` identifies the owner (its GUID) for later removal. `enabled`
    /// controls whether the model collides immediately — a door spawned open
    /// should be inserted disabled.
    ///
    /// Returns false when the object has no usable collision model, which is the
    /// common case: most gameobjects are not in the extracted model list.
    pub fn insert_gameobject_model(
        &self,
        map_id: u32,
        id: u64,
        spawn: GameObjectModelSpawn,
        enabled: bool,
    ) -> bool {
        if !self.loaded {
            return false;
        }

        let Some(model_data) = self.gameobject_models.get(&spawn.display_id) else {
            return false;
        };

        // A zero-volume box carries no geometry worth testing.
        if !model_data.has_bounds() {
            debug!(
                "VMap: gameobject model '{}' has zero bounds, skipping",
                model_data.name
            );
            return false;
        }

        let world_model = match self.file_loader.load_world_model(&model_data.name) {
            Ok(m) => m,
            Err(e) => {
                debug!(
                    "VMap: failed to load gameobject model '{}': {}",
                    model_data.name, e
                );
                return false;
            }
        };

        // Doors and generic objects always break line of sight; other M2 doodads
        // are tagged so LoS checks that ignore doodads skip them.
        let model_type = if model_data.is_m2() && !spawn.always_break_los {
            ModelType::M2
        } else {
            ModelType::WMO
        };

        let scale = if spawn.scale > 0.0 { spawn.scale } else { 1.0 };
        let groups = place_world_model(
            &world_model,
            spawn.position,
            scale,
            spawn.display_id,
            model_type,
        );

        if groups.is_empty() {
            return false;
        }

        self.dynamic_tree_or_create(map_id)
            .insert(id, groups, enabled);

        debug!(
            "VMap: inserted gameobject model '{}' (display {}) on map {}",
            model_data.name, spawn.display_id, map_id
        );

        true
    }

    /// Remove a gameobject's collision model. Returns whether one was present.
    pub fn remove_gameobject_model(&self, map_id: u32, id: u64) -> bool {
        match self.dynamic_tree(map_id) {
            Some(tree) => tree.remove(id),
            None => false,
        }
    }

    /// Whether a gameobject's collision model is tracked for a map.
    pub fn contains_gameobject_model(&self, map_id: u32, id: u64) -> bool {
        self.dynamic_tree(map_id)
            .map(|tree| tree.contains(id))
            .unwrap_or(false)
    }

    /// Enable or disable a gameobject's collision, e.g. when a door opens.
    ///
    /// Returns whether a model with that id exists.
    pub fn set_gameobject_model_enabled(&self, map_id: u32, id: u64, enabled: bool) -> bool {
        match self.dynamic_tree(map_id) {
            Some(tree) => tree.set_enabled(id, enabled),
            None => false,
        }
    }

    /// Number of dynamic models tracked for a map.
    pub fn dynamic_model_count(&self, map_id: u32) -> usize {
        self.dynamic_tree(map_id).map(|t| t.len()).unwrap_or(0)
    }

    fn dynamic_tree(&self, map_id: u32) -> Option<Arc<DynamicMapTree>> {
        self.dynamic_trees.read().get(&map_id).cloned()
    }

    fn dynamic_tree_or_create(&self, map_id: u32) -> Arc<DynamicMapTree> {
        if let Some(tree) = self.dynamic_tree(map_id) {
            return tree;
        }

        let mut trees = self.dynamic_trees.write();
        Arc::clone(
            trees
                .entry(map_id)
                .or_insert_with(|| Arc::new(DynamicMapTree::new(map_bounds()))),
        )
    }

    /// Check if VMap data is available
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &std::path::Path {
        &self.base_path
    }

    /// Unload VMap data for a map
    pub fn unload_map(&self, map_id: u32) {
        self.bsp_trees.write().remove(&map_id);
        self.loaded_tiles.write().remove(&map_id);
        self.loaded_trees.write().remove(&map_id);
        if let Some(tree) = self.dynamic_trees.write().remove(&map_id) {
            tree.clear();
        }
        info!("Unloaded VMap for map {}", map_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::defines::MAP_ALL_LIQUIDS;

    fn manager() -> VMapManager {
        VMapManager::new("/nonexistent-data-dir", VMapConfig::default())
    }

    #[test]
    fn gameobject_model_api_is_inert_without_vmaps() {
        let mgr = manager();
        let spawn = GameObjectModelSpawn {
            display_id: 1,
            position: Position::new(0.0, 0.0, 0.0, 0.0),
            scale: 1.0,
            always_break_los: true,
        };

        assert_eq!(mgr.gameobject_model_count(), 0);
        assert!(!mgr.insert_gameobject_model(0, 42, spawn, true));
        assert!(!mgr.contains_gameobject_model(0, 42));
        assert!(!mgr.remove_gameobject_model(0, 42));
        assert!(!mgr.set_gameobject_model_enabled(0, 42, false));
        assert_eq!(mgr.dynamic_model_count(0), 0);
    }

    #[test]
    fn line_of_sight_is_clear_without_geometry() {
        let mgr = manager();
        let from = Position::new(0.0, 0.0, 0.0, 0.0);
        let to = Position::new(10.0, 10.0, 0.0, 0.0);

        assert!(mgr.is_in_line_of_sight(0, from, to));
        assert!(mgr.is_in_line_of_sight_filtered(0, from, to, true));
    }

    #[test]
    fn height_and_liquid_queries_return_none_without_data() {
        let mgr = manager();

        assert_eq!(mgr.get_height(0, 0.0, 0.0, 0.0), None);
        assert_eq!(mgr.get_height_within(0, 0.0, 0.0, 0.0, 10.0), None);
        assert_eq!(
            mgr.get_liquid_level(0, 0.0, 0.0, 0.0, MAP_ALL_LIQUIDS),
            None
        );
    }

    #[test]
    fn unload_map_is_safe_when_nothing_loaded() {
        let mgr = manager();
        mgr.unload_map(0);
        assert_eq!(mgr.dynamic_model_count(0), 0);
    }
}
