//! Per-tick map update orchestration.
//!
//! These functions drive the player subsystems for every active map. They live in
//! the world crate (not the map crate) because they need access to `World`'s game
//! systems; the map crate stays a World-free spatial-data leaf.

use crate::map::{Map, MapManager};
use crate::World;
use std::time::Duration;

/// Update all players on a single map (visibility + async subsystems).
pub async fn update_map(
    map: &Map,
    diff: Duration,
    current_tick: u32,
    world: &World,
) -> anyhow::Result<()> {
    let player_guids = map.player_guids();

    // Phase 1: Update visibility for all players (sync calculation).
    // Only players marked dirty, force_immediate, or throttle expired are processed.
    for &player_guid in &player_guids {
        let _ = world
            .systems
            .player
            .update_player_visibility(player_guid, current_tick, world);
    }

    // Phase 2: Update player subsystems and flush visibility notifications (async).
    for &player_guid in &player_guids {
        world
            .systems
            .player
            .update_player_async(player_guid, diff, world)
            .await?;
    }

    Ok(())
}

/// Update all active maps (async for packet sending).
pub async fn update_all_maps(
    map_mgr: &MapManager,
    diff: Duration,
    world: &World,
) -> anyhow::Result<()> {
    // Increment tick counter for visibility throttling.
    let current_tick = map_mgr.increment_tick();

    for map in map_mgr.active_maps() {
        let started = std::time::Instant::now();
        update_map(&map, diff, current_tick, world).await?;

        // Shed load by shrinking how far players see and how far grids stay
        // active when a map's tick gets expensive; grow back when it is cheap.
        map.tune_distances(started.elapsed());
    }

    Ok(())
}
