//! Grid and GridManager for spatial organization with lazy loading

use smallvec::SmallVec;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::grid::Cell;
use crate::grid::GridState;
use crate::grid_coords::{world_to_cell_in_grid, world_to_grid};
use oxcore_shared::protocol::ObjectGuid;

/// Number of grids per side (64x64 = 4096 grids)
pub const MAX_GRIDS: usize = 64;

/// Number of cells per grid side (16x16 = 256 cells per grid)
pub const CELLS_PER_GRID: usize = 16;

/// Default idle time before a grid may be unloaded. Overridable per map via
/// `MapConfig::grid_unload_delay`.
pub const DEFAULT_GRID_UNLOAD_DELAY: Duration = Duration::from_secs(300);

/// A single grid (533.33 x 533.33 units) with state machine
pub struct Grid {
    grid_x: u8,
    grid_y: u8,
    /// Current state in the loading lifecycle
    state: GridState,
    /// 16x16 cells
    cells: Box<[[Cell; CELLS_PER_GRID]; CELLS_PER_GRID]>,
    /// All objects in this grid (for fast iteration).
    ///
    /// This is the single source of truth for grid membership: it is what range
    /// queries read and what unload drains. There is deliberately no separate
    /// per-type roster — one existed and silently desynced from this list on
    /// every relocation, so creatures were despawned by grids they had left.
    objects: SmallVec<[ObjectGuid; 16]>,
    /// Player count (for unload decisions)
    player_count: u32,
    /// When grid became idle (for timeout)
    idle_since: Option<Instant>,
    /// Loading priority (higher = load first)
    loading_priority: u8,
    /// Manual pin, e.g. a grid holding objects that must never despawn.
    /// `GridInfo::i_unloadExplicitLock` (NGrid.h:55).
    unload_explicit_lock: bool,
    /// Number of active objects whose spawn point is in this grid. Keeps the
    /// grid loaded so reloading it cannot clone them.
    /// `GridInfo::i_unloadActiveLockCount` (NGrid.h:56).
    unload_active_lock: u16,
}

impl Grid {
    /// Create a new grid in Invalid state
    pub fn new(grid_x: u8, grid_y: u8) -> Self {
        use std::mem::MaybeUninit;

        // Initialize cells array directly on heap to avoid stack overflow
        let mut cells: Box<MaybeUninit<[[Cell; CELLS_PER_GRID]; CELLS_PER_GRID]>> =
            Box::new_uninit();

        unsafe {
            // Initialize each cell element by element
            let ptr = cells.as_mut_ptr();
            for i in 0..CELLS_PER_GRID {
                let row_ptr = (*ptr)[i].as_mut_ptr();
                for j in 0..CELLS_PER_GRID {
                    row_ptr.add(j).write(Cell::new());
                }
            }
            // SAFETY: All elements have been initialized
            let cells = cells.assume_init();

            Self {
                grid_x,
                grid_y,
                state: GridState::Invalid,
                cells,
                objects: SmallVec::new(),
                player_count: 0,
                idle_since: None,
                loading_priority: 0,
                unload_explicit_lock: false,
                unload_active_lock: 0,
            }
        }
    }

    /// Get grid X coordinate
    pub fn grid_x(&self) -> u8 {
        self.grid_x
    }

    /// Get grid Y coordinate
    pub fn grid_y(&self) -> u8 {
        self.grid_y
    }

    /// Get current state
    pub fn state(&self) -> GridState {
        self.state
    }

    /// Set grid state
    pub fn set_state(&mut self, state: GridState) {
        self.state = state;
    }

    /// Get mutable cell
    fn get_cell_mut(&mut self, cell_x: u8, cell_y: u8) -> Option<&mut Cell> {
        if cell_x < CELLS_PER_GRID as u8 && cell_y < CELLS_PER_GRID as u8 {
            Some(&mut self.cells[cell_x as usize][cell_y as usize])
        } else {
            None
        }
    }

    /// Get cell (read-only)
    pub fn get_cell(&self, cell_x: u8, cell_y: u8) -> Option<&Cell> {
        if cell_x < CELLS_PER_GRID as u8 && cell_y < CELLS_PER_GRID as u8 {
            Some(&self.cells[cell_x as usize][cell_y as usize])
        } else {
            None
        }
    }

    /// Add an object to the grid
    pub fn add_object(&mut self, guid: ObjectGuid, cell_x: u8, cell_y: u8) {
        if !self.objects.contains(&guid) {
            self.objects.push(guid);
        }
        if let Some(cell) = self.get_cell_mut(cell_x, cell_y) {
            cell.add_object(guid);
        }

        // Track player count and activation
        if guid.is_player() {
            self.player_count += 1;
            self.idle_since = None;
            // Transition from Idle to Active if player enters
            if self.state == GridState::Idle {
                self.state = GridState::Active;
            }
        }
    }

    /// Remove an object from the grid
    pub fn remove_object(&mut self, guid: ObjectGuid, cell_x: u8, cell_y: u8) {
        self.objects.retain(|g| *g != guid);
        if let Some(cell) = self.get_cell_mut(cell_x, cell_y) {
            cell.remove_object(guid);
        }

        // Track player count and possible deactivation
        if guid.is_player() {
            self.player_count = self.player_count.saturating_sub(1);
            // Transition from Active to Idle when last player leaves
            if self.player_count == 0 && self.state == GridState::Active {
                self.state = GridState::Idle;
                self.idle_since = Some(Instant::now());
            }
        }
    }

    /// Get all objects in grid
    pub fn objects(&self) -> &[ObjectGuid] {
        &self.objects
    }

    /// Increment player count
    pub fn increment_player_count(&mut self) {
        self.player_count += 1;
        self.idle_since = None;
        if self.state == GridState::Idle {
            self.state = GridState::Active;
        }
    }

    /// Decrement player count
    pub fn decrement_player_count(&mut self) {
        self.player_count = self.player_count.saturating_sub(1);
        if self.player_count == 0 && self.state == GridState::Active {
            self.state = GridState::Idle;
            self.idle_since = Some(Instant::now());
        }
    }

    /// Check if grid has players
    pub fn has_players(&self) -> bool {
        self.player_count > 0
    }

    /// Get player count
    pub fn player_count(&self) -> u32 {
        self.player_count
    }

    /// Whether anything is pinning this grid against unload.
    pub fn unload_locked(&self) -> bool {
        self.unload_explicit_lock || self.unload_active_lock > 0
    }

    /// Pin or unpin the grid manually.
    pub fn set_unload_explicit_lock(&mut self, on: bool) {
        self.unload_explicit_lock = on;
    }

    /// Register an active object whose spawn point is in this grid.
    pub fn inc_unload_active_lock(&mut self) {
        self.unload_active_lock = self.unload_active_lock.saturating_add(1);
    }

    /// Drop one active-object pin.
    pub fn dec_unload_active_lock(&mut self) {
        self.unload_active_lock = self.unload_active_lock.saturating_sub(1);
    }

    /// Restart the idle countdown, keeping a grid alive while it is being used.
    ///
    /// `Map::ResetGridExpiry` (Map.cpp:403-406).
    pub fn reset_idle_timer(&mut self) {
        if self.idle_since.is_some() {
            self.idle_since = Some(Instant::now());
        }
    }

    /// Take every object out of the grid, clearing all cells with it.
    ///
    /// Used by `Map::unload_grid`, which is responsible for deciding what
    /// happens to each returned GUID and for putting back anything that stays.
    pub fn take_objects(&mut self) -> SmallVec<[ObjectGuid; 16]> {
        for row in self.cells.iter_mut() {
            for cell in row.iter_mut() {
                cell.clear();
            }
        }
        self.player_count = 0;
        std::mem::take(&mut self.objects)
    }

    /// Check if grid should be unloaded (idle for longer than `delay`)
    pub fn should_unload(&self, delay: Duration) -> bool {
        if self.state != GridState::Idle {
            return false;
        }

        match self.idle_since {
            Some(idle_since) => idle_since.elapsed() > delay,
            None => false,
        }
    }

    /// Get objects in specific cells
    pub fn get_objects_in_cells(&self, cell_coords: &[(u8, u8)]) -> Vec<ObjectGuid> {
        let mut objects = Vec::new();
        for &(cx, cy) in cell_coords {
            if let Some(cell) = self.get_cell(cx, cy) {
                objects.extend(cell.objects().iter().copied());
            }
        }
        objects
    }

    /// Mark as loading
    pub fn mark_loading(&mut self) {
        if self.state == GridState::Invalid {
            self.state = GridState::Loading;
        }
    }

    /// Mark as loaded - transitions to Active if has players, Idle otherwise
    pub fn mark_loaded(&mut self) {
        if self.state == GridState::Loading {
            if self.has_players() {
                self.state = GridState::Active;
            } else {
                self.state = GridState::Idle;
            }
        }
    }

    /// Set loading priority (higher = load sooner)
    pub fn set_loading_priority(&mut self, priority: u8) {
        self.loading_priority = priority;
    }

    /// Get loading priority
    pub fn loading_priority(&self) -> u8 {
        self.loading_priority
    }
}

/// Manages all grids for a map with lazy loading support
pub struct GridManager {
    /// 64x64 grid array (lazy allocation)
    grids: Box<[[Option<Grid>; MAX_GRIDS]; MAX_GRIDS]>,
    /// Active grids (for iteration)
    active_grids: HashSet<(u8, u8)>,
}

impl GridManager {
    /// Create a new grid manager
    pub fn new() -> Self {
        use std::mem::MaybeUninit;

        // Initialize 2D array directly on the heap to avoid stack overflow
        let mut grids: Box<MaybeUninit<[[Option<Grid>; MAX_GRIDS]; MAX_GRIDS]>> = Box::new_uninit();

        unsafe {
            // Initialize the array element by element to avoid stack allocation
            let ptr = grids.as_mut_ptr();
            for i in 0..MAX_GRIDS {
                let row_ptr = (*ptr)[i].as_mut_ptr();
                for j in 0..MAX_GRIDS {
                    row_ptr.add(j).write(None);
                }
            }
            // SAFETY: All elements have been initialized to None
            let grids = grids.assume_init();

            Self {
                grids,
                active_grids: HashSet::new(),
            }
        }
    }

    /// Get or create a grid
    pub fn get_or_create_grid(&mut self, grid_x: u8, grid_y: u8) -> &mut Grid {
        let x = grid_x as usize;
        let y = grid_y as usize;

        if self.grids[x][y].is_none() {
            self.grids[x][y] = Some(Grid::new(grid_x, grid_y));
            self.active_grids.insert((grid_x, grid_y));
        }

        self.grids[x][y].as_mut().unwrap()
    }

    /// Get a grid if it exists
    pub fn get_grid(&self, grid_x: u8, grid_y: u8) -> Option<&Grid> {
        self.grids[grid_x as usize][grid_y as usize].as_ref()
    }

    /// Get a mutable grid if it exists
    pub fn get_grid_mut(&mut self, grid_x: u8, grid_y: u8) -> Option<&mut Grid> {
        self.grids[grid_x as usize][grid_y as usize].as_mut()
    }

    /// Ensure grid exists
    pub fn ensure_grid(&mut self, grid_x: u8, grid_y: u8) -> &mut Grid {
        self.get_or_create_grid(grid_x, grid_y)
    }

    /// Get or activate a grid - returns true if grid needs loading
    pub fn get_or_activate_grid(&mut self, grid_x: u8, grid_y: u8) -> bool {
        let grid = self.get_or_create_grid(grid_x, grid_y);

        match grid.state() {
            GridState::Invalid => {
                grid.mark_loading();
                true // Needs loading
            }
            GridState::Idle => {
                grid.set_state(GridState::Active);
                false // Already loaded
            }
            _ => false, // Already loading or active
        }
    }

    /// Add an object to the appropriate grid/cell
    pub fn add_object(&mut self, guid: ObjectGuid, x: f32, y: f32) {
        let (grid_x, grid_y) = world_to_grid(x, y);
        let (cell_x, cell_y) = world_to_cell_in_grid(x, y, grid_x, grid_y);

        let grid = self.get_or_create_grid(grid_x, grid_y);
        grid.add_object(guid, cell_x, cell_y);
        // Note: Grid::add_object already handles player counting

        // Prioritize if player enters loading grid
        if guid.is_player() && grid.state().is_loading() {
            grid.set_loading_priority(100);
        }
    }

    /// Remove an object from the grid
    pub fn remove_object(&mut self, guid: ObjectGuid, x: f32, y: f32) {
        let (grid_x, grid_y) = world_to_grid(x, y);
        let (cell_x, cell_y) = world_to_cell_in_grid(x, y, grid_x, grid_y);

        if let Some(grid) = self.get_grid_mut(grid_x, grid_y) {
            grid.remove_object(guid, cell_x, cell_y);
            // Note: Grid::remove_object already handles player counting
        }
    }

    /// Relocate an object
    pub fn relocate_object(
        &mut self,
        guid: ObjectGuid,
        old_x: f32,
        old_y: f32,
        new_x: f32,
        new_y: f32,
    ) {
        let (old_grid_x, old_grid_y) = world_to_grid(old_x, old_y);
        let (new_grid_x, new_grid_y) = world_to_grid(new_x, new_y);

        if old_grid_x != new_grid_x || old_grid_y != new_grid_y {
            // Grid changed
            self.remove_object(guid, old_x, old_y);
            self.add_object(guid, new_x, new_y);
        } else {
            // Same grid, might be different cell
            let (old_cell_x, old_cell_y) =
                world_to_cell_in_grid(old_x, old_y, old_grid_x, old_grid_y);
            let (new_cell_x, new_cell_y) =
                world_to_cell_in_grid(new_x, new_y, new_grid_x, new_grid_y);

            if old_cell_x != new_cell_x || old_cell_y != new_cell_y {
                if let Some(grid) = self.get_grid_mut(new_grid_x, new_grid_y) {
                    grid.remove_object(guid, old_cell_x, old_cell_y);
                    grid.add_object(guid, new_cell_x, new_cell_y);
                }
            }
        }
    }

    /// Get objects in range
    pub fn get_objects_in_range(&self, x: f32, y: f32, range: f32) -> HashSet<ObjectGuid> {
        let mut result = HashSet::new();

        let (min_grid_x, min_grid_y) = world_to_grid(x - range, y - range);
        let (max_grid_x, max_grid_y) = world_to_grid(x + range, y + range);

        for gx in min_grid_x..=max_grid_x {
            for gy in min_grid_y..=max_grid_y {
                // No state filter: an object is in this list only because it was
                // explicitly added, so anything here is live regardless of whether
                // the grid's spawn data is loaded. Filtering on state hid objects
                // during the load window (spawns are inserted while the grid is
                // still `Loading`) and hid players left standing in a grid that
                // was unloaded around them.
                if let Some(grid) = self.get_grid(gx, gy) {
                    result.extend(grid.objects().iter().copied());
                }
            }
        }

        result
    }

    /// Get grids that need loading (are in Loading state), sorted by priority
    pub fn get_grids_needing_load(&self) -> Vec<(u8, u8)> {
        let mut result: Vec<(u8, u8, u8)> = Vec::new();
        for &(gx, gy) in &self.active_grids {
            if let Some(grid) = self.get_grid(gx, gy) {
                if grid.state().is_loading() {
                    result.push((gx, gy, grid.loading_priority()));
                }
            }
        }

        // Sort by priority (descending - highest priority first)
        result.sort_by(|a, b| b.2.cmp(&a.2));

        // Return just coordinates (priority was for sorting)
        result.into_iter().map(|(gx, gy, _)| (gx, gy)).collect()
    }

    /// Get grids that have been idle for longer than `delay`
    pub fn get_grids_to_unload(&self, delay: Duration) -> Vec<(u8, u8)> {
        let mut result = Vec::new();
        for &(gx, gy) in &self.active_grids {
            if let Some(grid) = self.get_grid(gx, gy) {
                if grid.should_unload(delay) {
                    result.push((gx, gy));
                }
            }
        }
        result
    }

    /// Mark grid as loaded (called after spawning creatures)
    pub fn mark_loaded(&mut self, grid_x: u8, grid_y: u8) {
        if let Some(grid) = self.get_grid_mut(grid_x, grid_y) {
            grid.mark_loaded();
        }
    }

    /// Drain a grid's objects and reset it to `Invalid`.
    ///
    /// Callers must go through `Map::unload_grid`, which also clears the map's
    /// object registries and decides what happens to each returned GUID.
    pub(crate) fn drain_grid(&mut self, grid_x: u8, grid_y: u8) -> SmallVec<[ObjectGuid; 16]> {
        let Some(grid) = self.get_grid_mut(grid_x, grid_y) else {
            return SmallVec::new();
        };

        grid.set_state(GridState::Removal);
        let objects = grid.take_objects();
        grid.set_state(GridState::Invalid);
        objects
    }

    /// Number of active grids
    pub fn active_grid_count(&self) -> usize {
        self.active_grids.len()
    }

    /// Get grid state
    pub fn get_grid_state(&self, grid_x: u8, grid_y: u8) -> Option<GridState> {
        self.get_grid(grid_x, grid_y).map(|g| g.state())
    }
}

impl Default for GridManager {
    fn default() -> Self {
        Self::new()
    }
}
