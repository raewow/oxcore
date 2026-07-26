//! Map - spatial organization for a single map instance with grid loading

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::grid::GridManager;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Visible distance on continents (`DEFAULT_VISIBILITY_DISTANCE`, ObjectDefines.h:30).
pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 100.0;
/// Visible distance inside instances (`DEFAULT_VISIBILITY_INSTANCE`).
pub const DEFAULT_VISIBILITY_INSTANCE: f32 = 170.0;
/// Visible distance in battlegrounds (`DEFAULT_VISIBILITY_BG`).
pub const DEFAULT_VISIBILITY_BG: f32 = 533.0;
/// Hard ceiling on any visibility distance — one grid (`MAX_VISIBILITY_DISTANCE`).
pub const MAX_VISIBILITY_DISTANCE: f32 = 533.33333;

/// What kind of map this is, which sets its visibility defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapKind {
    Continent,
    Dungeon,
    Raid,
    BattleGround,
}

impl MapKind {
    /// Map from `Map.dbc`'s `map_type` column (0=Common, 1=Instance, 2=Raid, 3=BG).
    pub fn from_dbc_map_type(map_type: u32) -> Self {
        match map_type {
            1 => Self::Dungeon,
            2 => Self::Raid,
            3 => Self::BattleGround,
            _ => Self::Continent,
        }
    }

    /// Continents are the only maps whose distances are tuned under load.
    pub fn is_continent(self) -> bool {
        matches!(self, Self::Continent)
    }
}

/// Per-map tuning. Injected by the world layer, which owns DBC and config; the
/// map crate has no access to either.
#[derive(Debug, Clone, Copy)]
pub struct MapConfig {
    pub kind: MapKind,
    /// How far objects are shown to a player.
    pub visibility_distance: f32,
    /// How far out grids are loaded and kept active. Tracked separately from
    /// visibility so load shedding can shrink one without the other.
    pub grid_activation_distance: f32,
    /// Floor and ceiling for the runtime ramp.
    pub min_visibility_distance: f32,
    pub max_visibility_distance: f32,
    pub min_grid_activation_distance: f32,
    pub max_grid_activation_distance: f32,
    /// Tick cost above which distances shrink, and below which they grow again.
    pub tick_lower_threshold: Duration,
    pub tick_raise_threshold: Duration,
    /// How long a grid stays idle before it may be unloaded.
    pub grid_unload_delay: Duration,
}

impl MapConfig {
    /// Reference defaults for a map kind.
    pub fn for_kind(kind: MapKind) -> Self {
        let visibility = match kind {
            MapKind::Continent => DEFAULT_VISIBILITY_DISTANCE,
            MapKind::Dungeon | MapKind::Raid => DEFAULT_VISIBILITY_INSTANCE,
            MapKind::BattleGround => DEFAULT_VISIBILITY_BG,
        };

        Self {
            kind,
            visibility_distance: visibility,
            grid_activation_distance: visibility,
            min_visibility_distance: 45.0,
            max_visibility_distance: visibility,
            min_grid_activation_distance: 45.0,
            max_grid_activation_distance: visibility,
            tick_lower_threshold: Duration::from_millis(100),
            tick_raise_threshold: Duration::from_millis(30),
            grid_unload_delay: Duration::from_secs(300),
        }
    }

    /// Override the visibility distance, keeping the ramp ceiling consistent.
    pub fn with_visibility_distance(mut self, distance: f32) -> Self {
        let distance = distance.min(MAX_VISIBILITY_DISTANCE);
        self.visibility_distance = distance;
        self.max_visibility_distance = distance;
        self.grid_activation_distance = distance;
        self.max_grid_activation_distance = distance;
        self
    }
}

impl Default for MapConfig {
    fn default() -> Self {
        Self::for_kind(MapKind::Continent)
    }
}

/// Which registry an object belongs to on a map.
///
/// Derived from the GUID's high type. Note `ObjectGuid::is_unit()` is true for
/// players and pets as well as creatures, so it must never be used to classify —
/// use this instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Player,
    /// Creatures and pets. Pets are distinguished by `ObjectGuid::is_pet()` where
    /// it matters (they are never despawned by a grid unload).
    Creature,
    GameObject,
    Corpse,
}

impl ObjectKind {
    /// Classify a GUID, or `None` for a type a map does not track.
    pub fn of(guid: ObjectGuid) -> Option<Self> {
        if guid.is_player() {
            Some(Self::Player)
        } else if guid.is_creature_or_pet() {
            Some(Self::Creature)
        } else if guid.is_game_object() {
            Some(Self::GameObject)
        } else if guid.is_corpse() {
            Some(Self::Corpse)
        } else {
            None
        }
    }
}

/// Outcome of moving an object to a new position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocateResult {
    /// The object moved.
    Moved,
    /// The move crossed into a grid that is not loaded and was rejected. The
    /// caller should send the object back to its spawn point.
    ///
    /// Mirrors `Map::CreatureCellRelocation` returning false (Map.cpp:1485-1508).
    Refused,
}

/// What a grid unload handed back, partitioned by what the caller must do.
#[derive(Debug, Default, Clone)]
pub struct UnloadedObjects {
    /// Creatures to despawn.
    pub creatures: Vec<ObjectGuid>,
    /// GameObjects to despawn.
    pub gameobjects: Vec<ObjectGuid>,
    /// Corpses/bones to drop.
    pub corpses: Vec<ObjectGuid>,
    /// Pets that were standing in the grid. Never despawned — they belong to a
    /// player, not to the grid — and are put back into the grid on unload.
    pub pets: Vec<ObjectGuid>,
    /// Players that were standing in the grid. Should always be empty; a
    /// non-empty list means a grid was unloaded out from under someone.
    pub players: Vec<ObjectGuid>,
}

impl UnloadedObjects {
    /// Total objects actually removed from the map.
    pub fn despawned_count(&self) -> usize {
        self.creatures.len() + self.gameobjects.len() + self.corpses.len()
    }
}

/// Result of one grid state machine pass.
#[derive(Debug, Default, Clone)]
pub struct GridStatePass {
    /// Grids that have passed every unload guard and may now be drained.
    pub to_unload: Vec<(u8, u8)>,
}

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
    /// Objects that keep themselves and their surroundings loaded regardless of
    /// player proximity (`Map::m_activeNonPlayers`, Map.h:693).
    active_objects: DashMap<ObjectGuid, Position>,
    /// Immutable per-map tuning, including the ramp bounds.
    config: MapConfig,
    /// Live visibility distance, as `f32::to_bits`. Atomic because it is read
    /// from every thread that queries the map and written by the tick loop.
    visibility_distance: AtomicU32,
    /// Live grid activation distance, as `f32::to_bits`.
    grid_activation_distance: AtomicU32,
}

impl Map {
    /// Create a map with the default (continent) tuning.
    pub fn new(map_id: u32, instance_id: u32) -> Self {
        Self::with_config(map_id, instance_id, MapConfig::default())
    }

    /// Create a map with explicit tuning.
    pub fn with_config(map_id: u32, instance_id: u32, config: MapConfig) -> Self {
        Self {
            map_id,
            instance_id,
            grid_manager: RwLock::new(GridManager::new()),
            players: DashMap::new(),
            creatures: DashMap::new(),
            gameobjects: DashMap::new(),
            corpses: DashMap::new(),
            active_objects: DashMap::new(),
            visibility_distance: AtomicU32::new(config.visibility_distance.to_bits()),
            grid_activation_distance: AtomicU32::new(config.grid_activation_distance.to_bits()),
            config,
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

    /// This map's immutable tuning.
    pub fn config(&self) -> &MapConfig {
        &self.config
    }

    /// What kind of map this is.
    pub fn kind(&self) -> MapKind {
        self.config.kind
    }

    /// Current visibility distance — how far objects are shown to a player.
    pub fn visibility_distance(&self) -> f32 {
        f32::from_bits(self.visibility_distance.load(Ordering::Relaxed))
    }

    /// Current grid activation distance — how far out grids are loaded.
    pub fn grid_activation_distance(&self) -> f32 {
        f32::from_bits(self.grid_activation_distance.load(Ordering::Relaxed))
    }

    /// Adjust both distances against the cost of the last tick.
    ///
    /// Port of the load-shedding ramp at the tail of `Map::Update`
    /// (Map.cpp:1052-1081): one yard per tick, clamped to the configured bounds,
    /// continents only.
    pub fn tune_distances(&self, tick_cost: Duration) {
        if !self.config.kind.is_continent() {
            return;
        }

        let step = |current: f32, min: f32, max: f32| -> f32 {
            if tick_cost > self.config.tick_lower_threshold {
                (current - 1.0).max(min)
            } else if tick_cost < self.config.tick_raise_threshold {
                (current + 1.0).min(max)
            } else {
                current
            }
        };

        let vis = step(
            self.visibility_distance(),
            self.config.min_visibility_distance,
            self.config.max_visibility_distance,
        );
        let grid = step(
            self.grid_activation_distance(),
            self.config.min_grid_activation_distance,
            self.config.max_grid_activation_distance,
        );

        self.visibility_distance
            .store(vis.to_bits(), Ordering::Relaxed);
        self.grid_activation_distance
            .store(grid.to_bits(), Ordering::Relaxed);
    }

    /// Get grid manager (read)
    pub fn grid_manager(&self) -> &RwLock<GridManager> {
        &self.grid_manager
    }

    /// The registry holding objects of `kind`.
    fn registry(&self, kind: ObjectKind) -> &DashMap<ObjectGuid, Position> {
        match kind {
            ObjectKind::Player => &self.players,
            ObjectKind::Creature => &self.creatures,
            ObjectKind::GameObject => &self.gameobjects,
            ObjectKind::Corpse => &self.corpses,
        }
    }

    // -- The three choke points -------------------------------------------------
    //
    // Every object entering, leaving or moving on a map goes through one of
    // these, so registry and grid membership cannot drift apart.

    /// Put an object on the map.
    pub fn add_object(&self, kind: ObjectKind, guid: ObjectGuid, position: Position) {
        self.registry(kind).insert(guid, position);

        {
            let mut grid_mgr = self.grid_manager.write();
            grid_mgr.add_object(guid, position.x, position.y);
        }

        if kind == ObjectKind::Player {
            self.activate_grids_around_position(position);
        }
    }

    /// Put an object on the map using a grid lock the caller already holds.
    ///
    /// Avoids re-taking the write lock per object while a grid is being filled.
    pub fn add_object_with_grid_manager(
        &self,
        kind: ObjectKind,
        guid: ObjectGuid,
        position: Position,
        grid_mgr: &mut GridManager,
    ) {
        self.registry(kind).insert(guid, position);
        grid_mgr.add_object(guid, position.x, position.y);
    }

    /// Take an object off the map.
    pub fn remove_object(&self, kind: ObjectKind, guid: ObjectGuid, position: Position) {
        // Prefer the recorded position: the caller's copy may be stale, and
        // removing against the wrong cell would leave the GUID in the grid.
        let pos = self
            .registry(kind)
            .remove(&guid)
            .map(|(_, p)| p)
            .unwrap_or(position);

        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.remove_object(guid, pos.x, pos.y);
    }

    /// Move an object, updating its grid and cell membership.
    ///
    /// Returns [`RelocateResult::Refused`] when a creature would cross into a
    /// grid that is not loaded; the caller is expected to send it home rather
    /// than leave it standing in unloaded space.
    pub fn relocate_object(
        &self,
        kind: ObjectKind,
        guid: ObjectGuid,
        old_pos: Position,
        new_pos: Position,
    ) -> RelocateResult {
        use crate::grid_coords::{world_to_cell, world_to_grid};

        let old_grid = world_to_grid(old_pos.x, old_pos.y);
        let new_grid = world_to_grid(new_pos.x, new_pos.y);

        if old_grid != new_grid {
            // Creatures may not walk into unloaded space. Players and their pets
            // may — a player entering a grid is what triggers loading it.
            if kind == ObjectKind::Creature && !guid.is_pet() {
                let loaded = {
                    let grid_mgr = self.grid_manager.read();
                    grid_mgr
                        .get_grid(new_grid.0, new_grid.1)
                        .is_some_and(|g| g.state().is_loaded())
                };
                if !loaded {
                    return RelocateResult::Refused;
                }
            }

            {
                let mut grid_mgr = self.grid_manager.write();
                grid_mgr.remove_object(guid, old_pos.x, old_pos.y);
                grid_mgr.add_object(guid, new_pos.x, new_pos.y);
            }

            if kind == ObjectKind::Player {
                self.activate_grids_around_position(new_pos);
            }
        } else {
            // Same grid - only acquire write lock if cell actually changed
            let old_cell = world_to_cell(old_pos.x, old_pos.y);
            let new_cell = world_to_cell(new_pos.x, new_pos.y);
            if old_cell != new_cell {
                let mut grid_mgr = self.grid_manager.write();
                grid_mgr.relocate_object(guid, old_pos.x, old_pos.y, new_pos.x, new_pos.y);
            }
        }

        self.registry(kind).insert(guid, new_pos);
        RelocateResult::Moved
    }

    /// Unload a grid: drain its objects, drop the despawnable ones from the map's
    /// registries, and return everything partitioned by what the caller must do.
    ///
    /// Players and pets stay on the map and are put back into the grid.
    pub fn unload_grid(&self, grid_x: u8, grid_y: u8) -> UnloadedObjects {
        let objects = {
            let mut grid_mgr = self.grid_manager.write();
            grid_mgr.drain_grid(grid_x, grid_y)
        };

        let mut out = UnloadedObjects::default();
        let mut keep = Vec::new();

        for guid in objects {
            match ObjectKind::of(guid) {
                Some(ObjectKind::Player) => {
                    out.players.push(guid);
                    keep.push((ObjectKind::Player, guid));
                }
                Some(ObjectKind::Creature) if guid.is_pet() => {
                    out.pets.push(guid);
                    keep.push((ObjectKind::Creature, guid));
                }
                Some(ObjectKind::Creature) => {
                    self.creatures.remove(&guid);
                    out.creatures.push(guid);
                }
                Some(ObjectKind::GameObject) => {
                    self.gameobjects.remove(&guid);
                    out.gameobjects.push(guid);
                }
                Some(ObjectKind::Corpse) => {
                    self.corpses.remove(&guid);
                    out.corpses.push(guid);
                }
                None => {}
            }
        }

        // Restore whatever outlives the grid, so it stays findable by range
        // queries even though the grid was reset.
        if !keep.is_empty() {
            let mut grid_mgr = self.grid_manager.write();
            for (kind, guid) in keep {
                if let Some(pos) = self.registry(kind).get(&guid).map(|p| *p.value()) {
                    grid_mgr.add_object(guid, pos.x, pos.y);
                }
            }
        }

        out
    }

    // -- Grid lifecycle ---------------------------------------------------------

    /// Mark an object as active: it ticks and stays visible regardless of player
    /// proximity, and pins the grid holding its spawn point.
    ///
    /// `Map::AddToActive` (Map.cpp:1924-1948). `spawn_pos` is the object's
    /// *respawn* point, which is what the reference pins — reloading a grid whose
    /// active spawn is elsewhere would clone it.
    pub fn add_to_active(&self, guid: ObjectGuid, spawn_pos: Position) {
        use crate::grid_coords::world_to_grid;

        if self.active_objects.insert(guid, spawn_pos).is_some() {
            return; // already active
        }

        let (gx, gy) = world_to_grid(spawn_pos.x, spawn_pos.y);
        let mut grid_mgr = self.grid_manager.write();
        grid_mgr.get_or_create_grid(gx, gy).inc_unload_active_lock();
    }

    /// Stop treating an object as active and release its spawn-grid pin.
    ///
    /// `Map::RemoveFromActive` (Map.cpp:1950-1984).
    pub fn remove_from_active(&self, guid: ObjectGuid) {
        use crate::grid_coords::world_to_grid;

        let Some((_, spawn_pos)) = self.active_objects.remove(&guid) else {
            return;
        };

        let (gx, gy) = world_to_grid(spawn_pos.x, spawn_pos.y);
        let mut grid_mgr = self.grid_manager.write();
        if let Some(grid) = grid_mgr.get_grid_mut(gx, gy) {
            grid.dec_unload_active_lock();
        }
    }

    /// Whether a grid must stay loaded because a player or active object is close
    /// enough to see into it.
    ///
    /// Expands the grid's cell box by the visibility radius and tests every
    /// player and active object against it. Port of `Map::ActiveObjectsNearGrid`
    /// (Map.cpp:1886-1922) — without this a player standing just over a boundary
    /// does not keep the neighbouring grid alive, and it unloads under them.
    ///
    /// Must not be called while the grid lock is held.
    pub fn active_objects_near_grid(&self, grid_x: u8, grid_y: u8) -> bool {
        use crate::grid_coords::{world_to_cell, CELLS_PER_GRID, CELL_SIZE};

        const MAX_CELL: i32 = 1023;

        let cells_per_grid = CELLS_PER_GRID as i32;
        let range = (self.visibility_distance() / CELL_SIZE).ceil() as i32 + 1;

        let min_x = (grid_x as i32 * cells_per_grid - range).max(0);
        let min_y = (grid_y as i32 * cells_per_grid - range).max(0);
        let max_x = (grid_x as i32 * cells_per_grid + cells_per_grid + range).min(MAX_CELL);
        let max_y = (grid_y as i32 * cells_per_grid + cells_per_grid + range).min(MAX_CELL);

        let in_box = |pos: &Position| {
            let (cx, cy) = world_to_cell(pos.x, pos.y);
            let (cx, cy) = (cx as i32, cy as i32);
            cx >= min_x && cx <= max_x && cy >= min_y && cy <= max_y
        };

        self.players.iter().any(|e| in_box(e.value()))
            || self.active_objects.iter().any(|e| in_box(e.value()))
    }

    /// Restart a grid's idle countdown so it survives while someone is using it.
    pub fn reset_grid_expiry(&self, grid_x: u8, grid_y: u8) {
        let mut grid_mgr = self.grid_manager.write();
        if let Some(grid) = grid_mgr.get_grid_mut(grid_x, grid_y) {
            grid.reset_idle_timer();
        }
    }

    /// Run one pass of the grid state machine.
    ///
    /// Returns the work the world layer must do: creatures to quiesce in grids
    /// that just went idle, and grids cleared for unload. Ported from
    /// `GridStates.cpp:32-73`, with all three of the reference's guards —
    /// unload locks, nearby players/active objects, and the idle timer.
    pub fn update_grid_states(&self) -> GridStatePass {
        let mut pass = GridStatePass::default();
        let delay = self.config.grid_unload_delay;

        let idle_candidates: Vec<(u8, u8)> = {
            let grid_mgr = self.grid_manager.read();
            grid_mgr.get_grids_to_unload(delay)
        };

        for (gx, gy) in idle_candidates {
            // Cheap, lock-free guards first.
            {
                let grid_mgr = self.grid_manager.read();
                if grid_mgr.get_grid(gx, gy).is_some_and(|g| g.unload_locked()) {
                    continue;
                }
            }

            // Then the expensive one, with no grid lock held.
            if self.active_objects_near_grid(gx, gy) {
                // Someone is still close enough to see in — restart the timer so
                // we do not re-test this grid every tick.
                self.reset_grid_expiry(gx, gy);
                continue;
            }

            pass.to_unload.push((gx, gy));
        }

        pass
    }

    /// Creatures currently inside a grid, for callers that need to act on them
    /// without unloading the grid (quiescing an idle grid).
    pub fn creatures_in_grid(&self, grid_x: u8, grid_y: u8) -> Vec<ObjectGuid> {
        let grid_mgr = self.grid_manager.read();
        grid_mgr
            .get_grid(grid_x, grid_y)
            .map(|grid| {
                grid.objects()
                    .iter()
                    .copied()
                    .filter(|g| g.is_creature())
                    .collect()
            })
            .unwrap_or_default()
    }

    // -- Typed wrappers ---------------------------------------------------------

    /// Add a player to the map - activates grids around player
    #[inline]
    pub fn add_player(&self, guid: ObjectGuid, position: Position) {
        self.add_object(ObjectKind::Player, guid, position);
    }

    /// Remove a player from the map
    #[inline]
    pub fn remove_player(&self, guid: ObjectGuid) {
        let pos = self
            .players
            .get(&guid)
            .map(|p| *p.value())
            .unwrap_or_default();
        self.remove_object(ObjectKind::Player, guid, pos);
    }

    /// Update player position - handles grid changes
    #[inline]
    pub fn relocate_player(&self, guid: ObjectGuid, old_pos: Position, new_pos: Position) {
        self.relocate_object(ObjectKind::Player, guid, old_pos, new_pos);
    }

    /// Update creature position - handles grid changes
    #[inline]
    pub fn relocate_creature(
        &self,
        guid: ObjectGuid,
        old_pos: Position,
        new_pos: Position,
    ) -> RelocateResult {
        self.relocate_object(ObjectKind::Creature, guid, old_pos, new_pos)
    }

    /// Add a creature to the map
    #[inline]
    pub fn add_creature(&self, guid: ObjectGuid, position: Position) {
        self.add_object(ObjectKind::Creature, guid, position);
    }

    /// Remove a creature from the map
    #[inline]
    pub fn remove_creature(&self, guid: ObjectGuid, position: Position) {
        self.remove_object(ObjectKind::Creature, guid, position);
    }

    /// Add a gameobject to the map
    #[inline]
    pub fn add_gameobject(&self, guid: ObjectGuid, position: Position) {
        self.add_object(ObjectKind::GameObject, guid, position);
    }

    /// Remove a gameobject from the map
    #[inline]
    pub fn remove_gameobject(&self, guid: ObjectGuid, position: Position) {
        self.remove_object(ObjectKind::GameObject, guid, position);
    }

    /// Add a corpse to the map
    #[inline]
    pub fn add_corpse(&self, guid: ObjectGuid, position: Position) {
        self.add_object(ObjectKind::Corpse, guid, position);
    }

    /// Remove a corpse from the map
    #[inline]
    pub fn remove_corpse(&self, guid: ObjectGuid, position: Position) {
        self.remove_object(ObjectKind::Corpse, guid, position);
    }

    /// Activate the grids covering the activation radius around a position.
    ///
    /// The box is derived from the corners of the radius, as
    /// `Cell::CalculateCellArea` does (CellImpl.h:39-52). An earlier version used
    /// `ceil(distance / GRID_SIZE) + 1` as a half-width, which is a cell-level
    /// expansion misapplied at grid level: it activated 5x5 = 25 grids per player
    /// even at short distances, spawning roughly an order of magnitude more
    /// creatures than the reference.
    fn activate_grids_around_position(&self, pos: Position) {
        use crate::grid_coords::world_to_grid;

        let distance = self.grid_activation_distance();
        let (center_gx, center_gy) = world_to_grid(pos.x, pos.y);
        let (min_gx, min_gy) = world_to_grid(pos.x - distance, pos.y - distance);
        let (max_gx, max_gy) = world_to_grid(pos.x + distance, pos.y + distance);

        let mut grid_mgr = self.grid_manager.write();

        for gx in min_gx..=max_gx {
            for gy in min_gy..=max_gy {
                // Mark the grid for loading if needed
                let needs_load = grid_mgr.get_or_activate_grid(gx, gy);

                // Anyone standing within activation range keeps the grid alive,
                // even when they are in the grid next door (Map.cpp:1419-1423).
                if let Some(grid) = grid_mgr.get_grid_mut(gx, gy) {
                    grid.reset_idle_timer();
                }

                // Set priority based on distance from the player's own grid
                if needs_load {
                    if let Some(grid) = grid_mgr.get_grid_mut(gx, gy) {
                        let dx = gx as i32 - center_gx as i32;
                        let dy = gy as i32 - center_gy as i32;
                        let distance_sq = dx * dx + dy * dy;
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
    pub fn get_players_in_range_limit(
        &self,
        center: Position,
        range: f32,
        max_results: usize,
    ) -> Vec<ObjectGuid> {
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
    pub fn get_creatures_in_range(
        &self,
        center: Position,
        range_sq: f32,
        result: &mut Vec<ObjectGuid>,
    ) {
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

    /// Snapshot of all player GUIDs currently on this map.
    pub fn player_guids(&self) -> Vec<ObjectGuid> {
        self.players.iter().map(|r| *r.key()).collect()
    }
}
