//! VMap (Virtual Maps) system for world
//! Handles 3D geometry, collision detection, line of sight, and height calculations.
//! Ported from server/src/world/map/vmap/

pub mod bsp_tree;
pub mod file_loader;
pub mod manager;
pub mod types;

pub use manager::VMapManager;
pub use types::*;
