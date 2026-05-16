//! GridSystem - coordinates grid loading/unloading operations

use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::map::grid::GridManager;
use crate::world::World;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

/// Maximum grids to load per tick (prevents DB overload)
/// At 50ms/tick, this allows loading up to 60 grids/second per map
const MAX_GRIDS_PER_TICK: usize = 3;

/// GridSystem coordinates grid loading/unloading across maps
#[derive(Clone)]
pub struct GridSystem {
    /// Track grids currently being loaded asynchronously (map_id, grid_x, grid_y) -> task handle
    /// This prevents duplicate load requests and allows checking load status
    loading_grids: Arc<DashMap<(u32, u8, u8), tokio::task::JoinHandle<()>>>,
}

impl GridSystem {
    /// Create a new GridSystem
    pub fn new() -> Self {
        Self {
            loading_grids: Arc::new(DashMap::new()),
        }
    }

    /// Initialize the grid system
    pub async fn init(&self) -> anyhow::Result<()> {
        // Grid system initializes lazily as players enter maps
        Ok(())
    }

    /// Update the grid system (called each tick)
    pub fn update(&self, _diff: Duration) -> anyhow::Result<()> {
        // Grid loading/unloading is handled in process_map_grids
        Ok(())
    }

    /// Save respawn states for all creatures in loaded grids
    ///
    /// Should be called before shutdown to ensure creature respawn times are persisted
    pub fn save_all_creature_states(&self, world: &World) {
        tracing::debug!("[GRID] Saving creature respawn states...");

        for map_id in [0, 1] {
            let map = world.managers.map_mgr.get_continent(map_id);

            // Get all creatures from the creature manager and save their states
            // This is a simpler approach than iterating grids
            let creature_guids: Vec<_> = world
                .managers
                .creature_mgr
                .iter_creatures()
                .map(|entry| *entry.key())
                .collect();

            for guid in creature_guids {
                world.managers.creature_mgr.save_respawn_state(guid);
            }

            tracing::debug!("[GRID] Saved respawn states for map {}", map_id);
        }

        tracing::debug!("[GRID] Creature respawn states saved");
    }

    /// Shutdown the grid system
    ///
    /// Note: Call save_all_creature_states() before shutdown to persist respawn times
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        tracing::debug!("[GRID] Grid system shutdown complete");
        Ok(())
    }

    /// Process grid loading/unloading for a specific map
    pub async fn process_map_grids(
        &self,
        map_id: u32,
        instance_id: u32,
        world: &World,
    ) -> anyhow::Result<()> {
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);

        // Process grid loading
        self.load_pending_grids(map_id, world, &map).await?;

        // Process grid unloading
        self.unload_idle_grids(map_id, world, &map).await?;

        Ok(())
    }

    /// Load creatures for grids that are in Loading state
    async fn load_pending_grids(
        &self,
        map_id: u32,
        world: &World,
        map: &std::sync::Arc<crate::world::map::Map>,
    ) -> anyhow::Result<()> {
        // Get grids needing load
        let grids_to_load = {
            let grid_mgr = map.grid_manager();
            let grid_mgr = grid_mgr.read();
            grid_mgr.get_grids_needing_load()
        };

        // Throttle: only load up to MAX_GRIDS_PER_TICK grids per tick
        let grids_to_load_this_tick = grids_to_load.into_iter().take(MAX_GRIDS_PER_TICK);

        for (grid_x, grid_y) in grids_to_load_this_tick {
            self.load_grid(map_id, grid_x, grid_y, world, map).await?;
        }

        Ok(())
    }

    /// Load creatures for a specific grid
    async fn load_grid(
        &self,
        map_id: u32,
        grid_x: u8,
        grid_y: u8,
        world: &World,
        map: &std::sync::Arc<crate::world::map::Map>,
    ) -> anyhow::Result<()> {
        // Check if grid still needs loading (might have been loaded/unloaded concurrently)
        {
            let grid_mgr = map.grid_manager();
            let grid_mgr = grid_mgr.read();
            if let Some(grid) = grid_mgr.get_grid(grid_x, grid_y) {
                // Skip if already loaded
                if grid.state().is_loaded() {
                    tracing::debug!(
                        "[GRID] Grid ({}, {}) already loaded, skipping",
                        grid_x,
                        grid_y
                    );
                    return Ok(());
                }
                // Skip if being removed or invalid (player logged out)
                use crate::world::map::grid::GridState;
                if grid.state() == GridState::Removal || grid.state() == GridState::Invalid {
                    tracing::debug!(
                        "[GRID] Grid ({}, {}) no longer needs loading (state: {:?}), skipping",
                        grid_x,
                        grid_y,
                        grid.state()
                    );
                    return Ok(());
                }
            } else {
                // Grid was removed entirely
                tracing::debug!(
                    "[GRID] Grid ({}, {}) no longer exists, skipping",
                    grid_x,
                    grid_y
                );
                return Ok(());
            }
        }

        tracing::debug!(
            "[GRID] Loading grid ({}, {}) for map {}",
            grid_x,
            grid_y,
            map_id
        );

        // Load VMap/MMap tiles for this grid (before spawning creatures so pathfinding is available)
        world
            .managers
            .vmap_mgr
            .load_map(map_id, grid_x as i32, grid_y as i32);
        world
            .managers
            .mmap_mgr
            .load_map_tile(map_id, grid_x as i32, grid_y as i32);

        // Get spawns for this grid
        let spawns = world
            .managers
            .creature_mgr
            .get_spawns_for_grid(map_id, grid_x, grid_y);

        let spawn_count = spawns.len();

        // Spawn creatures
        for spawn in spawns {
            // Skip if already spawned (respawn case)
            if world.managers.creature_mgr.has_spawn(spawn.spawn_id) {
                continue;
            }

            // Spawn the creature with the map's instance_id
            if let Some(guid) = world
                .managers
                .creature_mgr
                .spawn_creature(&spawn, map.instance_id())
            {
                // Register with map and grid in a single lock acquisition
                {
                    let grid_mgr = map.grid_manager();
                    let mut grid_mgr = grid_mgr.write();

                    // Add to map's creature tracking and grid's object lists
                    map.add_creature_with_grid_manager(guid, spawn.position, &mut grid_mgr);

                    // Also register in grid for creature tracking (unload purposes)
                    grid_mgr.register_creature(guid, spawn.position.x, spawn.position.y);
                }

                // Mark as spawned in world
                world.managers.creature_mgr.with_creature_mut(guid, |c| {
                    c.in_world = true;
                });

                // Initialize movement generators based on spawn data (random wander, waypoints)
                world.managers.creature_mgr.initialize_creature_movement(
                    guid,
                    &spawn,
                    Some(&world.managers.waypoint_mgr),
                );
            }
        }

        // Spawn gameobjects for this grid
        let go_spawns = world
            .managers
            .gameobject_mgr
            .get_spawns_for_grid(map_id, grid_x, grid_y);

        for go_spawn in go_spawns {
            if world.managers.gameobject_mgr.has_spawn(go_spawn.spawn_id) {
                continue;
            }

            if let Some(guid) = world.managers.gameobject_mgr.spawn_gameobject(&go_spawn) {
                let grid_mgr = map.grid_manager();
                let mut grid_mgr = grid_mgr.write();

                map.add_gameobject_with_grid_manager(guid, go_spawn.position, &mut grid_mgr);
                grid_mgr.register_gameobject(guid, go_spawn.position.x, go_spawn.position.y);
            }
        }

        // Mark grid as loaded
        {
            let grid_mgr = map.grid_manager();
            let mut grid_mgr = grid_mgr.write();
            grid_mgr.mark_loaded(grid_x, grid_y);
        }

        Ok(())
    }

    /// Unload grids that have been idle too long
    async fn unload_idle_grids(
        &self,
        map_id: u32,
        world: &World,
        map: &std::sync::Arc<crate::world::map::Map>,
    ) -> anyhow::Result<()> {
        // Get grids to unload
        let grids_to_unload = {
            let grid_mgr = map.grid_manager();
            let grid_mgr = grid_mgr.read();
            grid_mgr.get_grids_to_unload()
        };

        for (grid_x, grid_y) in grids_to_unload {
            self.unload_grid(map_id, grid_x, grid_y, world, map).await?;
        }

        Ok(())
    }

    /// Unload a specific grid and despawn its creatures
    async fn unload_grid(
        &self,
        map_id: u32,
        grid_x: u8,
        grid_y: u8,
        world: &World,
        map: &std::sync::Arc<crate::world::map::Map>,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            "[GRID] Unloading grid ({}, {}) for map {}",
            grid_x,
            grid_y,
            map_id
        );

        // Get creatures and gameobjects to despawn
        let (creatures, gameobjects) = {
            let grid_mgr = map.grid_manager();
            let mut grid_mgr = grid_mgr.write();
            grid_mgr.unload_grid(grid_x, grid_y)
        };

        let creature_count = creatures.len();
        let go_count = gameobjects.len();

        // Despawn creatures
        for guid in creatures {
            world.managers.creature_mgr.save_respawn_state(guid);
            world.managers.creature_mgr.remove_creature(guid);
        }

        // Despawn gameobjects
        for guid in gameobjects {
            world.managers.gameobject_mgr.remove_gameobject(guid);
        }

        // Unload VMap/MMap tiles for this grid
        world
            .managers
            .mmap_mgr
            .unload_tile(map_id, grid_x as i32, grid_y as i32);

        if creature_count > 0 || go_count > 0 {
            tracing::info!(
                "[GRID] Unloaded {} creatures, {} gameobjects from grid ({}, {}) for map {}",
                creature_count,
                go_count,
                grid_x,
                grid_y,
                map_id
            );
        }

        Ok(())
    }

    /// Force load grids around a player position synchronously
    ///
    /// This is called during player login to ensure creatures are spawned
    /// BEFORE we send creature packets to the client.
    pub async fn force_load_grids_for_player(
        &self,
        player_guid: crate::shared::protocol::ObjectGuid,
        map_id: u32,
        instance_id: u32,
        world: &World,
    ) -> anyhow::Result<()> {
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);

        // Get player position from map
        let position = map.get_player_position(player_guid).ok_or_else(|| {
            anyhow::anyhow!("Player {:?} not found in map {}", player_guid, map_id)
        })?;

        tracing::info!(
            "[GRID] Force loading grids for player {:?} at ({:.1}, {:.1}) on map {}",
            player_guid,
            position.x,
            position.y,
            map_id
        );

        // Get grids needing load around the player
        let grids_to_load = {
            let grid_mgr = map.grid_manager();
            let grid_mgr = grid_mgr.read();
            grid_mgr.get_grids_needing_load()
        };

        let grid_count = grids_to_load.len();

        // Load each grid synchronously
        for (grid_x, grid_y) in grids_to_load {
            self.load_grid(map_id, grid_x, grid_y, world, &map).await?;
        }

        if grid_count > 0 {
            tracing::info!(
                "[GRID] Force loaded {} grids for player {:?}",
                grid_count,
                player_guid
            );
        }

        Ok(())
    }

    /// Trigger async grid loading for a player (returns immediately, loads in background)
    ///
    /// This version spawns background tasks to load grids without blocking the login handler.
    /// The visibility system must check are_grids_loaded() before sending creature packets.
    pub fn async_load_grids_for_player(
        &self,
        player_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        world: Arc<World>,
    ) {
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);

        // Get player position from map
        let position = match map.get_player_position(player_guid) {
            Some(pos) => pos,
            None => {
                tracing::warn!(
                    "[GRID] Cannot async load grids: player {:?} not found in map {}",
                    player_guid,
                    map_id
                );
                return;
            }
        };

        tracing::info!(
            "[GRID] Triggering async grid loading for player {:?} at ({:.1}, {:.1}) on map {}",
            player_guid,
            position.x,
            position.y,
            map_id
        );

        // Get grids needing load around the player
        let grids_to_load = {
            let grid_mgr = map.grid_manager();
            let grid_mgr = grid_mgr.read();
            grid_mgr.get_grids_needing_load()
        };

        if grids_to_load.is_empty() {
            tracing::info!(
                "[GRID] No grids need loading for player {:?} (already loaded)",
                player_guid
            );
            return;
        }

        tracing::info!(
            "[GRID] Spawning async tasks to load {} grids for player {:?}",
            grids_to_load.len(),
            player_guid
        );

        // Spawn background task for each grid
        for (grid_x, grid_y) in grids_to_load {
            let key = (map_id, grid_x, grid_y);

            // Skip if already loading
            if self.loading_grids.contains_key(&key) {
                tracing::debug!(
                    "[GRID] Grid ({}, {}) already loading, skipping",
                    grid_x,
                    grid_y
                );
                continue;
            }

            let world_clone = Arc::clone(&world);
            let map_clone = Arc::clone(&map);
            let loading_grids = Arc::clone(&self.loading_grids);
            let system = self.clone();

            let handle = tokio::spawn(async move {
                tracing::debug!(
                    "[GRID] Async loading grid ({}, {}) on map {}",
                    grid_x,
                    grid_y,
                    map_id
                );

                if let Err(e) = system
                    .load_grid(map_id, grid_x, grid_y, &world_clone, &map_clone)
                    .await
                {
                    tracing::warn!(
                        "[GRID] Async load failed for grid ({}, {}) on map {}: {}",
                        grid_x,
                        grid_y,
                        map_id,
                        e
                    );
                } else {
                    tracing::debug!(
                        "[GRID] Async load complete for grid ({}, {}) on map {}",
                        grid_x,
                        grid_y,
                        map_id
                    );
                }

                // Remove from loading set
                loading_grids.remove(&key);
            });

            self.loading_grids.insert(key, handle);
        }
    }

    /// Check if all grids around a position are fully loaded
    ///
    /// Used by visibility system to determine if it's safe to send creature packets.
    /// Returns true only if all required grids are in Loaded state.
    pub fn are_grids_loaded(&self, map: &crate::world::map::Map, pos: Position) -> bool {
        use crate::world::map::grid_coords::world_to_grid;

        let (center_gx, center_gy) = world_to_grid(pos.x, pos.y);

        let grid_mgr = map.grid_manager();
        let grid_mgr = grid_mgr.read();

        // Check 3x3 grid area around player (same as visibility range)
        for dx in -1..=1 {
            for dy in -1..=1 {
                let gx = (center_gx as i32 + dx) as u8;
                let gy = (center_gy as i32 + dy) as u8;

                // Check if grid exists and is loaded
                if let Some(grid) = grid_mgr.get_grid(gx, gy) {
                    if !grid.state().is_loaded() {
                        tracing::trace!(
                            "[GRID] Grid ({}, {}) not yet loaded (state: {:?})",
                            gx,
                            gy,
                            grid.state()
                        );
                        return false; // Still loading
                    }
                } else {
                    tracing::trace!("[GRID] Grid ({}, {}) does not exist yet", gx, gy);
                    return false; // Grid doesn't exist yet
                }
            }
        }

        true // All required grids are loaded
    }

    /// Activate grids around a player position
    /// Returns list of grids that need loading
    pub fn activate_grids_around_player(
        &self,
        map: &crate::world::map::Map,
        pos: Position,
        visibility_range: f32,
    ) -> Vec<(u8, u8)> {
        use crate::world::map::grid::MAX_GRIDS;
        use crate::world::map::grid_coords::world_to_grid;
        use crate::world::map::grid_coords::GRID_SIZE;

        let mut needs_load = Vec::new();

        let (center_gx, center_gy) = world_to_grid(pos.x, pos.y);
        let grid_range = (visibility_range / GRID_SIZE).ceil() as i32 + 1;

        let grid_mgr = map.grid_manager();
        let mut grid_mgr = grid_mgr.write();

        for dx in -grid_range..=grid_range {
            for dy in -grid_range..=grid_range {
                let gx = center_gx as i32 + dx;
                let gy = center_gy as i32 + dy;

                if gx < 0 || gx >= MAX_GRIDS as i32 || gy < 0 || gy >= MAX_GRIDS as i32 {
                    continue;
                }

                if grid_mgr.get_or_activate_grid(gx as u8, gy as u8) {
                    needs_load.push((gx as u8, gy as u8));
                }
            }
        }

        needs_load
    }
}

impl Default for GridSystem {
    fn default() -> Self {
        Self::new()
    }
}
