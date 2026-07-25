//! Grid unload guards: the checks that must pass before a grid may be drained.

use std::time::Duration;

use oxcore_map::grid_coords::{world_to_grid, GRID_SIZE};
use oxcore_map::map::{MapConfig, MapKind};
use oxcore_map::Map;
use oxcore_shared::protocol::{ObjectGuid, Position};

fn pos(x: f32, y: f32) -> Position {
    Position {
        x,
        y,
        z: 0.0,
        o: 0.0,
    }
}

/// A map whose grids go stale immediately, so the state machine is testable
/// without waiting out the real five minute delay.
fn impatient_map() -> Map {
    let mut config = MapConfig::for_kind(MapKind::Continent);
    config.grid_unload_delay = Duration::ZERO;
    Map::with_config(0, 0, config)
}

/// Drive a grid to Idle: a player arrives, the grid finishes loading (which the
/// grid system does in production), then the player leaves.
fn make_idle(map: &Map, at: Position) -> (u8, u8) {
    let (gx, gy) = world_to_grid(at.x, at.y);
    let player = ObjectGuid::new_player(999);

    map.add_player(player, at);
    {
        let grid_mgr = map.grid_manager();
        grid_mgr.write().mark_loaded(gx, gy);
    }
    map.remove_player(player);

    (gx, gy)
}

#[test]
fn idle_grid_with_nobody_around_unloads() {
    let map = impatient_map();
    let (gx, gy) = make_idle(&map, pos(266.0, 266.0));

    let pass = map.update_grid_states();
    assert!(
        pass.to_unload.contains(&(gx, gy)),
        "an idle grid with nobody near it should unload"
    );
}

#[test]
fn active_objects_near_grid_holds_neighbour() {
    // The "unloads under their feet" bug: a player standing just over the
    // boundary must keep the grid they left alive.
    let map = impatient_map();

    let near_edge = pos(520.0, 266.0);
    let over_boundary = pos(near_edge.x + 20.0, near_edge.y);
    let (gx, gy) = world_to_grid(near_edge.x, near_edge.y);
    assert_ne!(
        (gx, gy),
        world_to_grid(over_boundary.x, over_boundary.y),
        "fixture should place the player in the next grid over"
    );

    make_idle(&map, near_edge);
    map.add_player(ObjectGuid::new_player(1), over_boundary);

    assert!(map.active_objects_near_grid(gx, gy));
    assert!(
        !map.update_grid_states().to_unload.contains(&(gx, gy)),
        "grid must not unload while a player is 20 yards away in the next grid"
    );

    // Far away, the same grid is free to go.
    let far = pos(near_edge.x + GRID_SIZE * 4.0, near_edge.y);
    map.relocate_player(ObjectGuid::new_player(1), over_boundary, far);
    assert!(!map.active_objects_near_grid(gx, gy));
}

#[test]
fn unload_respects_explicit_and_active_locks() {
    let map = impatient_map();
    let at = pos(266.0, 266.0);
    let (gx, gy) = make_idle(&map, at);

    // Explicit pin.
    {
        let grid_mgr = map.grid_manager();
        let mut grid_mgr = grid_mgr.write();
        grid_mgr
            .get_grid_mut(gx, gy)
            .unwrap()
            .set_unload_explicit_lock(true);
    }
    assert!(!map.update_grid_states().to_unload.contains(&(gx, gy)));

    // Released again.
    {
        let grid_mgr = map.grid_manager();
        let mut grid_mgr = grid_mgr.write();
        grid_mgr
            .get_grid_mut(gx, gy)
            .unwrap()
            .set_unload_explicit_lock(false);
    }
    assert!(map.update_grid_states().to_unload.contains(&(gx, gy)));
}

#[test]
fn active_object_pins_its_spawn_grid() {
    let map = impatient_map();
    let at = pos(266.0, 266.0);
    let (gx, gy) = make_idle(&map, at);

    // An active object far away, but whose spawn point is in this grid.
    let boss = ObjectGuid::new_creature(1, 1);
    map.add_to_active(boss, at);
    assert!(!map.update_grid_states().to_unload.contains(&(gx, gy)));

    map.remove_from_active(boss);
    assert!(map.update_grid_states().to_unload.contains(&(gx, gy)));
}

#[test]
fn active_object_keeps_neighbouring_grids_loaded() {
    // Active objects count for proximity just like players do.
    let map = impatient_map();
    let near_edge = pos(520.0, 266.0);
    let (gx, gy) = world_to_grid(near_edge.x, near_edge.y);
    make_idle(&map, near_edge);

    let ship = ObjectGuid::new_creature(2, 1);
    map.add_to_active(ship, pos(near_edge.x + 20.0, near_edge.y));

    assert!(map.active_objects_near_grid(gx, gy));
}

#[test]
fn a_player_walking_through_keeps_the_grid_alive() {
    let map = impatient_map();
    let at = pos(266.0, 266.0);
    let (gx, gy) = make_idle(&map, at);

    // Idle and unloadable...
    assert!(map.update_grid_states().to_unload.contains(&(gx, gy)));

    // ...until someone walks back in.
    map.add_player(ObjectGuid::new_player(2), at);
    assert!(!map.update_grid_states().to_unload.contains(&(gx, gy)));
}

#[test]
fn creatures_in_grid_lists_only_creatures() {
    let map = Map::new(0, 0);
    let at = pos(266.0, 266.0);

    let creature = ObjectGuid::new_creature(1, 1);
    map.add_creature(creature, at);
    map.add_creature(ObjectGuid::new_pet(1, 1), at);
    map.add_player(ObjectGuid::new_player(1), at);
    map.add_gameobject(ObjectGuid::new_gameobject(1, 1), at);

    let (gx, gy) = world_to_grid(at.x, at.y);
    assert_eq!(map.creatures_in_grid(gx, gy), vec![creature]);
}
