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
pub mod terrain;

pub use self::pathfinding::{GamePathFinder, MMapManager, PathFinder, PathResult, VMapManager};
pub use grid_coords::{CellPair, GridPair};
pub use manager::{MapConfigProvider, MapManager};
pub use map::{
    GridStatePass, Map, MapConfig, MapKind, ObjectKind, RelocateResult, UnloadedObjects,
};
pub use terrain::{LiquidData, LiquidStatusFlags, TerrainInfo, TerrainManager};
