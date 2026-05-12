//! Movement module - creature movement system

mod types;
mod generator;
mod motion_master;
mod spline;
mod system;
pub mod generators;
pub mod waypoint_repository;
pub mod waypoint_manager;

pub use types::{MovementGeneratorType, MovementSpeeds, MoveType};
pub use generator::{MovementGenerator, MovementUpdate};
pub use motion_master::MotionMaster;
pub use spline::MoveSpline;
pub use system::MovementSystem;
pub use waypoint_manager::WaypointManager;
