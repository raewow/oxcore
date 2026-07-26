//! Per-map visibility/activation tuning and the load-shedding ramp.

use std::time::Duration;

use oxcore_map::grid_coords::world_to_grid;
use oxcore_map::map::{
    MapConfig, MapKind, DEFAULT_VISIBILITY_BG, DEFAULT_VISIBILITY_DISTANCE,
    DEFAULT_VISIBILITY_INSTANCE,
};
use oxcore_map::{Map, MapManager};
use oxcore_shared::protocol::{ObjectGuid, Position};

fn pos(x: f32, y: f32) -> Position {
    Position {
        x,
        y,
        z: 0.0,
        o: 0.0,
    }
}

/// Grids that exist after a player is added at `at`.
fn activated_grids(map: &Map, at: Position) -> usize {
    map.add_player(ObjectGuid::new_player(1), at);
    let grid_mgr = map.grid_manager();
    grid_mgr.read().active_grid_count()
}

#[test]
fn map_kind_defaults_match_the_reference() {
    assert_eq!(
        MapConfig::for_kind(MapKind::Continent).visibility_distance,
        DEFAULT_VISIBILITY_DISTANCE
    );
    assert_eq!(
        MapConfig::for_kind(MapKind::Dungeon).visibility_distance,
        DEFAULT_VISIBILITY_INSTANCE
    );
    assert_eq!(
        MapConfig::for_kind(MapKind::Raid).visibility_distance,
        DEFAULT_VISIBILITY_INSTANCE
    );
    assert_eq!(
        MapConfig::for_kind(MapKind::BattleGround).visibility_distance,
        DEFAULT_VISIBILITY_BG
    );

    assert_eq!(MapKind::from_dbc_map_type(0), MapKind::Continent);
    assert_eq!(MapKind::from_dbc_map_type(1), MapKind::Dungeon);
    assert_eq!(MapKind::from_dbc_map_type(2), MapKind::Raid);
    assert_eq!(MapKind::from_dbc_map_type(3), MapKind::BattleGround);
}

#[test]
fn activation_covers_only_the_radius() {
    // Well inside a grid, so a 100 yard radius cannot reach a neighbour.
    let centre = pos(266.0, 266.0);

    let continent = Map::with_config(0, 0, MapConfig::for_kind(MapKind::Continent));
    assert_eq!(
        activated_grids(&continent, centre),
        1,
        "a 100 yard radius in the middle of a grid should activate exactly that grid"
    );

    // A battleground sees a full grid width, so it must reach further.
    let bg = Map::with_config(30, 1, MapConfig::for_kind(MapKind::BattleGround));
    assert!(
        activated_grids(&bg, centre) > 1,
        "533 yard visibility must activate neighbouring grids"
    );
}

#[test]
fn activation_spans_a_boundary_when_the_radius_does() {
    // Near a grid edge the radius genuinely straddles two grids.
    let map = Map::with_config(0, 0, MapConfig::for_kind(MapKind::Continent));
    let near_edge = pos(520.0, 266.0);

    let (gx_lo, _) = world_to_grid(near_edge.x - 100.0, near_edge.y);
    let (gx_hi, _) = world_to_grid(near_edge.x + 100.0, near_edge.y);
    assert_ne!(gx_lo, gx_hi, "fixture should straddle a grid boundary");

    assert_eq!(activated_grids(&map, near_edge), 2);
}

#[test]
fn distance_ramp_clamps_to_bounds() {
    let mut config = MapConfig::for_kind(MapKind::Continent);
    config.min_visibility_distance = 45.0;
    config.min_grid_activation_distance = 45.0;
    let map = Map::with_config(0, 0, config);

    let slow = config.tick_lower_threshold + Duration::from_millis(1);
    let fast = Duration::from_millis(0);

    assert_eq!(map.visibility_distance(), DEFAULT_VISIBILITY_DISTANCE);

    // Shrinks one yard per tick, never past the floor.
    map.tune_distances(slow);
    assert_eq!(map.visibility_distance(), DEFAULT_VISIBILITY_DISTANCE - 1.0);
    for _ in 0..1000 {
        map.tune_distances(slow);
    }
    assert_eq!(map.visibility_distance(), 45.0);
    assert_eq!(map.grid_activation_distance(), 45.0);

    // Grows back, never past the ceiling.
    for _ in 0..1000 {
        map.tune_distances(fast);
    }
    assert_eq!(map.visibility_distance(), DEFAULT_VISIBILITY_DISTANCE);
    assert_eq!(map.grid_activation_distance(), DEFAULT_VISIBILITY_DISTANCE);
}

#[test]
fn only_continents_ramp() {
    let map = Map::with_config(389, 1, MapConfig::for_kind(MapKind::Dungeon));
    let before = map.visibility_distance();

    for _ in 0..10 {
        map.tune_distances(Duration::from_secs(1));
    }

    assert_eq!(map.visibility_distance(), before);
}

#[test]
fn manager_applies_the_installed_provider() {
    let mgr = MapManager::new();

    // Without a provider maps fall back to the continent defaults.
    assert_eq!(
        mgr.get_or_create_map(0, 0).visibility_distance(),
        DEFAULT_VISIBILITY_DISTANCE
    );

    mgr.set_config_provider(std::sync::Arc::new(|map_id: u32, _instance: u32| {
        if map_id == 30 {
            MapConfig::for_kind(MapKind::BattleGround)
        } else {
            MapConfig::for_kind(MapKind::Continent)
        }
    }));

    let bg = mgr.get_or_create_map(30, 1);
    assert_eq!(bg.kind(), MapKind::BattleGround);
    assert_eq!(bg.visibility_distance(), DEFAULT_VISIBILITY_BG);

    // Already-created maps keep the config they were built with.
    assert_eq!(mgr.get_or_create_map(0, 0).kind(), MapKind::Continent);
}

#[test]
fn visibility_override_keeps_the_ramp_ceiling_consistent() {
    // The config kill-switch: restoring the old 533 yard continent radius.
    let config = MapConfig::for_kind(MapKind::Continent).with_visibility_distance(533.0);
    assert_eq!(config.visibility_distance, 533.0);
    assert_eq!(config.max_visibility_distance, 533.0);
    assert_eq!(config.grid_activation_distance, 533.0);

    let map = Map::with_config(0, 0, config);
    for _ in 0..10 {
        map.tune_distances(Duration::from_millis(0));
    }
    assert_eq!(
        map.visibility_distance(),
        533.0,
        "must not ramp past the override"
    );
}
