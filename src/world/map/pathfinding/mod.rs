//! Pathfinding module - navigation mesh, collision, and path calculation
//!
//! Architecture:
//! - GamePathFinder: High-level API for movement generators
//! - PathFinder: Low-level MMap PathFinder with obstacle avoidance
//! - MMapManager: NavMesh loading and management
//! - NavMesh: Rust-native A* pathfinding through polygon graph
//! - VMapManager: Collision detection and line-of-sight

pub mod game_pathfinder;
pub mod mmap_manager;
pub mod navmesh;
pub mod pathfinder;
pub mod types;
pub mod vmap;

pub use game_pathfinder::GamePathFinder;
pub use mmap_manager::MMapManager;
pub use navmesh::NavMesh;
pub use pathfinder::PathFinder;
pub use types::{path_flags, HeightResult, PathResult};
pub use vmap::VMapManager;
