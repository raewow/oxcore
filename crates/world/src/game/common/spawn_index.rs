//! Spatial index over a map's spawn table.
//!
//! Grid loading needs "every spawn in grid (gx, gy)". Without an index that is a
//! linear scan of the whole map's spawn list per grid load — tens of thousands of
//! `world_to_grid` calls on a continent. This builds a per-grid guid set once at
//! startup, at grid granularity.
//!
//! Kept as a free function rather than a manager method so it is unit-testable:
//! `CreatureManager::new` requires a live database pool.

use std::collections::HashMap;

use crate::map::grid_coords::world_to_grid;
use oxcore_shared::protocol::Position;

/// Spawn list indices bucketed by spatial grid.
pub type SpawnGridIndex = HashMap<(u8, u8), Vec<u32>>;

/// Bucket `spawns` by the grid their position falls in.
///
/// Values are indices into `spawns`, so the caller keeps ownership of the spawn
/// data. Valid only while `spawns` is not reordered — both spawn tables are
/// append-only after load.
pub fn build_grid_index<T>(spawns: &[T], position: impl Fn(&T) -> Position) -> SpawnGridIndex {
    let mut index: SpawnGridIndex = HashMap::new();

    for (i, spawn) in spawns.iter().enumerate() {
        let pos = position(spawn);
        index
            .entry(world_to_grid(pos.x, pos.y))
            .or_default()
            .push(i as u32);
    }

    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32) -> Position {
        Position {
            x,
            y,
            z: 0.0,
            o: 0.0,
        }
    }

    /// Deterministic LCG so a failure is reproducible.
    fn sample_positions(count: usize) -> Vec<Position> {
        let mut seed: u32 = 0xc0ff_ee11;
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / 16_777_216.0 * 30_000.0 - 15_000.0
        };
        (0..count).map(|_| pos(next(), next())).collect()
    }

    #[test]
    fn index_matches_linear_filter() {
        let spawns = sample_positions(10_000);
        let index = build_grid_index(&spawns, |p| *p);

        // Every occupied grid must hold exactly what a linear scan would find.
        for (&(gx, gy), indices) in &index {
            let expected: Vec<u32> = spawns
                .iter()
                .enumerate()
                .filter(|(_, p)| world_to_grid(p.x, p.y) == (gx, gy))
                .map(|(i, _)| i as u32)
                .collect();
            assert_eq!(indices, &expected, "mismatch for grid ({gx}, {gy})");
        }

        // And nothing is lost: every spawn appears exactly once.
        let total: usize = index.values().map(Vec::len).sum();
        assert_eq!(total, spawns.len());
    }

    #[test]
    fn empty_input_yields_empty_index() {
        let spawns: Vec<Position> = Vec::new();
        assert!(build_grid_index(&spawns, |p| *p).is_empty());
    }

    #[test]
    fn spawns_sharing_a_grid_are_bucketed_together() {
        // Three points well inside one grid, one far away. Note the world origin
        // sits exactly on a grid boundary, so points are chosen away from it.
        let spawns = vec![
            pos(100.0, 100.0),
            pos(150.0, 120.0),
            pos(200.0, 90.0),
            pos(5000.0, 5000.0),
        ];
        let index = build_grid_index(&spawns, |p| *p);

        assert_eq!(index.len(), 2);
        assert_eq!(index[&world_to_grid(100.0, 100.0)], vec![0, 1, 2]);
        assert_eq!(index[&world_to_grid(5000.0, 5000.0)], vec![3]);
    }
}
