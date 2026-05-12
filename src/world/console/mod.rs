//! Console Module for world
//!
//! Exports console commands for the world server.

pub mod commands;

// Re-export command registration function
pub use commands::register_all_commands;
