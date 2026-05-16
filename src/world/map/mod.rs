//! Map system - spatial organization with grid hierarchy
//!
//! Structure:
//! - Map (64×64 grids)
//!   - Grid (16×16 cells, 533.33 units each)
//!     - Cell (33.33 units each)

pub mod grid;
pub mod grid_coords;
pub mod manager;
pub mod map;
pub mod pathfinding;

pub use grid_coords::{CellPair, GridPair};
pub use manager::MapManager;
pub use map::Map;
pub use pathfinding::{GamePathFinder, MMapManager, PathFinder, PathResult, VMapManager};
