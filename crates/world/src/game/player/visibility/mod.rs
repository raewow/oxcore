//! Visibility subsystem - player visibility tracking and notifications
//!
//! This is a subsystem of the player system (like movement).
//! Each player has their own VisibilityState embedded in the Player struct.

pub mod state;
pub mod system;

#[cfg(test)]
mod tests;

pub use state::VisibilityState;
pub use system::VisibilitySubsystem;
