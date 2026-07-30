use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

/// `ItemInstance::Write` for build 42597.
///
/// A bare item id with no bonuses or modifications, which is all a 1.12 item has. Shared by the loot
/// and item-push bodies, both of which embed one.
fn write_modern_item_instance(writer: &mut BitWriter, item_id: u32, random_property: u32) {
    writer.write_u32(item_id);
    writer.write_u32(0); // RandomPropertiesSeed
    writer.write_u32(random_property); // RandomPropertiesID
    writer.write_bit(false); // HasItemBonus
    writer.flush_bits();
    writer.write_bits(0, 6); // ItemModList count
    writer.flush_bits();
}

/// SMSG_LOOT_RESPONSE (0x0160)
#[derive(Debug, Clone)]
pub struct SmsgLootResponse {
    pub loot_guid: ObjectGuid,
    pub loot_type: u8,
    pub gold: u32,
    pub items: Vec<LootResponseItem>,
}

#[derive(Debug, Clone)]
pub struct LootResponseItem {
    pub slot: u8,
    pub item_id: u32,
    pub count: u32,
    pub display_id: u32,
    pub random_suffix: u32,
    pub random_property: u32,
    pub slot_type: u8, // 0 = normal, 1 = quest, 2 = group needed
}

impl ToWorldPacket for SmsgLootResponse {
    /// `LootResponse::Write` with `LootItemData` (the 1.14 reference
    ///, `:143-155`).
    ///
    /// 1.14 names **two** GUIDs — the looted object's owner and a separate loot object — where vanilla
    /// sends one. We have only the one, so it fills both: Classic Era has no distinct loot object, and
    /// an empty second GUID makes the client discard the window.
    ///
    /// Each item's type and UI type become 2- and 3-bit fields ahead of an embedded `ItemInstance`,
    /// and the slot moves *after* the quantity as `LootListID`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.loot_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Owner
        writer.write_packed_guid_128(high, low); // LootObj -- see above

        writer.write_u8(0); // FailureReason -- success
        writer.write_u8(self.loot_type); // AcquireReason
        writer.write_u8(0); // LootMethod: free-for-all
        writer.write_u8(2); // Threshold: uncommon, vanilla's default group threshold
        writer.write_u32(self.gold); // Coins
        writer.write_i32(self.items.len() as i32);
        writer.write_i32(0); // Currencies count -- none in Classic Era
        writer.write_bit(true); // Acquired
        writer.write_bit(false); // AELooting
        writer.flush_bits();

        for item in &self.items {
            writer.write_bits(0, 2); // Type: item
            writer.write_bits(u32::from(item.slot_type), 3); // UIType
            writer.write_bit(false); // CanTradeToTapList
            writer.flush_bits();
            write_modern_item_instance(&mut writer, item.item_id, item.random_property);
            writer.write_u32(item.count); // Quantity
            writer.write_u8(0); // LootItemType
            writer.write_u8(item.slot); // LootListID
        }

        Some(writer.finish(Opcode::SMSG_LOOT_RESPONSE))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_RESPONSE);
        packet.write_u64(self.loot_guid.raw());
        packet.write_u8(self.loot_type);
        packet.write_u32(self.gold);
        packet.write_u8(self.items.len() as u8);

        for item in &self.items {
            packet.write_u8(item.slot);
            packet.write_u32(item.item_id);
            packet.write_u32(item.count);
            packet.write_u32(item.display_id);
            packet.write_u32(item.random_suffix);
            packet.write_u32(item.random_property);
            packet.write_u8(item.slot_type);
        }

        packet
    }
}

/// SMSG_LOOT_RELEASE_RESPONSE (0x0161)
#[derive(Debug, Clone)]
pub struct SmsgLootReleaseResponse {
    pub loot_guid: ObjectGuid,
    pub unknown: u8, // always 1
}

impl ToWorldPacket for SmsgLootReleaseResponse {
    /// `LootReleaseResponse::Write`. Vanilla's trailing byte is gone;
    /// 1.14 sends the loot object and its owner instead.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.loot_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // LootObj
        writer.write_packed_guid_128(high, low); // Owner
        Some(writer.finish(Opcode::SMSG_LOOT_RELEASE_RESPONSE))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_RELEASE_RESPONSE);
        packet.write_u64(self.loot_guid.raw());
        packet.write_u8(self.unknown);
        packet
    }
}

/// SMSG_LOOT_REMOVED (0x0162)
#[derive(Debug, Clone)]
pub struct SmsgLootRemoved {
    pub slot: u8,
}

impl ToWorldPacket for SmsgLootRemoved {
    /// 1.14 names the owner and loot object ahead of the slot, where vanilla sends the slot alone.
    /// The struct carries no GUID, so both are sent empty -- the client matches the removal by slot,
    /// and the window it applies to is the one currently open.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(0, 0); // Owner
        writer.write_packed_guid_128(0, 0); // LootObj
        writer.write_u8(self.slot); // LootListID
        Some(writer.finish(Opcode::SMSG_LOOT_REMOVED))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_REMOVED);
        packet.write_u8(self.slot);
        packet
    }
}

/// SMSG_LOOT_MONEY_NOTIFY (0x0163)
#[derive(Debug, Clone)]
pub struct SmsgLootMoneyNotify {
    pub gold: u32,
}

impl ToWorldPacket for SmsgLootMoneyNotify {
    /// `LootMoneyNotify::Write`. Money widens to u64 and gains a
    /// modifier plus a sole-looter bit.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u64(u64::from(self.gold)); // Money
        writer.write_u64(0); // MoneyMod
        writer.write_bit(true); // SoleLooter -- free-for-all looting has no split
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_LOOT_MONEY_NOTIFY))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_MONEY_NOTIFY);
        packet.write_u32(self.gold);
        packet
    }
}

/// SMSG_LOOT_CLEAR_MONEY (0x0165)
#[derive(Debug, Clone)]
pub struct SmsgLootClearMoney;

impl ToWorldPacket for SmsgLootClearMoney {
    /// Empty in both protocols.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOOT_CLEAR_MONEY)
    }
}
