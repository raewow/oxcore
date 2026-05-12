//! Pathfinding module - navigation mesh, collision, and path calculation
//!
//! Architecture:
//! - GamePathFinder: High-level API for movement generators
//! - PathFinder: Low-level MMap PathFinder with obstacle avoidance
//! - MMapManager: NavMesh loading and management
//! - NavMesh: Rust-native A* pathfinding through polygon graph
//! - VMapManager: Collision detection and line-of-sight

pub mod types;
pub mod navmesh;
pub mod vmap;
pub mod mmap_manager;
pub mod pathfinder;
pub mod game_pathfinder;

pub use types::{PathResult, HeightResult, path_flags};
pub use navmesh::NavMesh;
pub use vmap::VMapManager;
pub use mmap_manager::MMapManager;
pub use pathfinder::PathFinder;
pub use game_pathfinder::GamePathFinder;
