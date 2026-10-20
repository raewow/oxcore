//! VMapManager - manages VMap loading and spatial queries for world
//! Static geometry is shared; runtime collision belongs to a map instance.

use super::bsp_tree::{BSPModelInstance, BSPTree};
use super::dynamic_tree::DynamicMapTree;
use super::file_loader::{VMapFileLoader, WorldModelData};
use super::go_model_list::{load_gameobject_model_list, GameObjectModelData};
use super::model_transform::{place_static_model, place_world_model};
use super::types::{
    wmo_liquid_type_mask, BoundingBox, ModelInstance, ModelType, VMapConfig, VMapLoadResult,
    WmoLiquidInfo, VMAP_INVALID_HEIGHT_VALUE,
};
use oxcore_shared::protocol::Position;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
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

/// One tile's static collision (or a non-tiled map's global model), with the bounds of its
/// models so a query can skip the trees it cannot touch.
struct StaticTree {
    bounds: BoundingBox,
    tree: BSPTree,
}

impl StaticTree {
    fn bounds_of(models: &[Arc<BSPModelInstance>]) -> Option<BoundingBox> {
        let first = models.first()?.bounding_box;
        Some(models.iter().fold(first, |mut b, m| {
            let mb = &m.bounding_box;
            b.min = Position::new(
                b.min.x.min(mb.min.x),
                b.min.y.min(mb.min.y),
                b.min.z.min(mb.min.z),
                0.0,
            );
            b.max = Position::new(
                b.max.x.max(mb.max.x),
                b.max.y.max(mb.max.y),
                b.max.z.max(mb.max.z),
                0.0,
            );
            b
        }))
    }

    /// Whether a vertical query at (x, y) can meet any of this tree's models.
    fn reaches_column(&self, x: f32, y: f32) -> bool {
        x >= self.bounds.min.x
            && x <= self.bounds.max.x
            && y >= self.bounds.min.y
            && y <= self.bounds.max.y
    }

    /// Whether a segment's bounding box overlaps this tree's models.
    fn reaches_segment(&self, from: &Position, to: &Position) -> bool {
        from.x.max(to.x) >= self.bounds.min.x
            && from.x.min(to.x) <= self.bounds.max.x
            && from.y.max(to.y) >= self.bounds.min.y
            && from.y.min(to.y) <= self.bounds.max.y
            && from.z.max(to.z) >= self.bounds.min.z
            && from.z.min(to.z) <= self.bounds.max.z
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
    /// Loaded `.vmtree` files: the map's global model when it is not tiled.
    loaded_trees: RwLock<HashMap<u32, Option<ModelInstance>>>,
    /// Loaded tiles (map_id -> (tile_x, tile_y))
    loaded_tiles: RwLock<HashMap<u32, HashSet<(u32, u32)>>>,
    /// Static collision per map: one tree per loaded tile, plus a non-tiled map's global model.
    ///
    /// Loading a tile builds only that tile's tree. A single tree per map had to be rebuilt from
    /// every loaded tile each time one more loaded, which grew without bound as a continent
    /// filled in. Each model lives in exactly one tree, so a query asks every tree it can reach.
    static_trees: RwLock<HashMap<u32, Vec<StaticTree>>>,
    /// Runtime gameobject collision, keyed by (map_id, instance_id).
    dynamic_trees: RwLock<HashMap<(u32, u32), Arc<DynamicMapTree>>>,
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
            static_trees: RwLock::new(HashMap::new()),
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
                Ok(tree_data) => {
                    self.add_static_tree(
                        map_id,
                        self.place_spawns(tree_data.global_model.as_slice()),
                    );
                    self.loaded_trees
                        .write()
                        .insert(map_id, tree_data.global_model);
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
        let map_tiles = tiles.entry(map_id).or_default();

        if !map_tiles.contains(&tile_key) {
            match self.file_loader.load_map_tile(map_id, tile_x, tile_y) {
                Ok(Some(tile_data)) => {
                    map_tiles.insert(tile_key);
                    self.add_static_tree(map_id, self.place_spawns(&tile_data.model_instances));
                }
                Ok(None) => {
                    // Tile doesn't exist - normal for tiles without VMap data
                    map_tiles.insert(tile_key);
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

        VMapLoadResult::Ok
    }

    /// Build a tree from one batch of placed models and add it to the map's static collision.
    fn add_static_tree(&self, map_id: u32, models: Vec<Arc<BSPModelInstance>>) {
        let Some(bounds) = StaticTree::bounds_of(&models) else {
            return;
        };
        debug!(
            "Building BSP tree for map {} from {} models",
            map_id,
            models.len()
        );
        let mut tree = BSPTree::new(map_bounds());
        tree.build(models);
        self.static_trees
            .write()
            .entry(map_id)
            .or_default()
            .push(StaticTree { bounds, tree });
    }

    /// Place static model spawns in the world, reading each distinct model file once.
    fn place_spawns(&self, spawns: &[ModelInstance]) -> Vec<Arc<BSPModelInstance>> {
        let mut models: HashMap<&str, Option<WorldModelData>> = HashMap::new();
        let mut placed = Vec::new();
        for spawn in spawns {
            let model = models.entry(&spawn.model_name).or_insert_with(|| {
                self.file_loader
                    .load_world_model(&spawn.model_name)
                    .map_err(|e| debug!("Failed to load world model '{}': {}", spawn.model_name, e))
                    .ok()
            });
            if let Some(model) = model {
                placed.extend(place_static_model(model, spawn));
            }
        }
        placed
    }

    /// Check line of sight between two points.
    /// Returns true if there is a clear line of sight.
    pub fn is_in_line_of_sight(
        &self,
        map_id: u32,
        instance_id: u32,
        from: Position,
        to: Position,
    ) -> bool {
        self.is_in_line_of_sight_filtered(map_id, instance_id, from, to, false)
    }

    /// Check line of sight, optionally ignoring M2 doodad geometry.
    ///
    /// Both static world geometry and spawned gameobject models are consulted.
    pub fn is_in_line_of_sight_filtered(
        &self,
        map_id: u32,
        instance_id: u32,
        from: Position,
        to: Position,
        ignore_m2: bool,
    ) -> bool {
        if !self.config.enable_los {
            return true;
        }

        // Static world geometry. No VMap loaded means we assume a clear view.
        if let Some(trees) = self.static_trees.read().get(&map_id) {
            if trees.iter().any(|t| {
                t.reaches_segment(&from, &to) && t.tree.raycast_with_filter(&from, &to, ignore_m2)
            }) {
                return false;
            }
        }

        // Spawned gameobjects (closed doors, drawbridges, ...).
        if let Some(dyn_tree) = self.dynamic_tree(map_id, instance_id) {
            if dyn_tree.raycast(&from, &to, ignore_m2) {
                return false;
            }
        }

        true
    }

    /// Get ground height at a position.
    /// Returns None if no valid height found.
    pub fn get_height(&self, map_id: u32, instance_id: u32, x: f32, y: f32, z: f32) -> Option<f32> {
        self.get_height_within(map_id, instance_id, x, y, z, 50.0)
    }

    /// Query shared static geometry without consulting any instance's gameobjects.
    pub fn get_static_height_within(
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
        let trees = self.static_trees.read();
        let mut best: Option<f32> = None;
        for t in trees.get(&map_id).into_iter().flatten() {
            if !t.reaches_column(x, y) {
                continue;
            }
            if let Some(h) = t.tree.get_height(&pos, max_search_dist) {
                // The surface nearest the query, as a single tree picks between its halves.
                if best.is_none_or(|b| (z - h).abs() < (z - b).abs()) {
                    best = Some(h);
                }
            }
        }
        best
    }

    /// Get ground height at a position, searching at most `max_search_dist`.
    ///
    /// Considers both static geometry and spawned gameobject models, returning
    /// whichever surface is nearer to `z`.
    pub fn get_height_within(
        &self,
        map_id: u32,
        instance_id: u32,
        x: f32,
        y: f32,
        z: f32,
        max_search_dist: f32,
    ) -> Option<f32> {
        if !self.config.enable_height {
            return None;
        }

        let pos = Position::new(x, y, z, 0.0);

        let static_height = self.get_static_height_within(map_id, x, y, z, max_search_dist);

        let dynamic_height = self
            .dynamic_tree(map_id, instance_id)
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

        let trees = self.static_trees.read();
        trees
            .get(&map_id)
            .into_iter()
            .flatten()
            .any(|t| t.reaches_column(pos.x, pos.y) && t.tree.get_area_info(&pos).is_some())
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
        let trees = self.static_trees.read();
        let pos = Position::new(x, y, z, 0.0);
        let liquid = trees
            .get(&map_id)?
            .iter()
            .filter(|t| t.reaches_column(x, y))
            .find_map(|t| t.tree.get_liquid_level(&pos, req_liquid_type))?;

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

    /// Add a spawned gameobject's collision model to its map instance's dynamic tree.
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
        instance_id: u32,
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

        self.dynamic_tree_or_create(map_id, instance_id)
            .insert(id, groups, enabled);

        debug!(
            "VMap: inserted gameobject model '{}' (display {}) on map {} instance {}",
            model_data.name, spawn.display_id, map_id, instance_id
        );

        true
    }

    /// Remove a gameobject's collision model. Returns whether one was present.
    pub fn remove_gameobject_model(&self, map_id: u32, instance_id: u32, id: u64) -> bool {
        match self.dynamic_tree(map_id, instance_id) {
            Some(tree) => tree.remove(id),
            None => false,
        }
    }

    /// Whether a gameobject's collision model is tracked for a map instance.
    pub fn contains_gameobject_model(&self, map_id: u32, instance_id: u32, id: u64) -> bool {
        self.dynamic_tree(map_id, instance_id)
            .map(|tree| tree.contains(id))
            .unwrap_or(false)
    }

    /// Enable or disable a gameobject's collision, e.g. when a door opens.
    ///
    /// Returns whether a model with that id exists.
    pub fn set_gameobject_model_enabled(
        &self,
        map_id: u32,
        instance_id: u32,
        id: u64,
        enabled: bool,
    ) -> bool {
        match self.dynamic_tree(map_id, instance_id) {
            Some(tree) => tree.set_enabled(id, enabled),
            None => false,
        }
    }

    /// Number of dynamic models tracked for a map instance.
    pub fn dynamic_model_count(&self, map_id: u32, instance_id: u32) -> usize {
        self.dynamic_tree(map_id, instance_id)
            .map(|t| t.len())
            .unwrap_or(0)
    }

    fn dynamic_tree(&self, map_id: u32, instance_id: u32) -> Option<Arc<DynamicMapTree>> {
        self.dynamic_trees
            .read()
            .get(&(map_id, instance_id))
            .cloned()
    }

    fn dynamic_tree_or_create(&self, map_id: u32, instance_id: u32) -> Arc<DynamicMapTree> {
        if let Some(tree) = self.dynamic_tree(map_id, instance_id) {
            return tree;
        }

        let mut trees = self.dynamic_trees.write();
        Arc::clone(
            trees
                .entry((map_id, instance_id))
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

    /// Remove one instance's runtime collision without unloading shared geometry.
    pub fn unload_instance(&self, map_id: u32, instance_id: u32) {
        if let Some(tree) = self.dynamic_trees.write().remove(&(map_id, instance_id)) {
            tree.clear();
        }
    }

    /// Unload shared VMap data and every instance's collision for this map.
    pub fn unload_map(&self, map_id: u32) {
        self.static_trees.write().remove(&map_id);
        self.loaded_tiles.write().remove(&map_id);
        self.loaded_trees.write().remove(&map_id);
        self.dynamic_trees.write().retain(|&(owner_map, _), tree| {
            if owner_map == map_id {
                tree.clear();
                false
            } else {
                true
            }
        });
        info!("Unloaded VMap for map {}", map_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfinding::vmap::file_loader::Triangle;
    use crate::pathfinding::{
        game_pathfinder::GamePathFinder, mmap_manager::MMapManager, pathfinder::PathFinder,
        types::PathResult,
    };
    use crate::terrain::defines::MAP_ALL_LIQUIDS;
    use crate::terrain::TerrainInfo;

    fn manager() -> VMapManager {
        VMapManager::new("/nonexistent-data-dir", VMapConfig::default())
    }

    fn floor(z: f32) -> Vec<Arc<BSPModelInstance>> {
        vec![Arc::new(BSPModelInstance {
            model_id: 1,
            model_type: ModelType::WMO,
            // Keep the query origin inside the bounds to isolate ownership
            // from the separate floor-query origin/bounds defect.
            bounding_box: BoundingBox {
                min: Position::new(-5.0, -5.0, 0.0, 0.0),
                max: Position::new(5.0, 5.0, 10.0, 0.0),
            },
            triangles: vec![Triangle {
                v0: Position::new(-5.0, -5.0, z, 0.0),
                v1: Position::new(5.0, -5.0, z, 0.0),
                v2: Position::new(0.0, 5.0, z, 0.0),
            }],
            liquid_data: None,
            footprint: Default::default(),
        })]
    }

    fn add_static_floor(mgr: &VMapManager, map_id: u32) {
        mgr.add_static_tree(map_id, floor(0.0));
    }

    #[test]
    fn dynamic_heights_and_los_are_isolated_from_shared_terrain() {
        let mgr = manager();
        add_static_floor(&mgr, 36);
        mgr.dynamic_tree_or_create(36, 1)
            .insert(7, floor(3.0), true);
        mgr.dynamic_tree_or_create(36, 2)
            .insert(7, floor(6.0), true);
        assert_eq!(mgr.get_height(36, 1, 0.0, 0.0, 7.0), Some(3.0));
        assert_eq!(mgr.get_height(36, 2, 0.0, 0.0, 7.0), Some(6.0));
        assert_eq!(mgr.get_height(36, 0, 0.0, 0.0, 7.0), Some(0.0));

        let terrain = TerrainInfo::new(36, PathBuf::from("/nonexistent/maps"));
        assert_eq!(
            terrain.get_height_static(0.0, 0.0, 7.0, Some(&mgr), 50.0),
            0.0
        );
        let from = Position::new(0.0, 0.0, 1.0, 0.0);
        let to = Position::new(0.0, 0.0, 8.0, 0.0);
        assert!(!mgr.is_in_line_of_sight(36, 1, from, to));
        assert!(!mgr.is_in_line_of_sight(36, 2, from, to));
        assert!(mgr.is_in_line_of_sight(36, 0, from, to));

        assert!(mgr.set_gameobject_model_enabled(36, 1, 7, false));
        assert!(mgr.is_in_line_of_sight_filtered(36, 1, from, to, true));
        assert!(!mgr.is_in_line_of_sight_filtered(36, 2, from, to, true));
        assert_eq!(mgr.get_height(36, 1, 0.0, 0.0, 7.0), Some(0.0));
        assert!(mgr.contains_gameobject_model(36, 1, 7));
        assert!(!mgr.contains_gameobject_model(36, 0, 7));
        assert!(mgr.remove_gameobject_model(36, 1, 7));
        assert!(!mgr.remove_gameobject_model(36, 1, 7));
        assert!(mgr.contains_gameobject_model(36, 2, 7));
        assert_eq!(mgr.get_height(36, 2, 0.0, 0.0, 7.0), Some(6.0));
    }

    #[test]
    fn unloading_collision_respects_map_and_instance_boundaries() {
        let mgr = manager();
        add_static_floor(&mgr, 36);
        for (map_id, instance_id, height) in [(36, 1, 3.0), (36, 2, 6.0), (37, 1, 9.0)] {
            mgr.dynamic_tree_or_create(map_id, instance_id)
                .insert(7, floor(height), true);
        }
        mgr.unload_instance(36, 1);
        assert_eq!(mgr.dynamic_model_count(36, 1), 0);
        assert_eq!(mgr.dynamic_model_count(36, 2), 1);
        assert_eq!(mgr.get_height(36, 1, 0.0, 0.0, 7.0), Some(0.0));
        assert_eq!(mgr.get_height(36, 2, 0.0, 0.0, 7.0), Some(6.0));
        let from = Position::new(0.0, 0.0, -1.0, 0.0);
        let to = Position::new(0.0, 0.0, 1.0, 0.0);
        assert!(
            !mgr.is_in_line_of_sight(36, 1, from, to),
            "shared static floor survives instance unload"
        );

        mgr.unload_map(36);
        assert_eq!(mgr.dynamic_model_count(36, 2), 0);
        assert_eq!(mgr.get_height(36, 2, 0.0, 0.0, 7.0), None);
        assert!(mgr.is_in_line_of_sight(36, 1, from, to));
        assert_eq!(mgr.dynamic_model_count(37, 1), 1);
        assert_eq!(mgr.get_height(37, 1, 0.0, 0.0, 10.0), Some(9.0));
    }

    #[test]
    fn paths_and_shortcuts_use_only_the_requested_instances_collision() {
        let mgr = Arc::new(manager());
        let wall = Arc::new(BSPModelInstance {
            model_id: 1,
            model_type: ModelType::WMO,
            bounding_box: BoundingBox {
                min: Position::new(-5.0, -0.5, 0.0, 0.0),
                max: Position::new(5.0, 0.5, 10.0, 0.0),
            },
            triangles: vec![Triangle {
                v0: Position::new(-5.0, 0.0, 0.0, 0.0),
                v1: Position::new(5.0, 0.0, 0.0, 0.0),
                v2: Position::new(0.0, 0.0, 10.0, 0.0),
            }],
            liquid_data: None,
            footprint: Default::default(),
        });
        mgr.dynamic_tree_or_create(36, 1)
            .insert(7, vec![wall.clone()], true);
        mgr.dynamic_tree_or_create(36, 2)
            .insert(7, vec![wall], false);
        let mmap = Arc::new(MMapManager::new("/nonexistent-data-dir", mgr.clone()));
        let pathfinder = Arc::new(PathFinder::new(mmap));
        let game_pathfinder = GamePathFinder::with_mmap(pathfinder.clone());
        let from = Position::new(0.0, -5.0, 5.0, 0.0);
        let to = Position::new(0.0, 5.0, 5.0, 0.0);
        assert!(!matches!(
            game_pathfinder.calculate_path(36, 1, from, to),
            PathResult::StraightLine(_, _)
        ));
        assert!(matches!(
            game_pathfinder.calculate_path(36, 2, from, to),
            PathResult::StraightLine(_, _)
        ));
        assert!(matches!(
            game_pathfinder.calculate_path(36, 0, from, to),
            PathResult::StraightLine(_, _)
        ));
        let mut closed = vec![from, Position::new(10.0, 0.0, 5.0, 0.0), to];
        let mut open = closed.clone();
        pathfinder.smooth_path(36, 1, &mut closed);
        pathfinder.smooth_path(36, 2, &mut open);
        assert_eq!(closed.len(), 3);
        assert_eq!(open, vec![from, to]);
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
        assert!(!mgr.insert_gameobject_model(0, 0, 42, spawn, true));
        assert!(!mgr.contains_gameobject_model(0, 0, 42));
        assert!(!mgr.remove_gameobject_model(0, 0, 42));
        assert!(!mgr.set_gameobject_model_enabled(0, 0, 42, false));
        assert_eq!(mgr.dynamic_model_count(0, 0), 0);
    }

    #[test]
    fn line_of_sight_is_clear_without_geometry() {
        let mgr = manager();
        let from = Position::new(0.0, 0.0, 0.0, 0.0);
        let to = Position::new(10.0, 10.0, 0.0, 0.0);

        assert!(mgr.is_in_line_of_sight(0, 0, from, to));
        assert!(mgr.is_in_line_of_sight_filtered(0, 0, from, to, true));
    }

    #[test]
    fn height_and_liquid_queries_return_none_without_data() {
        let mgr = manager();

        assert_eq!(mgr.get_height(0, 0, 0.0, 0.0, 0.0), None);
        assert_eq!(mgr.get_height_within(0, 0, 0.0, 0.0, 0.0, 10.0), None);
        assert_eq!(
            mgr.get_liquid_level(0, 0.0, 0.0, 0.0, MAP_ALL_LIQUIDS),
            None
        );
    }

    #[test]
    fn unload_map_is_safe_when_nothing_loaded() {
        let mgr = manager();
        mgr.unload_map(0);
        assert_eq!(mgr.dynamic_model_count(0, 0), 0);
    }
}
