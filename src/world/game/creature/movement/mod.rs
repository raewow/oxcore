//! Movement module - creature movement system

mod generator;
pub mod generators;
mod motion_master;
mod spline;
mod system;
mod types;
pub mod waypoint_manager;
pub mod waypoint_repository;

pub use generator::{MovementGenerator, MovementUpdate};
pub use motion_master::MotionMaster;
pub use spline::MoveSpline;
pub use system::MovementSystem;
pub use types::{MoveType, MovementGeneratorType, MovementSpeeds};
pub use waypoint_manager::WaypointManager;
