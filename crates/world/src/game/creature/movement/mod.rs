//! Movement module - creature movement system

mod generator;
mod follower_reference;
pub mod generators;
mod motion_master;
pub mod packet_sender;
mod spline;
mod system;
mod types;
pub mod waypoint_manager;
pub mod waypoint_repository;

pub use generator::{MovementGenerator, MovementUpdate};
pub use follower_reference::FollowerReference;
pub use motion_master::MotionMaster;
pub use packet_sender::{MovementChangeType, MovementPacketSender};
pub use spline::MoveSpline;
pub use system::MovementSystem;
pub use types::{MoveType, MovementGeneratorType, MovementSpeeds};
pub use waypoint_manager::WaypointManager;
