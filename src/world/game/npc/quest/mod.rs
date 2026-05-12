//! Quest system module
//!
//! Provides quest functionality including quest templates, quest giver status,
//! quest accept/complete, and integration with gossip menus.
//!
//! # Architecture
//!
//! - [`QuestManager`](manager::QuestManager): State storage (DashMaps)
//! - [`QuestSystem`](system::QuestSystem): Business logic and packet handling
//! - Types in [`types`](types): QuestTemplate, QuestStatus, QuestProgress, etc.

pub mod manager;
pub mod system;
pub mod types;

// Re-exports for convenience
pub use manager::QuestManager;
pub use system::QuestSystem;
pub use types::*;