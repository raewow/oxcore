//! Trade system message structs
//!
//! This module contains type-safe message structures for all trade-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgTradeStatus`] - Trade status update
//! - [`SmsgTradeStatusExtended`] - Extended trade status with item details

use crate::game::trade::{TradeStatus, TRADE_SLOT_COUNT};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// The 1.14 code for a vanilla trade status.
///
/// **Checked, and 1.14 does not renumber these.** Every vanilla value lands on the 1.14 status
/// that means the same thing, which is why the payload switch below can key off vanilla variants
/// directly. Written out anyway, matched by name with no catch-all arm, for two reasons: it is the
/// only place the correspondence is recorded, and a new vanilla variant must fail to compile here
/// rather than be cast into whichever 1.14 status shares its number — several of which carry a
/// payload, so a wrong guess truncates the body rather than just mislabelling it.
///
/// One pair agrees on number while disagreeing on meaning: vanilla's 22 is "only conjured items
/// may be traded here", 1.14's 22 is "that player is on another realm". Both take the same one-byte
/// slot payload, so the packet is well-formed either way — the player just reads the wrong excuse.
fn modern_trade_status(status: TradeStatus) -> u32 {
    match status {
        TradeStatus::Busy => 0,           // PlayerBusy
        TradeStatus::BeginTrade => 1,     // Proposed
        TradeStatus::OpenWindow => 2,     // Initiated
        TradeStatus::TradeCanceled => 3,  // Cancelled
        TradeStatus::TradeAccept => 4,    // Accepted
        TradeStatus::Busy2 => 5,          // AlreadyTrading
        TradeStatus::NoTarget => 6,       // NoTarget
        TradeStatus::BackToTrade => 7,    // Unaccepted
        TradeStatus::TradeComplete => 8,  // Complete
        TradeStatus::TradeRejected => 9,  // StateChanged
        TradeStatus::TargetTooFar => 10,  // TooFarAway
        TradeStatus::WrongFaction => 11,  // WrongFaction
        TradeStatus::CloseWindow => 12,   // Failed
        TradeStatus::Unknown13 => 13,     // Petition
        TradeStatus::IgnoreYou => 14,     // PlayerIgnored
        TradeStatus::YouStunned => 15,    // Stunned
        TradeStatus::TargetStunned => 16, // TargetStunned
        TradeStatus::YouDead => 17,       // Dead
        TradeStatus::TargetDead => 18,    // TargetDead
        TradeStatus::YouLogout => 19,     // LoggingOut
        TradeStatus::TargetLogout => 20,  // TargetLoggingOut
        TradeStatus::TrialAccount => 21,  // RestrictedAccount
        TradeStatus::OnlyConjured => 22,  // WrongRealm
        TradeStatus::NotOnTaplist => 23,  // NotOnTaplist
    }
}

/// The 1.14 `SMSG_TRADE_STATUS` body, shared by both structs below since they carry the same pair
/// of fields.
///
/// The status stops being a u32 and becomes 5 bits behind a leading account-sharing bit, and the
/// per-status payload is reshaped:
///
/// * `BeginTrade` gains a second GUID naming the partner's Battle.net game account. It is read
///   unconditionally, so leaving it out cuts the body short and the trade request never appears.
/// * `CloseWindow` reorders its three fields — the byte vanilla writes second becomes a **bit
///   written first**, ahead of the two words. Vanilla's order would have the client read the
///   inventory result out of the misaligned tail.
/// * The slot byte is required for status 23 as well as 22, which vanilla's own body only writes
///   for 22. Sending it for one and not the other leaves the client a byte short.
fn write_modern_trade_status(status: TradeStatus, partner_guid: Option<ObjectGuid>) -> WorldPacket {
    let mut writer = BitWriter::new();
    // Vanilla characters have no Battle.net account behind them, so no two ever share one.
    writer.write_bit(false); // PartnerIsSameBnetAccount
    writer.write_bits(modern_trade_status(status), 5);

    match status {
        TradeStatus::BeginTrade => {
            let (high, low) = partner_guid
                .unwrap_or_else(ObjectGuid::empty)
                .to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_packed_guid_128(0, 0); // PartnerAccount -- no bnet account mapping
        }
        TradeStatus::OpenWindow => {
            // 1.14 tags the session with an id the client quotes back on every later trade packet.
            // The field arrived after vanilla, so this body has no source for it.
            writer.write_u32(0); // Id
        }
        TradeStatus::CloseWindow => {
            // All three are the zeros the vanilla body already writes; the server never populates
            // an inventory result or an offending item here.
            writer.write_bit(false); // FailureForYou
            writer.write_i32(0); // BagResult
            writer.write_u32(0); // ItemID
        }
        TradeStatus::OnlyConjured | TradeStatus::NotOnTaplist => {
            writer.write_u8(0); // TradeSlot
        }
        // Every remaining status is payload-free in 1.14. A new vanilla variant would land here
        // silently, but it cannot reach this function without first being given a 1.14 code in
        // `modern_trade_status`, which will not compile until someone looks at both.
        _ => writer.flush_bits(),
    }

    writer.finish(Opcode::SMSG_TRADE_STATUS)
}

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
    /// See [`write_modern_trade_status`] for how the 1.14 body differs.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(write_modern_trade_status(self.status, self.partner_guid))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    /// See [`write_modern_trade_status`] for how the 1.14 body differs.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(write_modern_trade_status(self.status, self.partner_guid))
    }

    fn to_vanilla(&self) -> WorldPacket {
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

/// Not ported to 1.14: the replacement message is state-index driven and this struct has no state.
///
/// 1.14 retires `SMSG_TRADE_STATUS_EXTENDED` in favour of a trade-updated message that identifies
/// the trade by id and carries two sequence numbers — the last client state the server has seen,
/// and the server's own current state. The client quotes the current one back when the player hits
/// accept, and the server is required to reject a mismatch; that handshake is the anti-scam
/// mechanism that replaced vanilla's re-open-the-window approach. This struct has neither the trade
/// id nor either index, and they cannot be derived from a single message — they are per-session
/// counters. Sending zeros would let a trade be accepted against a stale item list, which is worse
/// than not showing the window at all.
///
/// The item records are also reshaped: 1.14 groups them into a wrapped/unwrapped split, moves gems
/// and the lock flag into a bit run, and replaces the flat display-id and enchant words with a
/// nested item instance. Those parts are all derivable from the fields here — only the session
/// state is missing.
impl ToWorldPacket for SmsgTradeStatusExtendedV2 {
    fn to_vanilla(&self) -> WorldPacket {
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
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_trade_status() {
        let msg = SmsgTradeStatus {
            status: TradeStatus::BeginTrade,
            partner_guid: Some(ObjectGuid::from_low(123)),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS);
    }

    #[test]
    fn test_smsg_trade_status_v2_begin_trade() {
        let msg = SmsgTradeStatusV2 {
            status: TradeStatus::BeginTrade,
            partner_guid: Some(ObjectGuid::from_low(456)),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS);
    }

    #[test]
    fn test_smsg_trade_status_v2_complete() {
        let msg = SmsgTradeStatusV2 {
            status: TradeStatus::TradeComplete,
            partner_guid: None,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS);
    }

    #[test]
    fn test_smsg_trade_status_extended_v2_empty() {
        let msg = SmsgTradeStatusExtendedV2::default();
        let packet = msg.to_vanilla();
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

        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TRADE_STATUS_EXTENDED);
    }

    #[test]
    fn test_trade_slot_info_v2_empty() {
        let slot = TradeSlotInfoV2::empty(3);
        assert_eq!(slot.slot_index, 3);
        assert_eq!(slot.item_entry, 0);
    }
}
