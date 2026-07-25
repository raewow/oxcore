//! Collision tree for models that appear and disappear at runtime.
//!
//! Spawned gameobjects with collision (doors, bridges, gates) are not part of
//! the static `.vmtile` geometry, so they live in a per-map dynamic tree that is
//! queried alongside the static one. Ported from MaNGOS `DynamicMapTree`.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::bsp_tree::{BSPModelInstance, BSPTree};
use super::types::BoundingBox;
use oxcore_shared::protocol::Position;

/// One dynamic model: the placed geometry plus whether it currently collides.
struct DynamicModel {
    /// One entry per model group, already transformed into world space.
    groups: Vec<Arc<BSPModelInstance>>,
    /// Cleared while a door stands open so it stops blocking line of sight.
    enabled: bool,
}

/// Per-map tree of runtime collision models, keyed by owner id (a GUID).
///
/// The tree is rebuilt lazily: mutations only mark it dirty, so a burst of
/// spawns during grid load costs one rebuild rather than one per model.
pub struct DynamicMapTree {
    models: RwLock<HashMap<u64, DynamicModel>>,
    tree: RwLock<Option<Arc<BSPTree>>>,
    bounds: BoundingBox,
}

impl DynamicMapTree {
    pub fn new(bounds: BoundingBox) -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            tree: RwLock::new(None),
            bounds,
        }
    }

    /// Insert (or replace) the model owned by `id`.
    pub fn insert(&self, id: u64, groups: Vec<Arc<BSPModelInstance>>, enabled: bool) {
        if groups.is_empty() {
            return;
        }

        self.models
            .write()
            .insert(id, DynamicModel { groups, enabled });
        self.invalidate();
    }

    /// Remove the model owned by `id`. Returns whether one was present.
    pub fn remove(&self, id: u64) -> bool {
        let removed = self.models.write().remove(&id).is_some();
        if removed {
            self.invalidate();
        }
        removed
    }

    pub fn contains(&self, id: u64) -> bool {
        self.models.read().contains_key(&id)
    }

    /// Toggle collision for the model owned by `id`.
    ///
    /// Returns whether the model exists; a no-op change does not rebuild.
    pub fn set_enabled(&self, id: u64, enabled: bool) -> bool {
        let mut models = self.models.write();
        let Some(model) = models.get_mut(&id) else {
            return false;
        };

        if model.enabled == enabled {
            return true;
        }

        model.enabled = enabled;
        drop(models);
        self.invalidate();
        true
    }

    pub fn len(&self) -> usize {
        self.models.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.read().is_empty()
    }

    pub fn clear(&self) {
        self.models.write().clear();
        *self.tree.write() = None;
    }

    /// True if the segment is blocked by an enabled dynamic model.
    pub fn raycast(&self, from: &Position, to: &Position, ignore_m2: bool) -> bool {
        match self.tree() {
            Some(tree) => tree.raycast_with_filter(from, to, ignore_m2),
            None => false,
        }
    }

    /// Height of the nearest enabled dynamic surface, if any.
    pub fn get_height(&self, pos: &Position, max_search_dist: f32) -> Option<f32> {
        self.tree()?.get_height(pos, max_search_dist)
    }

    /// Get the query tree, rebuilding it if a mutation invalidated it.
    ///
    /// Returns `None` when no enabled model is present.
    fn tree(&self) -> Option<Arc<BSPTree>> {
        if let Some(tree) = self.tree.read().clone() {
            return Some(tree);
        }

        let models = self.models.read();
        let instances: Vec<Arc<BSPModelInstance>> = models
            .values()
            .filter(|m| m.enabled)
            .flat_map(|m| m.groups.iter().cloned())
            .collect();
        drop(models);

        if instances.is_empty() {
            return None;
        }

        let mut tree = BSPTree::new(self.bounds);
        tree.build(instances);
        let tree = Arc::new(tree);

        *self.tree.write() = Some(Arc::clone(&tree));
        Some(tree)
    }

    /// Drop the cached tree so the next query rebuilds it.
    fn invalidate(&self) {
        *self.tree.write() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfinding::vmap::file_loader::Triangle;
    use crate::pathfinding::vmap::types::ModelType;

    fn world_bounds() -> BoundingBox {
        BoundingBox {
            min: Position::new(-17066.0, -17066.0, -1000.0, 0.0),
            max: Position::new(17066.0, 17066.0, 1000.0, 0.0),
        }
    }

    /// A vertical wall spanning x in [-5, 5] at y = 0, blocking travel along Y.
    fn wall(model_id: u32) -> Vec<Arc<BSPModelInstance>> {
        vec![Arc::new(BSPModelInstance {
            model_id,
            model_type: ModelType::WMO,
            bounding_box: BoundingBox {
                min: Position::new(-5.0, -0.5, 0.0, 0.0),
                max: Position::new(5.0, 0.5, 10.0, 0.0),
            },
            triangles: vec![
                Triangle {
                    v0: Position::new(-5.0, 0.0, 0.0, 0.0),
                    v1: Position::new(5.0, 0.0, 0.0, 0.0),
                    v2: Position::new(-5.0, 0.0, 10.0, 0.0),
                },
                Triangle {
                    v0: Position::new(5.0, 0.0, 0.0, 0.0),
                    v1: Position::new(5.0, 0.0, 10.0, 0.0),
                    v2: Position::new(-5.0, 0.0, 10.0, 0.0),
                },
            ],
            liquid_data: None,
        })]
    }

    fn across_wall() -> (Position, Position) {
        (
            Position::new(0.0, -5.0, 5.0, 0.0),
            Position::new(0.0, 5.0, 5.0, 0.0),
        )
    }

    #[test]
    fn empty_tree_blocks_nothing() {
        let tree = DynamicMapTree::new(world_bounds());
        let (from, to) = across_wall();

        assert!(tree.is_empty());
        assert!(!tree.raycast(&from, &to, false));
        assert!(tree.get_height(&from, 50.0).is_none());
    }

    #[test]
    fn inserted_model_blocks_line_of_sight() {
        let tree = DynamicMapTree::new(world_bounds());
        let (from, to) = across_wall();

        tree.insert(1, wall(1), true);
        assert!(tree.contains(1));
        assert_eq!(tree.len(), 1);
        assert!(tree.raycast(&from, &to, false));
    }

    #[test]
    fn removed_model_stops_blocking() {
        let tree = DynamicMapTree::new(world_bounds());
        let (from, to) = across_wall();

        tree.insert(1, wall(1), true);
        assert!(tree.raycast(&from, &to, false));

        assert!(tree.remove(1));
        assert!(!tree.contains(1));
        assert!(!tree.raycast(&from, &to, false));

        // Removing again reports that nothing was there.
        assert!(!tree.remove(1));
    }

    #[test]
    fn disabled_model_stops_blocking_without_removal() {
        let tree = DynamicMapTree::new(world_bounds());
        let (from, to) = across_wall();

        tree.insert(1, wall(1), true);
        assert!(tree.raycast(&from, &to, false));

        // An opened door is still tracked, but no longer collides.
        assert!(tree.set_enabled(1, false));
        assert!(tree.contains(1));
        assert!(!tree.raycast(&from, &to, false));

        assert!(tree.set_enabled(1, true));
        assert!(tree.raycast(&from, &to, false));
    }

    #[test]
    fn set_enabled_reports_unknown_id() {
        let tree = DynamicMapTree::new(world_bounds());
        assert!(!tree.set_enabled(42, false));
    }

    #[test]
    fn model_inserted_disabled_does_not_block() {
        let tree = DynamicMapTree::new(world_bounds());
        let (from, to) = across_wall();

        tree.insert(1, wall(1), false);
        assert!(tree.contains(1));
        assert!(!tree.raycast(&from, &to, false));
    }

    #[test]
    fn clear_drops_every_model() {
        let tree = DynamicMapTree::new(world_bounds());
        let (from, to) = across_wall();

        tree.insert(1, wall(1), true);
        tree.insert(2, wall(2), true);
        assert_eq!(tree.len(), 2);

        tree.clear();
        assert!(tree.is_empty());
        assert!(!tree.raycast(&from, &to, false));
    }

    #[test]
    fn inserting_empty_group_list_is_ignored() {
        let tree = DynamicMapTree::new(world_bounds());
        tree.insert(1, Vec::new(), true);
        assert!(!tree.contains(1));
    }
}
