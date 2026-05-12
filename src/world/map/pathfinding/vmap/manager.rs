//! VMapManager - manages VMap loading and spatial queries for world
//! Simplified from the old world/ VMapManager (no instance_id, Position-based API)

use crate::shared::protocol::Position;
use super::bsp_tree::{BSPModelInstance, BSPTree};
use super::file_loader::{MapTileData, VMapFileLoader};
use super::types::{BoundingBox, LiquidLevel, VMapConfig, VMapLoadResult, VMAP_INVALID_HEIGHT_VALUE};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

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

        Self {
            config,
            base_path: vmap_path.clone(),
            file_loader: VMapFileLoader::new(&vmap_path),
            loaded,
            loaded_trees: RwLock::new(HashMap::new()),
            loaded_tiles: RwLock::new(HashMap::new()),
            bsp_trees: RwLock::new(HashMap::new()),
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
        let bounds = BoundingBox {
            min: Position::new(-17066.0, -17066.0, -1000.0, 0.0),
            max: Position::new(17066.0, 17066.0, 1000.0, 0.0),
        };
        let mut tree = BSPTree::new(bounds);

        let mut bsp_models = Vec::new();
        let tiles = self.loaded_tiles.read();
        if let Some(map_tiles) = tiles.get(&map_id) {
            for (_tile_key, tile_data) in map_tiles.iter() {
                for model_instance in &tile_data.model_instances {
                    match self.file_loader.load_world_model(&model_instance.model_name) {
                        Ok(world_model) => {
                            for group in &world_model.groups {
                                let mut triangles = group.triangles.clone();

                                // Transform triangles: scale -> rotate -> translate
                                for triangle in &mut triangles {
                                    // Apply scale
                                    triangle.v0.x *= model_instance.scale;
                                    triangle.v0.y *= model_instance.scale;
                                    triangle.v0.z *= model_instance.scale;
                                    triangle.v1.x *= model_instance.scale;
                                    triangle.v1.y *= model_instance.scale;
                                    triangle.v1.z *= model_instance.scale;
                                    triangle.v2.x *= model_instance.scale;
                                    triangle.v2.y *= model_instance.scale;
                                    triangle.v2.z *= model_instance.scale;

                                    // Apply rotation around Z-axis
                                    let orientation = model_instance.position.o;
                                    if orientation != 0.0 {
                                        let cos_o = orientation.cos();
                                        let sin_o = orientation.sin();

                                        let (v0_x, v0_y) = (triangle.v0.x, triangle.v0.y);
                                        triangle.v0.x = v0_x * cos_o - v0_y * sin_o;
                                        triangle.v0.y = v0_x * sin_o + v0_y * cos_o;

                                        let (v1_x, v1_y) = (triangle.v1.x, triangle.v1.y);
                                        triangle.v1.x = v1_x * cos_o - v1_y * sin_o;
                                        triangle.v1.y = v1_x * sin_o + v1_y * cos_o;

                                        let (v2_x, v2_y) = (triangle.v2.x, triangle.v2.y);
                                        triangle.v2.x = v2_x * cos_o - v2_y * sin_o;
                                        triangle.v2.y = v2_x * sin_o + v2_y * cos_o;
                                    }

                                    // Apply position offset
                                    triangle.v0.x += model_instance.position.x;
                                    triangle.v0.y += model_instance.position.y;
                                    triangle.v0.z += model_instance.position.z;
                                    triangle.v1.x += model_instance.position.x;
                                    triangle.v1.y += model_instance.position.y;
                                    triangle.v1.z += model_instance.position.z;
                                    triangle.v2.x += model_instance.position.x;
                                    triangle.v2.y += model_instance.position.y;
                                    triangle.v2.z += model_instance.position.z;
                                }

                                // Transform bounding box
                                let mut bbox = BoundingBox {
                                    min: Position::new(
                                        group.bounding_box.min.x * model_instance.scale,
                                        group.bounding_box.min.y * model_instance.scale,
                                        group.bounding_box.min.z * model_instance.scale,
                                        0.0,
                                    ),
                                    max: Position::new(
                                        group.bounding_box.max.x * model_instance.scale,
                                        group.bounding_box.max.y * model_instance.scale,
                                        group.bounding_box.max.z * model_instance.scale,
                                        0.0,
                                    ),
                                };

                                let orientation = model_instance.position.o;
                                if orientation != 0.0 {
                                    let cos_o = orientation.cos();
                                    let sin_o = orientation.sin();

                                    let corners = [
                                        (bbox.min.x, bbox.min.y, bbox.min.z),
                                        (bbox.max.x, bbox.min.y, bbox.min.z),
                                        (bbox.min.x, bbox.max.y, bbox.min.z),
                                        (bbox.max.x, bbox.max.y, bbox.min.z),
                                        (bbox.min.x, bbox.min.y, bbox.max.z),
                                        (bbox.max.x, bbox.min.y, bbox.max.z),
                                        (bbox.min.x, bbox.max.y, bbox.max.z),
                                        (bbox.max.x, bbox.max.y, bbox.max.z),
                                    ];

                                    let mut min_x = f32::INFINITY;
                                    let mut min_y = f32::INFINITY;
                                    let mut min_z = f32::INFINITY;
                                    let mut max_x = f32::NEG_INFINITY;
                                    let mut max_y = f32::NEG_INFINITY;
                                    let mut max_z = f32::NEG_INFINITY;

                                    for (x, y, z) in corners {
                                        let rot_x = x * cos_o - y * sin_o;
                                        let rot_y = x * sin_o + y * cos_o;

                                        min_x = min_x.min(rot_x);
                                        min_y = min_y.min(rot_y);
                                        min_z = min_z.min(z);
                                        max_x = max_x.max(rot_x);
                                        max_y = max_y.max(rot_y);
                                        max_z = max_z.max(z);
                                    }

                                    bbox = BoundingBox {
                                        min: Position::new(min_x, min_y, min_z, 0.0),
                                        max: Position::new(max_x, max_y, max_z, 0.0),
                                    };
                                }

                                bbox.min.x += model_instance.position.x;
                                bbox.min.y += model_instance.position.y;
                                bbox.min.z += model_instance.position.z;
                                bbox.max.x += model_instance.position.x;
                                bbox.max.y += model_instance.position.y;
                                bbox.max.z += model_instance.position.z;

                                let liquid_data =
                                    group.liquid_data.as_ref().map(|ld| LiquidLevel {
                                        level: ld.level + model_instance.position.z,
                                        floor: ld.floor + model_instance.position.z,
                                        liquid_type: ld.liquid_type,
                                    });

                                bsp_models.push(Arc::new(BSPModelInstance {
                                    model_id: model_instance.model_id,
                                    model_type: model_instance.model_type,
                                    bounding_box: bbox,
                                    triangles,
                                    liquid_data,
                                }));
                            }
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
    pub fn is_in_line_of_sight(
        &self,
        map_id: u32,
        from: Position,
        to: Position,
    ) -> bool {
        if !self.config.enable_los {
            return true;
        }

        let trees = self.bsp_trees.read();
        if let Some(tree) = trees.get(&map_id) {
            // Raycast returns true if hit (blocked), so invert for LOS
            !tree.raycast(&from, &to)
        } else {
            true // No VMap loaded, assume clear
        }
    }

    /// Get ground height at a position.
    /// Returns None if no valid height found.
    pub fn get_height(&self, map_id: u32, x: f32, y: f32, z: f32) -> Option<f32> {
        if !self.config.enable_height {
            return None;
        }

        let trees = self.bsp_trees.read();
        if let Some(tree) = trees.get(&map_id) {
            let pos = Position::new(x, y, z, 0.0);
            tree.get_height(&pos, 50.0) // 50 unit search distance
        } else {
            None
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
        info!("Unloaded VMap for map {}", map_id);
    }
}
