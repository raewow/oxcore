//! Map - spatial organization for a single map instance with grid loading

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;

use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::map::grid::GridManager;

/// Default visibility distance (one grid = 533.33 units)
pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 533.33333;

/// A single map instance with lazy grid loading
pub struct Map {
    /// Map ID (0 = Eastern Kingdoms, 1 = Kalimdor, etc.)
    map_id: u32,
    /// Instance ID (0 for continents)
    instance_id: u32,
    /// Grid manager
    grid_manager: RwLock<GridManager>,
    /// Players on this map
    players: DashMap<ObjectGuid, Position>,
    /// Creatures on this map
    creatures: DashMap<ObjectGuid, Position>,
    /// GameObjects on this map
    gameobjects: DashMap<ObjectGuid, Position>,
    /// Corpses on this map (player death bodies + bones). Tracked in the same
    /// grid as creatures/gameobjects so visibility queries pick them up.
    corpses: DashMap<ObjectGuid, Position>,
    /// Visibility distance
    visibility_distance: f32,
}

impl Map {
    /// Create a new map
    pub fn new(map_id: u32, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
            grid_manager: RwLock::new(GridManager::new()),
            players: DashMap::new(),
            creatures: DashMap::new(),
            gameobjects: DashMap::new(),
            corpses: DashMap::new(),
            visibility_distance: DEFAULT_VISIBILITY_DISTANCE,
        }
    }

    /// Get map ID
    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    /// Get instance ID
    pub fn instance_id(&self) -> u32 {
        self.instance_id
    }

    /// Get visibility distance
    pub fn visibility_distance(&self) -> f32 {
        self.visibility_distance
    }

    /// Get grid manager (read)
    pub fn grid_manager(&self) -> &RwLock<GridManager> {
        &self.grid_manager
    }

    /// Add a player to the map - activates grids around player
    pub fn add_player(&self, guid: ObjectGuid, position: Position) {
        self.players.insert(guid, position);

        // Add to grid
        {
            let mut grid_mgr = self.grid_manager.write();
            grid_mgr.add_object(guid, position.x, position.y);
        }

        // Activate grids around player
        self.activate_grids_around_position(position);
    }

    /// Remove a player from the map
    pub fn remove_player(&self, guid: ObjectGuid) {
        if let Some((_, pos)) = self.players.remove(&guid) {
            let mut grid_mgr = self.grid_manager.write();
            grid_mgr.remove_object(guid, pos.x, pos.y);
        }
    }

    /// Update player position - handles grid changes
    pub fn relocate_player(&self, guid: ObjectGuid, old_pos: Position, new_pos: Position) {
        use crate::world::map::grid_coords::{world_to_grid, world_to_cell};

        let old_grid = world_to_grid(old_pos.x, old_pos.y);
        let new_grid = world_to_grid(new_pos.x, new_pos.y);

        if old_grid != new_grid {
            // Grid changed - remove from old, add to new
            {
                let mut grid_mgr = self.grid_manager.write();
                grid_mgr.remove_object(guid, old_pos.x, old_pos.y);
                grid_mgr.add_object(guid, new_pos.x, new_pos.y);
            }

            // Activate grids around new position
            self.activate_grids_around_position(new_pos);
        } else {
            // Same grid - only acquire write lock if cell actually changed
            let old_cell = world_to_cell(old_pos.x, old_pos.y);
            let new_cell = world_to_cell(new_pos.x, new_pos.y);
            if old_cell != new_cell {
                let mut grid_mgr = self.grid_manager.write();
                grid_mgr.relocate_object(guid, old_pos.x, old_pos.y, new_pos.x, new_pos.y);
            }
        }

        self.players.insert(guid, new_pos);
    }

    /// Update creature position - handles grid changes
    pub fn relocate_creature(&self, guid: ObjectGuid, old_pos: Position, new_pos: Position) {
        use crate::world::map::grid_coords::{world_to_grid, world_to_cell};

        let old_grid = world_to_grid(old_pos.x, old_pos.y);
        let new_grid = world_to_grid(new_pos.x, new_pos.y);

        if old_grid != new_grid {
            let mut grid_mgr = self.grid_manager.write();
            grid_mgr.remove_object(guid, old_pos.x, old_pos.y);
            grid_mgr.add_object(guid, new_pos.x, new_pos.y);
        } else {
            let old_cell = world_to_cell(old_pos.x, old_pos.y);
            let new_cell = world_to_cell(new_pos.x, new_pos.y);
            if old_cell != new_cell {
                let mut grid_mgr = self.grid_manager.write();
                grid_mgr.relocate_object(guid, old_pos.x, old_pos.y, new_pos.x, new_pos.y);
            }
        }

        self.creatures.insert(guid, new_pos);
    }

    /// Add a creature to the map
    pub fn add_creature(&self, guid: ObjectGuid, position: Position) {
        self.creatures.insert(guid, position);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.add_object(guid, position.x, position.y);
    }

    /// Add a creature to the map with an existing grid_manager lock
    /// This avoids double-locking when spawning creatures during grid loading
    pub fn add_creature_with_grid_manager(
        &self,
        guid: ObjectGuid,
        position: Position,
        grid_mgr: &mut GridManager,
    ) {
        self.creatures.insert(guid, position);
        grid_mgr.add_object(guid, position.x, position.y);
    }

    /// Remove a creature from the map
    pub fn remove_creature(&self, guid: ObjectGuid, position: Position) {
        self.creatures.remove(&guid);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.remove_object(guid, position.x, position.y);
    }

    /// Add a gameobject to the map
    pub fn add_gameobject(&self, guid: ObjectGuid, position: Position) {
        self.gameobjects.insert(guid, position);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.add_object(guid, position.x, position.y);
    }

    /// Add a gameobject to the map with an existing grid_manager lock
    pub fn add_gameobject_with_grid_manager(
        &self,
        guid: ObjectGuid,
        position: Position,
        grid_mgr: &mut GridManager,
    ) {
        self.gameobjects.insert(guid, position);
        grid_mgr.add_object(guid, position.x, position.y);
    }

    /// Remove a gameobject from the map
    pub fn remove_gameobject(&self, guid: ObjectGuid, position: Position) {
        self.gameobjects.remove(&guid);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.remove_object(guid, position.x, position.y);
    }

    /// Add a corpse to the map
    pub fn add_corpse(&self, guid: ObjectGuid, position: Position) {
        self.corpses.insert(guid, position);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.add_object(guid, position.x, position.y);
    }

    /// Remove a corpse from the map
    pub fn remove_corpse(&self, guid: ObjectGuid, position: Position) {
        self.corpses.remove(&guid);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.remove_object(guid, position.x, position.y);
    }

    /// Activate grids around a position
    fn activate_grids_around_position(&self, pos: Position) {
        use crate::world::map::grid_coords::{world_to_grid, GRID_SIZE};
        use crate::world::map::grid::MAX_GRIDS;

        let (center_gx, center_gy) = world_to_grid(pos.x, pos.y);
        let grid_range = (self.visibility_distance / GRID_SIZE).ceil() as i32 + 1;

        let mut grid_mgr = self.grid_manager.write();

        for dx in -grid_range..=grid_range {
            for dy in -grid_range..=grid_range {
                let gx = center_gx as i32 + dx;
                let gy = center_gy as i32 + dy;

                if gx < 0 || gx >= MAX_GRIDS as i32 || gy < 0 || gy >= MAX_GRIDS as i32 {
                    continue;
                }

                // Mark the grid for loading if needed
                let needs_load = grid_mgr.get_or_activate_grid(gx as u8, gy as u8);

                // Set priority based on distance from player
                if needs_load {
                    if let Some(grid) = grid_mgr.get_grid_mut(gx as u8, gy as u8) {
                        let distance_sq = (dx * dx + dy * dy) as u8;
                        // Priority: 100 for player's grid, decreases with distance
                        let priority = if distance_sq == 0 {
                            100 // Player's current grid
                        } else if distance_sq <= 2 {
                            50 // Adjacent grids (distance 1 or sqrt(2))
                        } else {
                            25 // Farther grids
                        };
                        grid.set_loading_priority(priority);
                    }
                }
            }
        }
    }

    /// Get all objects within range of a position
    pub fn get_objects_in_range(&self, center: Position, range: f32) -> HashSet<ObjectGuid> {
        let grid_mgr = self.grid_manager.read();
        grid_mgr.get_objects_in_range(center.x, center.y, range)
    }

    /// Get all players within range
    pub fn get_players_in_range(&self, center: Position, range: f32) -> Vec<ObjectGuid> {
        let range_sq = range * range;
        self.players
            .iter()
            .filter(|r| {
                let pos = r.value();
                let dx = pos.x - center.x;
                let dy = pos.y - center.y;
                dx * dx + dy * dy <= range_sq
            })
            .map(|r| *r.key())
            .collect()
    }

    /// Get players within range with early-exit optimization
    /// Limits result to max_results to avoid full map scans
    /// Use this for broadcasts where you don't need ALL players, just nearby ones
    pub fn get_players_in_range_limit(&self, center: Position, range: f32, max_results: usize) -> Vec<ObjectGuid> {
        let range_sq = range * range;
        let mut result = Vec::with_capacity(max_results.min(20));

        for entry in self.players.iter() {
            if result.len() >= max_results {
                break; // Early exit - stop scanning once we have enough
            }

            let pos = entry.value();
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            if dx * dx + dy * dy <= range_sq {
                result.push(*entry.key());
            }
        }

        result
    }

    /// Get all creatures within range and append to result vector
    pub fn get_creatures_in_range(&self, center: Position, range_sq: f32, result: &mut Vec<ObjectGuid>) {
        for entry in self.creatures.iter() {
            let pos = entry.value();
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            if dx * dx + dy * dy <= range_sq {
                result.push(*entry.key());
            }
        }
    }

    /// Get a player's position
    pub fn get_player_position(&self, guid: ObjectGuid) -> Option<Position> {
        self.players.get(&guid).map(|r| *r.value())
    }

    /// Get player count
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Get creature count
    pub fn creature_count(&self) -> usize {
        self.creatures.len()
    }

    /// Update all players on this map (async for packet sending)
    pub async fn update(
        &self,
        diff: std::time::Duration,
        current_tick: u32,
        world: &crate::world::World,
    ) -> anyhow::Result<()> {
        // Collect player GUIDs to avoid holding iterator across await
        let player_guids: Vec<ObjectGuid> = self.players.iter().map(|r| *r.key()).collect();

        // Phase 1: Update visibility for all players (sync calculation)
        // Only players marked dirty, force_immediate, or throttle expired will be processed
        for &player_guid in &player_guids {
            let _ = world
                .systems
                .player
                .update_player_visibility(player_guid, current_tick, world);
        }

        // Phase 2: Update player subsystems and flush visibility notifications (async)
        for &player_guid in &player_guids {
            world
                .systems
                .player
                .update_player_async(player_guid, diff, world)
                .await?;
        }

        Ok(())
    }
}
