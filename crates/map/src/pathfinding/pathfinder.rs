//! PathFinder - chooses between the navmesh and collision-based paths.
//!
//! On maps with a navmesh, paths come only from the mesh: a clear line of
//! sight says nothing about whether the ground between two points is walkable,
//! so it is never used as a shortcut or to straighten a mesh path. If the
//! start or end lies in a tile that is not loaded, the unit walks straight
//! there; if either has no walkable polygon nearby, there is no path.
//!
//! Maps without a navmesh fall back to collision: a straight line when it is
//! visible and the ground along it is continuous, otherwise a stepwise search
//! that only steps onto ground within climbing distance.

use super::mmap_manager::MMapManager;
use super::navmesh::{nav_terrain, NavPath};
use super::types::PathResult;
use crate::terrain::defines::INVALID_HEIGHT;
use crate::terrain::TerrainManager;
use oxcore_shared::protocol::Position;
use std::sync::Arc;

/// Step length of the collision-based search.
const STEP_SIZE: f32 = 3.0;
/// Directions tried at each step, as offsets from the bearing to the target.
const STEP_DIRECTIONS: [f32; 12] = [
    0.0,
    std::f32::consts::PI / 6.0,
    -std::f32::consts::PI / 6.0,
    std::f32::consts::PI / 3.0,
    -std::f32::consts::PI / 3.0,
    std::f32::consts::PI / 2.0,
    -std::f32::consts::PI / 2.0,
    std::f32::consts::PI * 2.0 / 3.0,
    -std::f32::consts::PI * 2.0 / 3.0,
    std::f32::consts::PI * 5.0 / 6.0,
    -std::f32::consts::PI * 5.0 / 6.0,
    std::f32::consts::PI,
];

/// MMap PathFinder - integrates NavMesh and VMap
pub struct PathFinder {
    mmap_mgr: Arc<MMapManager>,
    terrain: Option<Arc<TerrainManager>>,
}

impl PathFinder {
    pub fn new(mmap_mgr: Arc<MMapManager>) -> Self {
        Self {
            mmap_mgr,
            terrain: None,
        }
    }

    /// A pathfinder that also samples terrain heights for collision paths.
    pub fn with_terrain(mmap_mgr: Arc<MMapManager>, terrain: Arc<TerrainManager>) -> Self {
        Self {
            mmap_mgr,
            terrain: Some(terrain),
        }
    }

    /// Path for a unit that can walk and swim.
    pub fn calculate_path(
        &self,
        map_id: u32,
        instance_id: u32,
        start: Position,
        end: Position,
    ) -> PathResult {
        self.calculate_path_for(map_id, instance_id, start, end, nav_terrain::WALK_AND_SWIM)
    }

    /// Path over terrain classes in `include` (see [`nav_terrain`]).
    pub fn calculate_path_for(
        &self,
        map_id: u32,
        instance_id: u32,
        start: Position,
        end: Position,
        include: u16,
    ) -> PathResult {
        match self.mmap_mgr.calculate_path(map_id, start, end, include) {
            None => self.path_without_navmesh(map_id, instance_id, start, end),
            // Tiles not loaded at either end: head straight there.
            Some(NavPath::NoTiles) => PathResult::StraightLine(start, end),
            // A hole in the mesh: not reachable on foot.
            Some(NavPath::NoPolygon) => PathResult::NoPath,
            Some(NavPath::Path { points, complete }) => {
                if complete {
                    PathResult::Complete(points)
                } else {
                    PathResult::Partial(points)
                }
            }
        }
    }

    /// Collision-based path on a map without a navmesh.
    fn path_without_navmesh(
        &self,
        map_id: u32,
        instance_id: u32,
        start: Position,
        end: Position,
    ) -> PathResult {
        if distance(&start, &end) <= STEP_SIZE || self.is_clear_path(map_id, instance_id, start, end)
        {
            return PathResult::StraightLine(start, end);
        }

        let max_steps = ((distance(&start, &end) / STEP_SIZE) as usize + 3) * 2;
        let mut visited = Vec::with_capacity(max_steps);
        let mut path = vec![start];
        let mut steps = max_steps;
        if self.step_towards(map_id, instance_id, start, end, &mut path, &mut visited, &mut steps) {
            path.push(end);
            self.smooth_path(map_id, instance_id, &mut path);
            return PathResult::Complete(path);
        }

        tracing::debug!(
            "[PATHFIND] No path found from ({:.1}, {:.1}) to ({:.1}, {:.1})",
            start.x,
            start.y,
            end.x,
            end.y
        );
        PathResult::Partial(vec![start])
    }

    /// Depth-first step search towards `target`, preferring directions closest
    /// to the bearing, never revisiting a spot and never climbing or dropping
    /// more than a step.
    #[allow(clippy::too_many_arguments)]
    fn step_towards(
        &self,
        map_id: u32,
        instance_id: u32,
        current: Position,
        target: Position,
        path: &mut Vec<Position>,
        visited: &mut Vec<Position>,
        steps_left: &mut usize,
    ) -> bool {
        if distance(&current, &target) < STEP_SIZE + 1.0
            && self.is_clear_path(map_id, instance_id, current, target)
        {
            return true;
        }
        if *steps_left == 0 {
            return false;
        }
        *steps_left -= 1;

        let bearing = (target.y - current.y).atan2(target.x - current.x);
        for offset in STEP_DIRECTIONS {
            let angle = bearing + offset;
            let x = current.x + STEP_SIZE * angle.cos();
            let y = current.y + STEP_SIZE * angle.sin();
            let Some(z) = self.ground_height(map_id, instance_id, x, y, current.z) else {
                continue;
            };
            let dz = z - current.z;
            if dz < -STEP_SIZE || dz > STEP_SIZE * 0.9 {
                continue;
            }
            let next = Position::new(x, y, z, 0.0);
            if visited
                .iter()
                .any(|seen| distance(seen, &next) < STEP_SIZE * 0.5)
            {
                continue;
            }
            visited.push(next);
            if !self.is_in_sight(map_id, instance_id, current, next) {
                continue;
            }
            path.push(next);
            if self.step_towards(map_id, instance_id, next, target, path, visited, steps_left) {
                return true;
            }
            path.pop();
        }
        false
    }

    fn is_in_sight(&self, map_id: u32, instance_id: u32, a: Position, b: Position) -> bool {
        let lift = |p: Position| Position::new(p.x, p.y, p.z + 2.0, p.o);
        self.mmap_mgr
            .vmap()
            .is_in_line_of_sight(map_id, instance_id, lift(a), lift(b))
    }

    /// Ground height near `z` at (x, y): terrain and static models when terrain
    /// is attached, plus runtime collision (doors, bridges) of this copy.
    fn ground_height(&self, map_id: u32, instance_id: u32, x: f32, y: f32, z: f32) -> Option<f32> {
        let vmap = self.mmap_mgr.vmap();
        let mut best = vmap.get_height(map_id, instance_id, x, y, z + 2.0);
        if let Some(terrain) = &self.terrain {
            let h = terrain
                .get(map_id)
                .get_height_static(x, y, z + 2.0, Some(vmap), 50.0);
            if h > INVALID_HEIGHT {
                best = Some(match best {
                    Some(b) if (b - z).abs() < (h - z).abs() => b,
                    _ => h,
                });
            }
        }
        best
    }

    /// Check if straight line path is clear: visible, and the ground along it
    /// stays within a climb of the line. Ground that cannot be sampled at all
    /// (no terrain or model data) is not held against the path.
    fn is_clear_path(&self, map_id: u32, instance_id: u32, start: Position, end: Position) -> bool {
        if !self.is_in_sight(map_id, instance_id, start, end) {
            return false;
        }

        let dist = distance(&start, &end);
        let steps = (dist / 2.0).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = start.x + (end.x - start.x) * t;
            let y = start.y + (end.y - start.y) * t;
            let expected_z = start.z + (end.z - start.z) * t;
            if let Some(ground_z) = self.ground_height(map_id, instance_id, x, y, expected_z) {
                if (ground_z - expected_z).abs() > 3.0 {
                    return false; // Cliff, gap or wall
                }
            }
        }

        true
    }

    /// Remove waypoints of a collision path that a clear straight line skips.
    pub fn smooth_path(&self, map_id: u32, instance_id: u32, path: &mut Vec<Position>) {
        if path.len() <= 2 {
            return;
        }

        let mut i = 0;
        while i < path.len().saturating_sub(2) {
            if self.is_clear_path(map_id, instance_id, path[i], path[i + 2]) {
                path.remove(i + 1);
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
