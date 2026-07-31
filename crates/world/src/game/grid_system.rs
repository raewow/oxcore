//! GridSystem - coordinates grid loading/unloading operations

use crate::map::grid::GridManager;
use crate::World;
use dashmap::DashMap;
use oxcore_shared::protocol::{ObjectGuid, Position};
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

        // Re-derive the activation area from where the players actually are.
        // Relocation only does this when a player crosses a grid boundary, which
        // misses the case that matters most: walking to within the activation
        // radius of a boundary pulls the neighbouring grid into the area without
        // changing which grid the player is in.
        map.activate_grids_around_players();

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
        map: &std::sync::Arc<crate::map::Map>,
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
        map: &std::sync::Arc<crate::map::Map>,
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
                use crate::map::grid::GridState;
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

        // Load VMap/MMap tiles for this grid (before spawning creatures so pathfinding is available).
        // Tile files are named in terrain space, not the spatial grid space used
        // for object placement — convert before asking for them.
        let (tile_x, tile_y) = crate::map::grid_coords::grid_to_terrain_tile(grid_x, grid_y);
        world.managers.vmap_mgr.load_map(map_id, tile_x, tile_y);
        world
            .managers
            .mmap_mgr
            .load_map_tile(map_id, tile_x, tile_y);

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

            // Pooled spawns only exist while their pool has them on its roster
            if !world
                .systems
                .pool
                .can_creature_spawn(spawn.spawn_id, map.instance_id())
            {
                continue;
            }

            let spawned = world.managers.creature_mgr.spawn_into_map(
                &spawn,
                map,
                Some(&world.managers.waypoint_mgr),
            );

            if let Some(guid) = spawned {
                // Let the pool track the object it just got in this instance
                world
                    .systems
                    .pool
                    .on_creature_spawned(spawn.spawn_id, map.instance_id(), guid);
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

            if !world
                .systems
                .pool
                .can_gameobject_spawn(go_spawn.spawn_id, map.instance_id())
            {
                continue;
            }

            if let Some(guid) = world.managers.gameobject_mgr.spawn_gameobject(&go_spawn) {
                world.systems.pool.on_gameobject_spawned(
                    go_spawn.spawn_id,
                    map.instance_id(),
                    guid,
                );

                {
                    let grid_mgr = map.grid_manager();
                    let mut grid_mgr = grid_mgr.write();

                    map.add_object_with_grid_manager(
                        crate::map::map::ObjectKind::GameObject,
                        guid,
                        go_spawn.position,
                        &mut grid_mgr,
                    );
                }

                // Register collision so LoS, pathfinding, and height queries see
                // this object (closed doors, drawbridges, ...).
                world
                    .managers
                    .gameobject_mgr
                    .add_collision_model(guid, &world.managers.vmap_mgr);
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
        map: &std::sync::Arc<crate::map::Map>,
    ) -> anyhow::Result<()> {
        // Run the state machine, which applies the unload guards: manual and
        // active-object pins, and whether anyone is still close enough to see in.
        let pass = map.update_grid_states();

        for (grid_x, grid_y) in pass.to_unload {
            // Quiesce before draining: a creature that is mid-fight when its
            // grid disappears would otherwise be dropped still in combat, with
            // its threat list pointing at a player who is no longer there.
            // `ObjectGridStoper` (ObjectGridLoader.cpp:359-384).
            self.quiesce_grid(grid_x, grid_y, world, map);

            self.unload_grid(map_id, grid_x, grid_y, world, map).await?;
        }

        Ok(())
    }

    /// Put a creature back at its spawn point if that lies in a *different*,
    /// still-loaded grid. Returns whether it was relocated instead of despawned.
    fn move_to_home_grid(
        guid: ObjectGuid,
        unloading: (u8, u8),
        world: &World,
        map: &std::sync::Arc<crate::map::Map>,
    ) -> bool {
        use crate::map::grid_coords::world_to_grid;

        let Some((position, home)) = world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.position, c.home_position))
        else {
            return false;
        };

        let home_grid = world_to_grid(home.x, home.y);
        if home_grid == unloading {
            return false; // Belongs here; it goes away with the grid.
        }

        {
            let grid_mgr = map.grid_manager();
            let grid_mgr = grid_mgr.read();
            if !grid_mgr
                .get_grid(home_grid.0, home_grid.1)
                .is_some_and(|g| g.state().is_loaded())
            {
                return false; // Home is not loaded either — let it despawn.
            }
        }

        world.managers.creature_mgr.with_creature_mut(guid, |c| {
            c.motion_master.stop(guid);
            c.move_spline.stop();
            c.position = c.home_position;
        });
        map.add_creature(guid, home);

        tracing::debug!(
            "[GRID] Creature {:?} moved to its spawn grid ({}, {}) instead of despawning",
            guid,
            home_grid.0,
            home_grid.1
        );
        true
    }

    /// Send every creature in a grid back to its home and out of combat.
    fn quiesce_grid(
        &self,
        grid_x: u8,
        grid_y: u8,
        world: &World,
        map: &std::sync::Arc<crate::map::Map>,
    ) {
        use crate::game::creature::ai::executor::execute_actions;
        use crate::game::creature::ai::AIAction;

        for guid in map.creatures_in_grid(grid_x, grid_y) {
            execute_actions(world, guid, vec![AIAction::EnterEvadeMode]);
        }
    }

    /// Unload a specific grid and despawn its creatures
    async fn unload_grid(
        &self,
        map_id: u32,
        grid_x: u8,
        grid_y: u8,
        world: &World,
        map: &std::sync::Arc<crate::map::Map>,
    ) -> anyhow::Result<()> {
        tracing::debug!(
            "[GRID] Unloading grid ({}, {}) for map {}",
            grid_x,
            grid_y,
            map_id
        );

        // Drain the grid. This also drops the objects from the map's registries,
        // so nothing survives as a ghost in later range queries.
        let unloaded = map.unload_grid(grid_x, grid_y);

        let creature_count = unloaded.creatures.len();
        let go_count = unloaded.gameobjects.len();

        if !unloaded.players.is_empty() {
            tracing::warn!(
                "[GRID] Unloaded grid ({}, {}) on map {} with {} player(s) still in it",
                grid_x,
                grid_y,
                map_id,
                unloaded.players.len()
            );
        }

        // Despawn creatures. Pooled ones keep their roster slot, so they come
        // back when the grid is loaded again.
        //
        // A creature that wandered in from elsewhere is sent home instead of
        // despawned: its spawn point lives in another grid, which would otherwise
        // stop producing spawns until that grid itself cycled.
        // `ObjectGridRespawnMover` (ObjectGridLoader.cpp:36-81).
        let mut relocated = 0usize;
        for guid in unloaded.creatures {
            if Self::move_to_home_grid(guid, (grid_x, grid_y), world, map) {
                relocated += 1;
                continue;
            }

            world.managers.creature_mgr.save_respawn_state(guid);
            world.systems.pool.on_object_despawned(guid);
            world.managers.creature_mgr.remove_creature(guid);
        }
        let creature_count = creature_count - relocated;

        // Creatures that died and decayed are no longer in the grid, so the drain
        // above cannot see them — but their spawn point is here and their spawn
        // slot is still marked taken, which keeps a later grid load from putting
        // them back. Sweep them by spawn point so the reload spawns them fresh.
        let dead = world.managers.creature_mgr.dead_creatures_spawned_in_grid(
            map_id,
            map.instance_id(),
            (grid_x, grid_y),
        );
        for guid in dead {
            world.managers.creature_mgr.save_respawn_state(guid);
            world.systems.pool.on_object_despawned(guid);
            world.managers.creature_mgr.remove_creature(guid);
        }

        // Despawn gameobjects. Drop collision first — it is looked up through the
        // object, which `remove_gameobject` is about to discard.
        for guid in unloaded.gameobjects {
            world.systems.pool.on_object_despawned(guid);
            world
                .managers
                .gameobject_mgr
                .remove_collision_model(guid, &world.managers.vmap_mgr);
            world.managers.gameobject_mgr.remove_gameobject(guid);
        }

        // VMap/MMap tiles are deliberately *not* released per grid.
        // Unloading a map tile reparses every remaining tile to rebuild the merged
        // navmesh, and there is no per-tile unload for VMaps at all. Grid churn is
        // frequent, so both tile sets are held until the map itself is torn down
        // via `unload_map`.

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
        player_guid: oxcore_shared::protocol::ObjectGuid,
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
    pub fn are_grids_loaded(&self, map: &crate::map::Map, pos: Position) -> bool {
        use crate::map::grid_coords::world_to_grid;

        // Check exactly the grids activation covers. Using a fixed 3x3 here would
        // wait forever on grids outside the activation radius, which are never
        // created.
        let distance = map.grid_activation_distance();
        let (min_gx, min_gy) = world_to_grid(pos.x - distance, pos.y - distance);
        let (max_gx, max_gy) = world_to_grid(pos.x + distance, pos.y + distance);

        let grid_mgr = map.grid_manager();
        let grid_mgr = grid_mgr.read();

        for gx in min_gx..=max_gx {
            for gy in min_gy..=max_gy {
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
}

impl Default for GridSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::{CreatureSpawnData, CreatureTemplate};
    use crate::map::grid_coords::{world_to_grid, GRID_SIZE, MAP_HALF_SIZE};
    use oxcore_shared::database::Databases;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn template(entry: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 35,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 7,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn pos(x: f32, y: f32) -> Position {
        Position {
            x,
            y,
            z: 0.0,
            o: 0.0,
        }
    }

    /// West edge of grid `g` along either axis.
    fn grid_min(g: u8) -> f32 {
        g as f32 * GRID_SIZE - MAP_HALF_SIZE
    }

    /// A player standing near a grid boundary must get the creatures on the other
    /// side of it, not just the ones in the grid they are standing in.
    #[tokio::test]
    async fn spawns_in_the_neighbouring_grid_are_loaded() {
        let world = test_world();
        let (gx, gy) = (20u8, 20u8);

        world.managers.creature_mgr.add_template(template(70));

        // Two spawns: one in the player's grid, one just over the west boundary.
        let here = pos(grid_min(gx) + 20.0, grid_min(gy) + GRID_SIZE / 2.0);
        let next_door = pos(grid_min(gx) - 20.0, grid_min(gy) + GRID_SIZE / 2.0);
        assert_eq!(world_to_grid(here.x, here.y), (gx, gy));
        assert_eq!(world_to_grid(next_door.x, next_door.y), (gx - 1, gy));

        world.managers.creature_mgr.add_spawns_for_test(vec![
            CreatureSpawnData::new(1, 70, 0, here, 300),
            CreatureSpawnData::new(2, 70, 0, next_door, 300),
        ]);

        // Player inside the first grid, within activation range of the boundary.
        let map = world.managers.map_mgr.get_or_create_map(0, 0);
        map.add_player(ObjectGuid::new_player(1), here);

        world
            .systems
            .grid
            .process_map_grids(0, 0, &world)
            .await
            .expect("grid processing");

        assert!(
            world.managers.creature_mgr.has_spawn(1),
            "spawn in the player's own grid was not loaded"
        );
        assert!(
            world.managers.creature_mgr.has_spawn(2),
            "spawn in the neighbouring grid was not loaded"
        );
    }
}
