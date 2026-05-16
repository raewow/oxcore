//! Visibility state - per-player visibility tracking
//!
//! Each player has their own visibility state that tracks:
//! - Which objects they can currently see
//! - When they last had a visibility update (for throttling)
//! - Pending notifications to be batched and sent

use std::collections::HashSet;

use crate::shared::protocol::ObjectGuid;
use crate::world::map::grid_coords::CellPair;

/// Per-player visibility state
#[derive(Debug)]
pub struct VisibilityState {
    /// Objects currently visible to this player
    pub visible_objects: HashSet<ObjectGuid>,

    /// Last cell position (for cell-crossing detection)
    pub last_cell: CellPair,

    /// Last visibility update tick (for throttling)
    pub last_update_tick: u32,

    /// Pending appeared objects (for batching)
    pub pending_appeared: Vec<ObjectGuid>,

    /// Pending disappeared objects (for batching)
    pub pending_disappeared: Vec<ObjectGuid>,

    /// Dirty flag - needs visibility recalculation
    pub dirty: bool,

    /// Force immediate update (bypass throttle) - used for login/teleport
    pub force_immediate: bool,

    /// Objects that have had CREATE_OBJECT2 sent (safety guard against duplicates)
    /// Cleared when object goes out of range or player logs out
    pub objects_created: HashSet<ObjectGuid>,

    /// True while a visibility update is in progress
    /// Prevents concurrent updates from login + map loop
    pub update_in_progress: bool,
}

impl VisibilityState {
    /// Create new visibility state with initial cell position
    pub fn new(initial_cell: CellPair) -> Self {
        Self {
            visible_objects: HashSet::with_capacity(64), // Preallocate for nearby players
            last_cell: initial_cell,
            last_update_tick: 0,
            pending_appeared: Vec::new(),
            pending_disappeared: Vec::new(),
            dirty: true,           // Start dirty to force initial update
            force_immediate: true, // Force immediate on creation (login)
            objects_created: HashSet::with_capacity(64),
            update_in_progress: false,
        }
    }

    /// Check if player has crossed cell boundary
    pub fn has_crossed_cell(&self, new_cell: CellPair) -> bool {
        self.last_cell != new_cell
    }

    /// Update last cell and mark dirty if changed
    pub fn update_cell(&mut self, new_cell: CellPair) {
        if self.last_cell != new_cell {
            self.last_cell = new_cell;
            self.dirty = true;
        }
    }

    /// Mark for forced immediate update (login/teleport)
    pub fn mark_force_immediate(&mut self) {
        self.dirty = true;
        self.force_immediate = true;
    }

    /// Clear pending notifications after they've been sent
    pub fn clear_pending(&mut self) {
        self.pending_appeared.clear();
        self.pending_disappeared.clear();
    }

    /// Check if there are pending notifications to send
    pub fn has_pending_notifications(&self) -> bool {
        !self.pending_appeared.is_empty() || !self.pending_disappeared.is_empty()
    }
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self::new(CellPair::new(0, 0))
    }
}
