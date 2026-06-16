//! Vendor system message structs
//!
//! This module contains type-safe message structures for all vendor-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgListInventory`] - Vendor item list
//! - [`SmsgBuyItem`] - Buy item success
//! - [`SmsgBuyFailed`] - Buy item failure
//! - [`SmsgSellItem`] - Sell item result

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::packet::WorldPacketGuidExt;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::{Opcode, WorldPacket};

/// Vendor item data for SMSG_LIST_INVENTORY
#[derive(Debug, Clone)]
pub struct VendorItemData {
    /// Item slot index (1-based)
    pub index: u32,
    /// Item entry ID
    pub item_id: u32,
    /// Item display ID
    pub display_id: u32,
    /// Max count in stock (0xFFFFFFFF = unlimited, 0 = sold out)
    pub max_count: u32,
    /// Price in copper (after discounts)
    pub price: u32,
    /// Max durability
    pub max_durability: u32,
    /// Buy count (stack size)
    pub buy_count: u32,
}

/// SMSG_LIST_INVENTORY (0x19F) - Vendor item list
///
/// Sent when player opens a vendor window.
#[derive(Debug, Clone)]
pub struct SmsgListInventory {
    /// Vendor GUID
    pub vendor_guid: ObjectGuid,
    /// Items for sale
    pub items: Vec<VendorItemData>,
}

impl ToWorldPacket for SmsgListInventory {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LIST_INVENTORY);

        // Write vendor GUID (unpacked - matches MaNGOS behavior)
        packet.write_guid(self.vendor_guid);

        // Write item count (u8)
        packet.write_u8(self.items.len() as u8);

        // Write items
        for item in &self.items {
            packet.write_u32(item.index);
            packet.write_u32(item.item_id);
            packet.write_u32(item.display_id);
            packet.write_u32(item.max_count);
            packet.write_u32(item.price);
            packet.write_u32(item.max_durability);
            packet.write_u32(item.buy_count);
        }

        packet
    }
}

/// SMSG_BUY_ITEM (0x1A4) - Buy item success
#[derive(Debug, Clone)]
pub struct SmsgBuyItem {
    /// Vendor GUID
    pub vendor_guid: ObjectGuid,
    /// Vendor slot index (1-based)
    pub vendor_slot: u32,
    /// Remaining stock (0xFFFFFFFF = unlimited)
    pub remaining_stock: u32,
    /// Count purchased
    pub count: u32,
}

impl ToWorldPacket for SmsgBuyItem {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BUY_ITEM);
        packet.write_guid(self.vendor_guid);
        packet.write_u32(self.vendor_slot);
        packet.write_u32(self.remaining_stock);
        packet.write_u32(self.count);
        packet
    }
}

/// Buy error codes
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum BuyError {
    /// Item not found
    CantFind = 0,
    /// Item already sold
    ItemAlreadySold = 1,
    /// Not enough money
    NotEnoughMoney = 2,
    /// Seller doesn't like you (reputation)
    SellerDontLikeYou = 4,
    /// Too far from vendor
    DistanceTooFar = 5,
    /// Item is sold out
    ItemSoldOut = 7,
    /// Can't carry more (bag full)
    CantCarryMore = 8,
    /// Rank requirement not met
    RankRequire = 11,
    /// Reputation requirement not met
    ReputationRequire = 12,
}

/// SMSG_BUY_FAILED (0x1A5) - Buy item failure
#[derive(Debug, Clone)]
pub struct SmsgBuyFailed {
    /// Vendor GUID
    pub vendor_guid: ObjectGuid,
    /// Item entry ID
    pub item_id: u32,
    /// Error code
    pub error: BuyError,
}

impl ToWorldPacket for SmsgBuyFailed {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BUY_FAILED);
        packet.write_guid(self.vendor_guid);
        packet.write_u32(self.item_id);
        packet.write_u8(self.error as u8);
        packet
    }
}

/// Buy result codes for internal use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BuyResult {
    /// Purchase successful
    Success = 0,
    /// Item not found
    ItemNotFound = 1,
    /// Not enough money
    NotEnoughMoney = 2,
    /// Item sold out
    SoldOut = 3,
    /// Can't carry more
    InventoryFull = 4,
    /// Reputation too low
    ReputationTooLow = 5,
}

/// Sell result codes
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SellResult {
    /// Sell successful
    Ok = 0,
    /// Can't find item
    CantFindItem = 1,
    /// Can't sell this item
    CantSellItem = 2,
    /// Can't find vendor
    CantFindVendor = 3,
}

/// SMSG_SELL_ITEM (0x1A6) - Sell item result
#[derive(Debug, Clone)]
pub struct SmsgSellItem {
    /// Vendor GUID
    pub vendor_guid: ObjectGuid,
    /// Item GUID
    pub item_guid: ObjectGuid,
    /// Result code
    pub result: SellResult,
}

impl ToWorldPacket for SmsgSellItem {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SELL_ITEM);
        packet.write_guid(self.vendor_guid);
        packet.write_guid(self.item_guid);
        packet.write_u8(self.result as u8);
        packet
    }
}
