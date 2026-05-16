//! NPC Gossip module
//!
//! Handles gossip menu interactions between players and NPCs.

pub mod manager;
pub mod system;
#[cfg(test)]
mod tests;
pub mod types;

pub use manager::GossipManager;
pub use system::GossipSystem;
pub use types::*;
