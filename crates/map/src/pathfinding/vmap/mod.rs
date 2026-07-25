//! VMap (Virtual Maps) system for world
//! Handles 3D geometry, collision detection, line of sight, and height calculations.
//! Ported from server/src/world/map/vmap/

pub mod bsp_tree;
pub mod dynamic_tree;
pub mod file_loader;
pub mod go_model_list;
pub mod manager;
pub mod model_transform;
pub mod types;

pub use dynamic_tree::DynamicMapTree;
pub use go_model_list::{GameObjectModelData, GAMEOBJECT_MODELS_FILE};
pub use manager::{GameObjectModelSpawn, VMapManager};
pub use types::*;
