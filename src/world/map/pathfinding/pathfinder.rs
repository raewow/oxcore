//! PathFinder - integrates NavMesh and VMap for pathfinding
//!
//! Uses a 3-tier fallback: straight line -> NavMesh A* -> obstacle avoidance.
//! This is the low-level MMap PathFinder that the game-level wrapper delegates to.

use crate::shared::protocol::Position;
use super::mmap_manager::MMapManager;
use super::types::PathResult;
use std::sync::Arc;

/// MMap PathFinder - integrates NavMesh and VMap
pub struct PathFinder {
    mmap_mgr: Arc<MMapManager>,
}

impl PathFinder {
    pub fn new(mmap_mgr: Arc<MMapManager>) -> Self {
        Self { mmap_mgr }
    }

    /// Calculate path with integrated NavMesh + VMap + Obstacle Avoidance
    ///
    /// Fallback hierarchy:
    /// 1. Try straight line (fastest) - checks LOS and height
    /// 2. Try NavMesh A* pathfinding
    /// 3. Fallback: multi-waypoint obstacle avoidance
    pub fn calculate_path(
        &self,
        map_id: u32,
        start: Position,
        end: Position,
    ) -> PathResult {
        // 1. Try straight line first (fastest)
        if self.is_clear_path(map_id, start, end) {
            return PathResult::StraightLine(start, end);
        }

        // 2. Try NavMesh A* pathfinding
        let navmesh_result = self.mmap_mgr.calculate_path(map_id, start, end);

        match navmesh_result {
            PathResult::Complete(_) | PathResult::Partial(_) => {
                return navmesh_result;
            }
            PathResult::NoPath | PathResult::StraightLine(_, _) => {
                // NavMesh unavailable or failed - use obstacle avoidance
            }
        }

        // 3. Fallback: Multi-waypoint obstacle avoidance
        self.calculate_obstacle_avoidance(map_id, start, end)
    }

    /// Check if straight line path is clear
    fn is_clear_path(&self, map_id: u32, start: Position, end: Position) -> bool {
        let vmap = self.mmap_mgr.vmap();

        // Line of sight check
        if !vmap.is_in_line_of_sight(map_id, start, end) {
            return false;
        }

        // Short paths are trivially clear
        let dist = distance(&start, &end);
        if dist < 5.0 {
            return true;
        }

        // Height check along path - sample points to detect cliffs/gaps
        let steps = (dist / 2.0).ceil() as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let check_x = start.x + (end.x - start.x) * t;
            let check_y = start.y + (end.y - start.y) * t;
            let expected_z = start.z + (end.z - start.z) * t;

            if let Some(ground_z) = vmap.get_height(map_id, check_x, check_y, expected_z + 10.0) {
                let height_diff = (ground_z - expected_z).abs();
                if height_diff > 3.0 {
                    return false; // Too steep or gap
                }
            }
        }

        true
    }

    /// Multi-waypoint obstacle avoidance (fallback when NavMesh unavailable)
    ///
    /// Generates 8 candidate waypoint positions around the midpoint obstacle
    /// and tests each for clear paths to start/end.
    fn calculate_obstacle_avoidance(
        &self,
        map_id: u32,
        start: Position,
        end: Position,
    ) -> PathResult {
        let vmap = self.mmap_mgr.vmap();

        let mid = Position {
            x: (start.x + end.x) / 2.0,
            y: (start.y + end.y) / 2.0,
            z: start.z,
            o: 0.0,
        };

        // Generate candidate waypoints in 8 directions around obstacle
        let offsets = [
            (10.0, 0.0),     // East
            (-10.0, 0.0),    // West
            (0.0, 10.0),     // North
            (0.0, -10.0),    // South
            (7.0, 7.0),      // NE
            (-7.0, 7.0),     // NW
            (7.0, -7.0),     // SE
            (-7.0, -7.0),    // SW
        ];

        for (ox, oy) in offsets {
            let waypoint = Position {
                x: mid.x + ox,
                y: mid.y + oy,
                z: vmap.get_height(map_id, mid.x + ox, mid.y + oy, mid.z + 10.0)
                    .unwrap_or(mid.z),
                o: 0.0,
            };

            // Test if this waypoint creates valid 2-segment path
            if self.is_clear_path(map_id, start, waypoint) &&
               self.is_clear_path(map_id, waypoint, end) {
                tracing::debug!(
                    "[PATHFIND] Found obstacle avoidance path via waypoint ({:.1}, {:.1})",
                    waypoint.x,
                    waypoint.y
                );

                return PathResult::Complete(vec![start, waypoint, end]);
            }
        }

        // No valid path found - return partial
        tracing::warn!(
            "[PATHFIND] No path found from ({:.1}, {:.1}) to ({:.1}, {:.1})",
            start.x, start.y, end.x, end.y
        );

        PathResult::Partial(vec![start])
    }

    /// Path smoothing - remove redundant waypoints
    pub fn smooth_path(&self, map_id: u32, path: &mut Vec<Position>) {
        if path.len() <= 2 {
            return;
        }

        let mut i = 0;
        while i < path.len().saturating_sub(2) {
            // Can we skip the middle waypoint?
            if self.is_clear_path(map_id, path[i], path[i + 2]) {
                path.remove(i + 1);
                // Don't increment i - check again from same position
            } else {
                i += 1;
            }
        }
    }
}

/// Calculate 3D distance between two positions
fn distance(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
