//! Grid and cell structures for spatial organization with lazy loading
//!
//! This module implements the grid loading system for Phase 11:
//! - Grid state machine (Invalid → Loading → Active → Idle → Removal)
//! - Lazy creature loading when players approach
//! - Creature unloading when grids go idle
//! - Player-based grid activation

mod cell;
mod grid;
mod grid_state;

pub use cell::Cell;
pub use grid::{Grid, GridManager, CELLS_PER_GRID, DEFAULT_GRID_UNLOAD_DELAY, MAX_GRIDS};
pub use grid_state::GridState;
