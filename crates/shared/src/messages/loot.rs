use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};

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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOOT_MONEY_NOTIFY);
        packet.write_u32(self.gold);
        packet
    }
}

/// SMSG_LOOT_CLEAR_MONEY (0x0165)
#[derive(Debug, Clone)]
pub struct SmsgLootClearMoney;

impl ToWorldPacket for SmsgLootClearMoney {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOOT_CLEAR_MONEY)
    }
}
