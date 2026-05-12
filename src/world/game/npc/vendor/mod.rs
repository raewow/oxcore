//! NPC Vendor module
//!
//! Handles vendor interactions between players and NPCs.
//! Supports vendor templates, stock management, reputation discounts, and extended costs.

pub mod manager;
pub mod system;
pub mod types;

pub use manager::VendorManager;
pub use system::VendorSystem;
pub use types::*;
