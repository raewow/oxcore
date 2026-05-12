//! Trade system message structs
//!
//! This module contains type-safe message structures for all trade-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgTradeStatus`] - Trade status update
//! - [`SmsgTradeStatusExtended`] - Extended trade status with item details

use crate::shared::game::trade::{TradeStatus, TRADE_SLOT_COUNT};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

/// SMSG_TRADE_STATUS - Trade status update
///
/// Sent to the player to update the trade status.
#[derive(Debug, Clone)]
pub struct SmsgTradeStatus {
    /// Current trade status
    pub status: TradeStatus,
    /// Optional GUID of the trade partner
    pub partner_guid: Option<ObjectGuid>,
}

impl ToWorldPacket for SmsgTradeStatus {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRADE_STATUS);
        packet.write_u32(self.status as u32);

        match self.status {
            TradeStatus::BeginTrade => {
                if let Some(guid) = self.partner_guid {
                    packet.write_guid_raw(guid.raw());
                } else {
                    packet.write_u64(0);
                }
            }
            TradeStatus::CloseWindow => {
                packet.write_u32(0);
                packet.write_u8(0);
                packet.write_u32(0);
            }
            TradeStatus::OnlyConjured => {
                packet.write_u8(0);
            }
            _ => {}
        }

        packet
    }
}

// ========== PACKET STRUCTS ==========

/// Trade slot information for V2 packets (pre-resolved, no ObjectMgr needed)
#[derive(Debug, Clone, Default)]
pub struct TradeSlotInfoV2 {
    pub slot_index: u8,
    pub item_entry: u32,
    pub display_id: u32,
    pub count: u32,
    pub wrapped: bool,
    pub gift_creator_guid: ObjectGuid,
    pub permanent_enchant: u32,
    pub creator_guid: ObjectGuid,
    pub charges: i32,
    pub suffix_factor: u32,
    pub random_property_id: i32,
    pub lock_id: u32,
    pub max_durability: u32,
    pub durability: u32,
}

impl TradeSlotInfoV2 {
    pub fn empty(slot_index: u8) -> Self {
        Self {
            slot_index,
            ..Default::default()
        }
    }
}

/// SMSG_TRADE_STATUS for world - uses owned data
#[derive(Debug, Clone)]
pub struct SmsgTradeStatusV2 {
    /// Current trade status
    pub status: TradeStatus,
    /// Optional GUID of the trade partner (for BeginTrade status)
    pub partner_guid: Option<ObjectGuid>,
}

impl ToWorldPacket for SmsgTradeStatusV2 {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRADE_STATUS);
        packet.write_u32(self.status as u32);

        match self.status {
            TradeStatus::BeginTrade => {
                if let Some(guid) = self.partner_guid {
                    packet.write_guid_raw(guid.raw());
                } else {
                    packet.write_u64(0);
                }
            }
            TradeStatus::CloseWindow => {
                packet.write_u32(0); // inventory_result
                packet.write_u8(0); // target_error
                packet.write_u32(0); // item_limit_category_id
            }
            TradeStatus::OnlyConjured | TradeStatus::NotOnTaplist => {
                packet.write_u8(0); // slot
            }
            _ => {}
        }

        packet
    }
}

/// SMSG_TRADE_STATUS_EXTENDED for world - uses pre-resolved slot data
#[derive(Debug, Clone)]
pub struct SmsgTradeStatusExtendedV2 {
    /// Whether this shows trader's view (true) or player's own view (false)
    pub is_trader_view: bool,
    /// Trade slots (7 total: 0-5 traded, 6 non-traded for enchanting)
    pub trade_slots: [Option<TradeSlotInfoV2>; TRADE_SLOT_COUNT],
    /// Gold amount in copper
    pub gold: u32,
    /// Enchantment spell ID
    pub spell_id: u32,
}

impl Default for SmsgTradeStatusExtendedV2 {
    fn default() -> Self {
        Self {
            is_trader_view: false,
            trade_slots: Default::default(),
            gold: 0,
            spell_id: 0,
        }
    }
}

impl ToWorldPacket for SmsgTradeStatusExtendedV2 {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRADE_STATUS_EXTENDED);

        // Header
        packet.write_u8(if self.is_trader_view { 1 } else { 0 });
        packet.write_u32(TRADE_SLOT_COUNT as u32); // trade_slot_count1
        packet.write_u32(TRADE_SLOT_COUNT as u32); // trade_slot_count2
        packet.write_u32(self.gold);
        packet.write_u32(self.spell_id);

        // Write each slot
        for slot_idx in 0..TRADE_SLOT_COUNT {
            packet.write_u8(slot_idx as u8);

            if let Some(ref slot) = self.trade_slots[slot_idx] {
                packet.write_u32(slot.item_entry);
                packet.write_u32(slot.display_id);
                packet.write_u32(slot.count);
                packet.write_u32(if slot.wrapped { 1 } else { 0 });
                packet.write_guid_raw(slot.gift_creator_guid.raw());
                packet.write_u32(slot.permanent_enchant);
                packet.write_guid_raw(slot.creator_guid.raw());
                packet.write_u32(slot.charges as u32);
                packet.write_u32(slot.suffix_factor);
                packet.write_u32(slot.random_property_id as u32);
                packet.write_u32(slot.lock_id);
                packet.write_u32(slot.max_durability);
                packet.write_u32(slot.durability);
            } else {
                // Empty slot: write 15 u32 zeros (item_entry through durability)
                for _ in 0..15 {
                    packet.write_u32(0);
                }
            }
        }

        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_trade_status() {
        let msg = SmsgTradeStatus {
            status: TradeStatus::BeginTrade,
            partner_guid: Some(ObjectGuid::from_low(123)),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS);
    }

    #[test]
    fn test_smsg_trade_status_v2_begin_trade() {
        let msg = SmsgTradeStatusV2 {
            status: TradeStatus::BeginTrade,
            partner_guid: Some(ObjectGuid::from_low(456)),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS);
    }

    #[test]
    fn test_smsg_trade_status_v2_complete() {
        let msg = SmsgTradeStatusV2 {
            status: TradeStatus::TradeComplete,
            partner_guid: None,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS);
    }

    #[test]
    fn test_smsg_trade_status_extended_v2_empty() {
        let msg = SmsgTradeStatusExtendedV2::default();
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS_EXTENDED);
    }

    #[test]
    fn test_smsg_trade_status_extended_v2_with_items() {
        let mut msg = SmsgTradeStatusExtendedV2 {
            is_trader_view: true,
            trade_slots: Default::default(),
            gold: 10000,
            spell_id: 0,
        };

        msg.trade_slots[0] = Some(TradeSlotInfoV2 {
            slot_index: 0,
            item_entry: 12345,
            display_id: 54321,
            count: 5,
            ..Default::default()
        });

        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS_EXTENDED);
    }

    #[test]
    fn test_trade_slot_info_v2_empty() {
        let slot = TradeSlotInfoV2::empty(3);
        assert_eq!(slot.slot_index, 3);
        assert_eq!(slot.item_entry, 0);
    }
}
