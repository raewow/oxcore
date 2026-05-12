// Instance system - handles dungeon/raid instance management

pub mod manager;
pub mod types;

// Re-export all types, constants, and structs
pub use types::*;

// Re-export manager
pub use manager::InstanceMgr;
