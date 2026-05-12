//! Inventory module for world
//!
//! Lightweight inventory system supporting character enumeration and player login.
//! This is a simplified version of the full InventorySystem, focused on:
//! - Loading equipment for character enumeration
//! - Loading full inventory on player login
//! - Sending inventory items to client

pub mod cache;
pub mod inventory_types;
pub mod system;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::{CachedItemInfo, InventoryCache, PendingInventoryOp, PlayerInventoryData};
pub use system::InventorySystem;
pub use types::*;
