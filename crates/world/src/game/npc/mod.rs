//! NPC module - NPC interaction systems
//!
//! Contains systems for NPC interactions:
//! - gossip: Dialog menus and player-NPC interaction
//! - vendor: Buy/sell items from NPCs
//! - quest: Quest giving and completion

pub mod gossip;
pub mod quest;
pub mod trainer;
pub mod vendor;

pub use gossip::{GossipManager, GossipSystem};
pub use quest::{QuestManager, QuestSystem};
pub use trainer::TrainerManager;
pub use vendor::{VendorManager, VendorSystem};
