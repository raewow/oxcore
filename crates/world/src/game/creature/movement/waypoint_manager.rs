//! WaypointManager - manages waypoint state, no database access
//!
//! Stores loaded waypoint data and provides lookup by spawn GUID or entry.
//! Follows the Manager/Repository separation pattern: the node-editing methods here
//! change only the in-memory path; persisting the edit is the repository's job.

use super::generators::Waypoint;
use super::waypoint_repository::WaypointData;
use dashmap::DashMap;
use oxcore_shared::protocol::Position;
use std::sync::Arc;

pub use crate::core::common::position::{is_valid_map_coord, normalize_map_coord};

/// Orientation value the DB uses to mean "no orientation override at this node".
pub const NO_ORIENTATION: f32 = 100.0;

/// Which table a path came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaypointPathOrigin {
    /// No path is assigned to the creature.
    NoPath,
    /// `creature_movement`, keyed by creature spawn guid
    Guid,
    /// `creature_movement_template`, keyed by creature entry
    Entry,
    /// A path provided by a script rather than either waypoint table.
    Special,
}

impl std::fmt::Display for WaypointPathOrigin {
    /// Human-readable origin.
    ///
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            WaypointPathOrigin::NoPath => "<no path>",
            WaypointPathOrigin::Guid => "guid",
            WaypointPathOrigin::Entry => "entry",
            WaypointPathOrigin::Special => "special",
        })
    }
}

/// Waypoint data manager - state only, no database
pub struct WaypointManager {
    /// Waypoints by creature spawn GUID
    guid_waypoints: DashMap<u32, Arc<Vec<Waypoint>>>,
    /// Waypoints by creature entry (template)
    template_waypoints: DashMap<u32, Arc<Vec<Waypoint>>>,
}

impl WaypointManager {
    pub fn new() -> Self {
        Self {
            guid_waypoints: DashMap::new(),
            template_waypoints: DashMap::new(),
        }
    }

    /// Load waypoints from repository data
    pub fn load_from_data(&self, data: WaypointData) {
        for (id, waypoints) in data.guid_waypoints {
            self.guid_waypoints.insert(id, Arc::new(waypoints));
        }

        for (entry, waypoints) in data.template_waypoints {
            self.template_waypoints.insert(entry, Arc::new(waypoints));
        }

        tracing::debug!(
            "WaypointManager loaded {} GUID paths, {} template paths",
            self.guid_waypoints.len(),
            self.template_waypoints.len()
        );
    }

    /// Get waypoints for a creature spawn (checks GUID first, then entry)
    pub fn get_waypoints(&self, spawn_id: u32, entry: u32) -> Option<Arc<Vec<Waypoint>>> {
        // Try per-GUID waypoints first (FromGuid)
        if let Some(waypoints) = self.guid_waypoints.get(&spawn_id) {
            return Some(Arc::clone(&waypoints));
        }

        // Fall back to template waypoints (FromEntry)
        self.template_waypoints.get(&entry).map(|w| Arc::clone(&w))
    }

    /// Check if a creature has waypoints defined
    pub fn has_waypoints(&self, spawn_id: u32, entry: u32) -> bool {
        self.guid_waypoints.contains_key(&spawn_id) || self.template_waypoints.contains_key(&entry)
    }

    /// Drop every loaded path.
    ///
    /// The reference cleanup also issues database deletes; in this port the manager is
    /// state-only, so persisting the deletion is the repository's responsibility.
    pub fn cleanup(&self) {
        self.guid_waypoints.clear();
        self.template_waypoints.clear();
    }

    fn paths(&self, origin: WaypointPathOrigin) -> Option<&DashMap<u32, Arc<Vec<Waypoint>>>> {
        Some(match origin {
            WaypointPathOrigin::Guid => &self.guid_waypoints,
            WaypointPathOrigin::Entry => &self.template_waypoints,
            WaypointPathOrigin::NoPath | WaypointPathOrigin::Special => return None,
        })
    }

    /// Fetch a path for editing.
    ///
    /// Paths are shared as `Arc`s with the generators already patrolling them, so an
    /// edit builds a new vector and swaps it in; in-flight patrols keep their snapshot
    /// until they next look the path up.
    fn edit_path<F, R>(&self, origin: WaypointPathOrigin, key: u32, edit: F) -> Option<R>
    where
        F: FnOnce(&mut Vec<Waypoint>) -> R,
    {
        let paths = self.paths(origin)?;
        let mut nodes = paths.get(&key).map(|path| path.as_ref().clone())?;
        let result = edit(&mut nodes);
        paths.insert(key, Arc::new(nodes));
        Some(result)
    }

    /// Renumber points so they stay contiguous and 1-based, as the DB requires.
    fn renumber(nodes: &mut [Waypoint]) {
        for (index, node) in nodes.iter_mut().enumerate() {
            node.point_id = index as u32 + 1;
        }
    }

    /// Insert a node into a path, shifting the nodes after it along.
    ///
    /// `point` is the 1-based point id to insert at; `None` (or a point past the end)
    /// appends. Returns the point id the node landed on. Persisting the insert and the
    /// renumbering of later points is the caller's job.
    pub fn add_node(
        &self,
        origin: WaypointPathOrigin,
        key: u32,
        point: Option<u32>,
        position: Position,
    ) -> Option<u32> {
        let node = Waypoint {
            point_id: 0,
            position,
            wait_time: 0,
            wander_distance: 0.0,
            script_id: 0,
            // New nodes use the 100 sentinel: no forced orientation.
            orientation: None,
        };

        self.edit_path(origin, key, move |nodes| {
            let index = match point {
                Some(point) if point >= 1 => ((point - 1) as usize).min(nodes.len()),
                _ => nodes.len(),
            };

            nodes.insert(index, node);
            Self::renumber(nodes);
            index as u32 + 1
        })
    }

    /// Remove a node from a path, pulling later nodes back one point.
    pub fn delete_node(&self, origin: WaypointPathOrigin, key: u32, point: u32) -> bool {
        self.edit_path(origin, key, |nodes| {
            let Some(index) = nodes.iter().position(|node| node.point_id == point) else {
                return false;
            };

            nodes.remove(index);
            Self::renumber(nodes);
            true
        })
        .unwrap_or(false)
    }

    /// Drop an entire path.
    ///
    /// The reference only clears the node map, keeping the (now empty) entry alive because the
    /// generators hold raw pointers into it. `Arc` makes removal safe here.
    pub fn delete_path(&self, origin: WaypointPathOrigin, key: u32) -> bool {
        self.paths(origin)
            .and_then(|paths| paths.remove(&key))
            .is_some()
    }

    /// Move a node.
    pub fn set_node_position(
        &self,
        origin: WaypointPathOrigin,
        key: u32,
        point: u32,
        position: Position,
    ) -> bool {
        self.edit_path(origin, key, |nodes| {
            match nodes.iter_mut().find(|node| node.point_id == point) {
                Some(node) => {
                    node.position.x = position.x;
                    node.position.y = position.y;
                    node.position.z = position.z;
                    true
                }
                None => false,
            }
        })
        .unwrap_or(false)
    }

    /// Change how long a creature waits at a node.
    pub fn set_node_waittime(
        &self,
        origin: WaypointPathOrigin,
        key: u32,
        point: u32,
        wait_time: u32,
    ) -> bool {
        self.edit_path(origin, key, |nodes| {
            match nodes.iter_mut().find(|node| node.point_id == point) {
                Some(node) => {
                    node.wait_time = wait_time;
                    true
                }
                None => false,
            }
        })
        .unwrap_or(false)
    }

    /// Set the orientation a creature faces at a node.
    ///
    /// [`NO_ORIENTATION`] clears the override, the same value the DB uses to mean "no
    /// forced facing here".
    pub fn set_node_orientation(
        &self,
        origin: WaypointPathOrigin,
        key: u32,
        point: u32,
        orientation: f32,
    ) -> bool {
        self.edit_path(origin, key, |nodes| {
            match nodes.iter_mut().find(|node| node.point_id == point) {
                Some(node) => {
                    if orientation == NO_ORIENTATION {
                        node.orientation = None;
                    } else {
                        node.orientation = Some(orientation);
                        node.position.o = orientation;
                    }
                    true
                }
                None => false,
            }
        })
        .unwrap_or(false)
    }

    /// Attach a movement script to a node.
    ///
    /// Returns whether the node was found. The reference counterpart instead returns whether
    /// the script id is a known `creature_movement_scripts` entry; there is no such
    /// registry in this port, so that validity check is not modeled. The persisting
    /// UPDATE remains the repository's job.
    pub fn set_node_script_id(
        &self,
        origin: WaypointPathOrigin,
        key: u32,
        point: u32,
        script_id: u32,
    ) -> bool {
        self.edit_path(origin, key, |nodes| {
            match nodes.iter_mut().find(|node| node.point_id == point) {
                Some(node) => {
                    node.script_id = script_id;
                    true
                }
                None => false,
            }
        })
        .unwrap_or(false)
    }
}

impl Default for WaypointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32) -> Position {
        Position {
            x,
            y,
            z: 10.0,
            o: 0.0,
        }
    }

    fn manager_with_path() -> WaypointManager {
        let manager = WaypointManager::new();
        let nodes: Vec<Waypoint> = (1..=3)
            .map(|point| Waypoint {
                point_id: point,
                position: pos(point as f32, 0.0),
                wait_time: 0,
                wander_distance: 0.0,
                script_id: 0,
                orientation: None,
            })
            .collect();
        manager.guid_waypoints.insert(7, Arc::new(nodes));
        manager
    }

    fn points(manager: &WaypointManager) -> Vec<(u32, f32)> {
        manager
            .get_waypoints(7, 0)
            .unwrap()
            .iter()
            .map(|node| (node.point_id, node.position.x))
            .collect()
    }

    #[test]
    fn add_node_appends_when_no_point_is_given() {
        let manager = manager_with_path();

        assert_eq!(
            manager.add_node(WaypointPathOrigin::Guid, 7, None, pos(9.0, 0.0)),
            Some(4)
        );
        assert_eq!(
            points(&manager),
            vec![(1, 1.0), (2, 2.0), (3, 3.0), (4, 9.0)]
        );
    }

    #[test]
    fn add_node_inserts_and_shifts_the_following_points() {
        let manager = manager_with_path();

        assert_eq!(
            manager.add_node(WaypointPathOrigin::Guid, 7, Some(2), pos(9.0, 0.0)),
            Some(2)
        );
        assert_eq!(
            points(&manager),
            vec![(1, 1.0), (2, 9.0), (3, 2.0), (4, 3.0)]
        );
    }

    #[test]
    fn add_node_on_a_missing_path_changes_nothing() {
        let manager = manager_with_path();

        assert_eq!(
            manager.add_node(WaypointPathOrigin::Guid, 999, Some(1), pos(9.0, 0.0)),
            None
        );
        assert_eq!(
            manager.add_node(WaypointPathOrigin::Entry, 7, Some(1), pos(9.0, 0.0)),
            None
        );
    }

    #[test]
    fn delete_node_pulls_later_points_back() {
        let manager = manager_with_path();

        assert!(manager.delete_node(WaypointPathOrigin::Guid, 7, 2));
        assert_eq!(points(&manager), vec![(1, 1.0), (2, 3.0)]);

        // A point that is not on the path is a no-op.
        assert!(!manager.delete_node(WaypointPathOrigin::Guid, 7, 99));
        assert_eq!(points(&manager), vec![(1, 1.0), (2, 3.0)]);
    }

    #[test]
    fn set_node_position_and_waittime_edit_only_the_named_point() {
        let manager = manager_with_path();

        assert!(manager.set_node_position(WaypointPathOrigin::Guid, 7, 2, pos(42.0, 43.0)));
        assert!(manager.set_node_waittime(WaypointPathOrigin::Guid, 7, 2, 5_000));

        let path = manager.get_waypoints(7, 0).unwrap();
        assert_eq!(path[1].position.x, 42.0);
        assert_eq!(path[1].position.y, 43.0);
        assert_eq!(path[1].wait_time, 5_000);
        assert_eq!(path[0].wait_time, 0);

        assert!(!manager.set_node_waittime(WaypointPathOrigin::Guid, 7, 99, 1));
    }

    #[test]
    fn set_node_orientation_sets_and_clears_the_facing_override() {
        let manager = manager_with_path();

        assert!(manager.set_node_orientation(WaypointPathOrigin::Guid, 7, 2, 1.5));
        let path = manager.get_waypoints(7, 0).unwrap();
        assert_eq!(path[1].orientation, Some(1.5));
        assert_eq!(path[1].position.o, 1.5);
        // Untouched node keeps its lack of override.
        assert_eq!(path[0].orientation, None);

        // The sentinel clears the override again.
        assert!(manager.set_node_orientation(WaypointPathOrigin::Guid, 7, 2, NO_ORIENTATION));
        assert_eq!(manager.get_waypoints(7, 0).unwrap()[1].orientation, None);

        assert!(!manager.set_node_orientation(WaypointPathOrigin::Guid, 7, 99, 1.0));
    }

    #[test]
    fn origin_renders_the_table_name() {
        assert_eq!(WaypointPathOrigin::NoPath.to_string(), "<no path>");
        assert_eq!(WaypointPathOrigin::Guid.to_string(), "guid");
        assert_eq!(WaypointPathOrigin::Entry.to_string(), "entry");
        assert_eq!(WaypointPathOrigin::Special.to_string(), "special");
    }

    #[test]
    fn set_node_script_id_attaches_the_script_to_the_named_point() {
        let manager = manager_with_path();

        assert!(manager.set_node_script_id(WaypointPathOrigin::Guid, 7, 3, 555));
        let path = manager.get_waypoints(7, 0).unwrap();
        assert_eq!(path[2].script_id, 555);
        assert_eq!(path[0].script_id, 0);

        assert!(!manager.set_node_script_id(WaypointPathOrigin::Guid, 7, 99, 1));
    }

    #[test]
    fn editing_a_path_leaves_existing_holders_on_their_snapshot() {
        let manager = manager_with_path();
        let before = manager.get_waypoints(7, 0).unwrap();

        manager.delete_node(WaypointPathOrigin::Guid, 7, 1);

        assert_eq!(before.len(), 3);
        assert_eq!(manager.get_waypoints(7, 0).unwrap().len(), 2);
    }

    #[test]
    fn delete_path_and_cleanup_drop_the_paths() {
        let manager = manager_with_path();

        assert!(manager.delete_path(WaypointPathOrigin::Guid, 7));
        assert!(!manager.delete_path(WaypointPathOrigin::Guid, 7));
        assert!(!manager.has_waypoints(7, 0));

        let manager = manager_with_path();
        manager.cleanup();
        assert!(!manager.has_waypoints(7, 0));
    }
}
