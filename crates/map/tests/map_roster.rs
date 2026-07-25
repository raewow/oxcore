//! Grid membership and unload correctness.
//!
//! Each of the first three tests reproduces a defect that existed when the per-grid
//! creature/gameobject roster was maintained separately from `Grid::objects`:
//! the roster was only ever written at spawn time, so it drifted from reality on
//! the first relocation.

use oxcore_map::grid_coords::{world_to_grid, GRID_SIZE};
use oxcore_map::map::ObjectKind;
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

/// A point comfortably inside the grid one step along +x from `p`.
fn next_grid_over(p: Position) -> Position {
    let moved = pos(p.x + GRID_SIZE, p.y);
    assert_ne!(
        world_to_grid(p.x, p.y),
        world_to_grid(moved.x, moved.y),
        "fixture should cross a grid boundary"
    );
    moved
}

/// Mark a grid loaded so creatures are allowed to relocate into it.
fn mark_loaded(map: &Map, at: Position) {
    let (gx, gy) = world_to_grid(at.x, at.y);
    let mut grid_mgr = map.grid_manager().write();
    grid_mgr.get_or_activate_grid(gx, gy);
    grid_mgr.mark_loaded(gx, gy);
}

#[test]
fn relocation_moves_grid_membership() {
    let map = Map::new(0, 0);
    let start = pos(100.0, 100.0);
    let end = next_grid_over(start);

    mark_loaded(&map, start);
    mark_loaded(&map, end);

    let guid = ObjectGuid::new_creature(1, 1);
    map.add_creature(guid, start);
    map.relocate_creature(guid, start, end);

    let (start_gx, start_gy) = world_to_grid(start.x, start.y);
    let (end_gx, end_gy) = world_to_grid(end.x, end.y);

    // The grid it left must no longer claim it...
    let left = map.unload_grid(start_gx, start_gy);
    assert!(
        left.creatures.is_empty(),
        "creature was despawned by the grid it walked out of"
    );
    assert_eq!(map.creature_count(), 1, "creature should still be on the map");

    // ...and the grid it walked into must.
    let arrived = map.unload_grid(end_gx, end_gy);
    assert_eq!(arrived.creatures, vec![guid]);
    assert_eq!(map.creature_count(), 0);
}

#[test]
fn summoned_creature_is_in_grid_roster() {
    // Summons only ever call `add_creature` — they never went through the grid
    // load path that used to be the sole writer of the roster.
    let map = Map::new(0, 0);
    let at = pos(100.0, 100.0);
    let guid = ObjectGuid::new_creature(2, 7);

    map.add_creature(guid, at);

    let (gx, gy) = world_to_grid(at.x, at.y);
    let unloaded = map.unload_grid(gx, gy);
    assert_eq!(unloaded.creatures, vec![guid]);
}

#[test]
fn unload_clears_map_registries() {
    let map = Map::new(0, 0);
    let at = pos(100.0, 100.0);

    let creatures: Vec<_> = (1..=3).map(|i| ObjectGuid::new_creature(1, i)).collect();
    for &guid in &creatures {
        map.add_creature(guid, at);
    }
    let go = ObjectGuid::new_gameobject(10, 1);
    map.add_gameobject(go, at);
    let corpse = ObjectGuid::new_corpse(1);
    map.add_corpse(corpse, at);

    assert_eq!(map.creature_count(), 3);

    let (gx, gy) = world_to_grid(at.x, at.y);
    let unloaded = map.unload_grid(gx, gy);

    assert_eq!(unloaded.creatures.len(), 3);
    assert_eq!(unloaded.gameobjects, vec![go]);
    assert_eq!(unloaded.corpses, vec![corpse]);
    assert_eq!(unloaded.despawned_count(), 5);

    // The registries must be empty too, or range queries keep returning ghosts.
    assert_eq!(map.creature_count(), 0);
    let mut found = Vec::new();
    map.get_creatures_in_range(at, f32::MAX, &mut found);
    assert!(found.is_empty(), "despawned creatures still visible: {found:?}");
    assert!(map.get_objects_in_range(at, 500.0).is_empty());
}

#[test]
fn unload_excludes_pets_and_reports_players() {
    let map = Map::new(0, 0);
    let at = pos(100.0, 100.0);

    let player = ObjectGuid::new_player(1);
    let pet = ObjectGuid::new_pet(5, 1);
    let creature = ObjectGuid::new_creature(1, 1);

    map.add_player(player, at);
    map.add_creature(pet, at);
    map.add_creature(creature, at);

    let (gx, gy) = world_to_grid(at.x, at.y);
    let unloaded = map.unload_grid(gx, gy);

    assert_eq!(unloaded.creatures, vec![creature], "only the creature despawns");
    assert_eq!(unloaded.pets, vec![pet]);
    assert_eq!(unloaded.players, vec![player]);

    // Player and pet stay on the map and remain findable.
    assert_eq!(map.player_count(), 1);
    assert_eq!(map.creature_count(), 1);
    let visible = map.get_objects_in_range(at, 100.0);
    assert!(visible.contains(&player));
    assert!(visible.contains(&pet));
    assert!(!visible.contains(&creature));
}

#[test]
fn object_kind_of_matches_guid_high_type() {
    // `ObjectGuid::is_unit()` is true for players and pets, so classification
    // must not be built on it.
    assert_eq!(
        ObjectKind::of(ObjectGuid::new_player(1)),
        Some(ObjectKind::Player)
    );
    assert_eq!(
        ObjectKind::of(ObjectGuid::new_creature(1, 1)),
        Some(ObjectKind::Creature)
    );
    assert_eq!(
        ObjectKind::of(ObjectGuid::new_pet(1, 1)),
        Some(ObjectKind::Creature)
    );
    assert_eq!(
        ObjectKind::of(ObjectGuid::new_gameobject(1, 1)),
        Some(ObjectKind::GameObject)
    );
    assert_eq!(
        ObjectKind::of(ObjectGuid::new_corpse(1)),
        Some(ObjectKind::Corpse)
    );
}

#[test]
fn creature_may_not_relocate_into_an_unloaded_grid() {
    use oxcore_map::map::RelocateResult;

    let map = Map::new(0, 0);
    let start = pos(100.0, 100.0);
    let end = next_grid_over(start);

    mark_loaded(&map, start);
    // `end`'s grid is deliberately left unloaded.

    let guid = ObjectGuid::new_creature(1, 1);
    map.add_creature(guid, start);

    assert_eq!(
        map.relocate_creature(guid, start, end),
        RelocateResult::Refused
    );

    // A player crossing the same boundary is allowed — that is what loads grids.
    let player = ObjectGuid::new_player(1);
    map.add_player(player, start);
    assert_eq!(
        map.relocate_object(ObjectKind::Player, player, start, end),
        RelocateResult::Moved
    );
}
