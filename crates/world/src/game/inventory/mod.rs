//! Inventory module for world
//!
//! Lightweight inventory system supporting character enumeration and player login.
//! This is a simplified version of the full InventorySystem, focused on:
//! - Loading equipment for character enumeration
//! - Loading full inventory on player login
//! - Sending inventory items to client

pub mod cache;
pub mod can_store;
pub mod system;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::{CachedItemInfo, InventoryCache, PendingInventoryOp, PlayerInventoryData};
pub use can_store::{CanStoreChecker, CanStoreResult};
pub use oxcore_shared::game::inventory::{
    decode_position, encode_position, is_bag_pos, is_bank_pos, is_equipment_pos, is_inventory_pos,
    EnchantmentOffset, EnchantmentSlot, EquipmentSlot, InventoryResult, ItemLootUpdateState,
    ItemPosCount, ItemPosCountVec, ItemUpdateState, BANK_SLOT_BAG_END, BANK_SLOT_BAG_START,
    BANK_SLOT_ITEM_END, BANK_SLOT_ITEM_START, BUYBACK_SLOT_END, BUYBACK_SLOT_START,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, KEYRING_SLOT_END, KEYRING_SLOT_START,
    MAX_BAG_SIZE, MAX_ENCHANTMENT_OFFSET, MAX_ENCHANTMENT_SLOT, NULL_BAG, NULL_SLOT,
};
pub use system::InventorySystem;
pub use types::*;
