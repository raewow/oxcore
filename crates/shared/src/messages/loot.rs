use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::loot_guid128;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

/// `ItemInstance::Write` for build 42597.
///
/// A bare item id with no bonuses or modifications, which is all a 1.12 item has. Shared by the loot
/// and item-push bodies, both of which embed one.
pub fn write_modern_item_instance(writer: &mut BitWriter, item_id: u32, random_property: u32) {
    writer.write_u32(item_id);
    writer.write_u32(0); // RandomPropertiesSeed
    writer.write_u32(random_property); // RandomPropertiesID
    writer.write_bit(false); // HasItemBonus
    writer.flush_bits();
    writer.write_bits(0, 6); // ItemModList count
    writer.flush_bits();
}

/// Loot slot types on the 1.12 wire: 0 lootable, 1 roll ongoing, 2 master loot, 3 locked,
/// 4 owner. 1.14 keeps the values but swaps master and locked.
pub fn modern_loot_ui_type(vanilla_slot_type: u8) -> u32 {
    match vanilla_slot_type {
        2 => 3, // master
        3 => 2, // locked
        other => u32::from(other),
    }
}

/// `LootItemData` for the 1.14 wire: the per-item block shared by the loot window and the roll
/// packets. `loot_list_id` is already 1-based.
pub fn write_modern_loot_item_data(
    writer: &mut BitWriter,
    item_id: u32,
    random_property: u32,
    quantity: u32,
    vanilla_slot_type: u8,
    loot_list_id: u8,
) {
    writer.write_bits(0, 2); // Type: item
    writer.write_bits(modern_loot_ui_type(vanilla_slot_type), 3); // UIType
    writer.write_bit(false); // CanTradeToTapList
    writer.flush_bits();
    write_modern_item_instance(writer, item_id, random_property);
    writer.write_u32(quantity); // Quantity
    writer.write_u8(0); // LootItemType
    writer.write_u8(loot_list_id); // LootListID
}

/// SMSG_LOOT_RESPONSE (0x0160)
#[derive(Debug, Clone)]
pub struct SmsgLootResponse {
    pub loot_guid: ObjectGuid,
    pub loot_type: u8,
    pub gold: u32,
    pub loot_method: u8,
    pub threshold: u8,
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
    pub slot_type: u8, // 1.12 slot type: 0 lootable, 1 roll ongoing, 2 master, 3 locked, 4 owner
}

impl ToWorldPacket for SmsgLootResponse {
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
        let (loot_high, loot_low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(loot_high, loot_low); // LootObj

        writer.write_u8(0); // FailureReason -- success
        writer.write_u8(self.loot_type); // AcquireReason
        writer.write_u8(self.loot_method); // LootMethod
        writer.write_u8(self.threshold); // Threshold
        writer.write_u32(self.gold); // Coins
        writer.write_i32(self.items.len() as i32);
        writer.write_i32(0); // Currencies count -- none in Classic Era
        writer.write_bit(true); // Acquired
        writer.write_bit(false); // AELooting
        writer.flush_bits();

        for item in &self.items {
            // LootListID is 1-based on the modern wire (the client's own loot UI reserves 0),
            // while our internal `slot` is the 0-based wire slot. Vanilla's wire slot is
            // genuinely 0-based, so only the modern encoding offsets it.
            write_modern_loot_item_data(
                &mut writer,
                item.item_id,
                item.random_property,
                item.count,
                item.slot_type,
                item.slot + 1,
            );
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

/// SMSG_LOOT_LIST - who may loot a corpse (1.14 only)
///
/// Lets the 1.14 client show the corpse as lootable for the right players: the master looter
/// under master loot, or this corpse's round-robin owner. 1.12 has no such message.
#[derive(Debug, Clone, Copy)]
pub struct SmsgLootList {
    pub loot_guid: ObjectGuid,
    pub master: Option<ObjectGuid>,
    pub round_robin_winner: Option<ObjectGuid>,
}

impl ToWorldPacket for SmsgLootList {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOOT_LIST)
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.loot_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Owner
        let (loot_high, loot_low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(loot_high, loot_low); // LootObj
        writer.write_bit(self.master.is_some());
        writer.write_bit(self.round_robin_winner.is_some());
        writer.flush_bits();
        for guid in [self.master, self.round_robin_winner].into_iter().flatten() {
            let (h, l) = guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(h, l);
        }
        Some(writer.finish(Opcode::SMSG_LOOT_LIST))
    }
}

/// Why a loot window did not open.
pub mod loot_error {
    pub const DIDNT_KILL: u8 = 0;
    pub const TOO_FAR: u8 = 4;
    pub const NOT_STANDING: u8 = 8;
    pub const STUNNED: u8 = 9;
    pub const PLAYER_NOT_FOUND: u8 = 10;
    pub const ALREADY_PICKPOCKETED: u8 = 15;
}

/// SMSG_LOOT_RESPONSE carrying a failure instead of a loot list.
///
/// 1.12 signals it with loot type 0 followed by the error code; 1.14 has a FailureReason field
/// and an empty window.
#[derive(Debug, Clone, Copy)]
pub struct SmsgLootError {
    pub loot_guid: ObjectGuid,
    pub error: u8,
}

impl ToWorldPacket for SmsgLootError {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_RESPONSE);
        packet.write_u64(self.loot_guid.raw());
        packet.write_u8(0); // LOOT_NONE
        packet.write_u8(self.error);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.loot_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Owner
        let (loot_high, loot_low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(loot_high, loot_low); // LootObj
        writer.write_u8(self.error); // FailureReason
        writer.write_u8(0); // AcquireReason
        writer.write_u8(0); // LootMethod
        writer.write_u8(0); // Threshold
        writer.write_u32(0); // Coins
        writer.write_i32(0); // items
        writer.write_i32(0); // currencies
        writer.write_bit(false); // Acquired
        writer.write_bit(false); // AELooting
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_LOOT_RESPONSE))
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
    pub loot_guid: ObjectGuid,
    pub slot: u8,
}

impl ToWorldPacket for SmsgLootRemoved {
    /// 1.14 names the owner and loot object ahead of the slot, where vanilla sends the slot alone.
    /// Both GUIDs match the ones the loot window was opened with.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.loot_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Owner
        let (loot_high, loot_low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(loot_high, loot_low); // LootObj
        writer.write_u8(self.slot + 1); // LootListID -- see SmsgLootResponse::to_modern
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
    /// The looter kept the whole drop (no group split). 1.14 only.
    pub sole_looter: bool,
}

impl ToWorldPacket for SmsgLootMoneyNotify {
    /// `LootMoneyNotify::Write`. Money widens to u64 and gains a
    /// modifier plus a sole-looter bit.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u64(u64::from(self.gold)); // Money
        writer.write_u64(0); // MoneyMod
        writer.write_bit(self.sole_looter); // SoleLooter
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
pub struct SmsgLootClearMoney {
    pub loot_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgLootClearMoney {
    /// `SMSG_COIN_REMOVED` on 1.14: names the loot object whose coins went, matching the GUID
    /// the loot window was opened with.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = loot_guid128(&self.loot_guid);
        writer.write_packed_guid_128(high, low); // LootObj
        Some(writer.finish(Opcode::SMSG_LOOT_CLEAR_MONEY))
    }

    /// Empty on 1.12: it always refers to the open window.
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOOT_CLEAR_MONEY)
    }
}
