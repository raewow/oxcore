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

use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::ObjectGuid;
use crate::protocol::{Opcode, WorldPacket};

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
    /// 1.14 replaces vanilla's `(index, item, display, count, price, durability, buy count)` tuple
    /// with a wider entry: the price widens to u64, a type and extended-cost id are added, and an
    /// embedded item instance carries the item id -- so the display id, which vanilla sends
    /// explicitly, is resolved client-side instead.
    ///
    /// The count also moves ahead of a reason byte that vanilla has no equivalent for; zero means
    /// "the list follows", which is the only case a 1.12 server produces.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.vendor_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u8(0); // Reason: inventory follows
        writer.write_i32(self.items.len() as i32);

        for item in &self.items {
            writer.write_i32(item.index as i32); // Slot
            writer.write_i32(1); // Type: item, as opposed to a currency
                                 // Vanilla's "max count" is the remaining stock, and -1 there means unlimited; 1.14 reads
                                 // the same convention, so the value carries over.
            writer.write_i32(item.max_count as i32); // Quantity
            writer.write_u64(u64::from(item.price));
            writer.write_i32(item.max_durability as i32);
            writer.write_u32(item.buy_count); // StackCount
            writer.write_i32(0); // ExtendedCostID -- no honor or arena costs in Classic Era
            writer.write_i32(0); // PlayerConditionFailed

            // ItemInstance: a bare item id, which is all a 1.12 item has.
            writer.write_u32(item.item_id);
            writer.write_u32(0); // RandomPropertiesSeed
            writer.write_u32(0); // RandomPropertiesID
            writer.write_bit(false); // HasItemBonus
            writer.flush_bits();
            writer.write_bits(0, 6); // ItemModList count
            writer.flush_bits();

            writer.write_bit(false); // DoNotFilterOnVendor
            writer.write_bit(false); // Refundable
            writer.flush_bits();
        }

        Some(writer.finish(Opcode::SMSG_LIST_INVENTORY))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LIST_INVENTORY);

        // Write vendor GUID (unpacked)
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
    /// 1.14 sends the vendor's remaining stock as a signed `NewQuantity` and the amount bought
    /// separately, where vanilla sends the slot's new count and the purchased count.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.vendor_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.vendor_slot);
        writer.write_i32(self.remaining_stock as i32); // NewQuantity
        writer.write_u32(self.count); // QuantityBought
        Some(writer.finish(Opcode::SMSG_BUY_ITEM))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// 1.14 names the vendor **slot** where vanilla names the item id. We carry only the item id, so
    /// it goes in that field: the client uses it to pick the error text, and both are u32 values it
    /// looks up rather than indexes into anything.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.vendor_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.item_id); // Slot -- see above
        writer.write_u8(self.error as u8);
        Some(writer.finish(Opcode::SMSG_BUY_FAILED))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// The same three fields, with both GUIDs packed as guid128.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.vendor_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        let (high, low) = self.item_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u8(self.result as u8);
        Some(writer.finish(Opcode::SMSG_SELL_ITEM))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SELL_ITEM);
        packet.write_guid(self.vendor_guid);
        packet.write_guid(self.item_guid);
        packet.write_u8(self.result as u8);
        packet
    }
}
