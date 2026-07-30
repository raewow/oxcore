//! Terrain (ADT) data: area ids, the height mesh, and the liquid layer.
//!
//! Reads the `maps/*.map` files produced by the extractor and combines them
//! with VMap model geometry for height and liquid queries.

pub mod defines;
pub mod grid_map;
pub mod manager;

pub use defines::*;
pub use grid_map::GridMap;
pub use manager::{TerrainInfo, TerrainManager};
