//! Taxi system message structs
//!
//! This module contains type-safe message structures for all taxi-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgTaxinodeStatus`] - Status of a specific taxinode
//! - [`SmsgShowTaxinodes`] - Shows all available taxinodes
//! - [`SmsgNewTaxiPath`] - Notification of a new taxi path
//! - [`SmsgActivateTaxiReply`] - Response to a taxi activation request

use crate::game::taxi::{TaxiMask, TAXI_MASK_SIZE};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// SMSG_TAXINODE_STATUS - Status of a specific taxinode
///
/// Sent to the player to indicate if a specific taxinode is known.
#[derive(Debug, Clone)]
pub struct SmsgTaxinodeStatus {
    /// GUID of the creature associated with the taxinode
    pub creature_guid: ObjectGuid,
    /// Whether the taxinode is known to the player
    pub is_known: bool,
}

impl ToWorldPacket for SmsgTaxinodeStatus {
    /// 1.14 replaces the trailing known/unknown byte with a 2-bit `TaxiNodeStatus` code.
    ///
    /// The codes are 0 = None, 1 = Learned, 2 = Unlearned, 3 = NotEligible, so the false case is
    /// **2, not 0**. Passing vanilla's `0` through tells the client there is no flight point on
    /// this NPC at all, and it silently declines to open the flight map instead of showing the
    /// node greyed out.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.creature_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_bits(if self.is_known { 1 } else { 2 }, 2);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_TAXINODE_STATUS))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TAXINODE_STATUS);
        packet.write_guid_raw(self.creature_guid.raw());
        packet.write_u8(if self.is_known { 1 } else { 0 });
        packet
    }
}

/// SMSG_SHOWTAXINODES - Shows all available taxinodes
///
/// Sent to the player when they interact with a flight master.
#[derive(Debug, Clone)]
pub struct SmsgShowTaxinodes {
    /// GUID of the flight master creature
    pub creature_guid: ObjectGuid,
    /// Current taxinode ID
    pub current_node: u32,
    /// Mask of known taxinodes
    pub taxi_mask: TaxiMask,
}

impl ToWorldPacket for SmsgShowTaxinodes {
    /// The 1.14 layout front-loads both list lengths and splits the mask into two lists.
    ///
    /// Three things differ from vanilla and all three are silent failures if got wrong:
    ///
    /// * The leading `1` word becomes a single bit meaning "a flight master and current node
    ///   follow". Vanilla always names both, so the bit is always set.
    /// * Both list lengths are written **before** the GUID and current node, and the mask bytes
    ///   come after them. Vanilla interleaves the GUID and node ahead of the mask.
    /// * The mask is a plain byte-per-8-nodes bitmap here, where vanilla sends the same bits as
    ///   eight `u32` words. `TaxiMask::as_bytes` writes the words little-endian, which puts node
    ///   `n` at the same wire position under both encodings.
    ///
    /// 1.14 splits the mask into "can land" and "can use", which lets a server temporarily bar a
    /// node the player already knows. Vanilla has no such distinction, so both lists are the same.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bit(true); // HasWindowInfo
        writer.flush_bits();

        // Trailing zero bytes are trimmed; the client treats any node past the end as not known.
        let nodes = self.taxi_mask.as_bytes();
        let len = nodes
            .iter()
            .rposition(|&byte| byte != 0)
            .map_or(0, |i| i + 1);

        writer.write_i32(len as i32); // CanLandNodes count
        writer.write_i32(len as i32); // CanUseNodes count

        let (high, low) = self.creature_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.current_node);

        writer.write_bytes(&nodes[..len]);
        writer.write_bytes(&nodes[..len]);
        Some(writer.finish(Opcode::SMSG_SHOWTAXINODES))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SHOWTAXINODES);
        packet.write_u32(1);
        packet.write_guid_raw(self.creature_guid.raw());
        packet.write_u32(self.current_node);

        let mask_array = self.taxi_mask.as_array();
        for i in 0..TAXI_MASK_SIZE {
            packet.write_u32(mask_array[i]);
        }

        packet
    }
}

/// SMSG_NEW_TAXI_PATH - Notification of a new taxi path
///
/// Sent to the player when they select a new taxi path.
#[derive(Debug, Clone)]
pub struct SmsgNewTaxiPath {}

impl ToWorldPacket for SmsgNewTaxiPath {
    /// Empty body in 1.14 as well, so the vanilla packet is byte-identical.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_NEW_TAXI_PATH)
    }
}

/// SMSG_ACTIVATETAXIREPLY - Response to a taxi activation request
///
/// Sent to the player in response to a taxi activation request.
#[derive(Debug, Clone)]
pub struct SmsgActivateTaxiReply {
    /// Reply code (0 = OK, 1 = Error, etc.)
    pub reply: u32,
}

impl ToWorldPacket for SmsgActivateTaxiReply {
    /// 1.14 packs the reply into 4 bits where vanilla spends a whole u32 on it.
    ///
    /// The numbering is unchanged — 0 = Ok through 12 = NotStanding, the same set the `ERR_TAXI*`
    /// constants below name — so the value carries over as-is. The mask keeps a bad caller from
    /// bleeding high bits into whatever follows; every defined code already fits in 4 bits.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bits(self.reply & 0xF, 4);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_ACTIVATETAXIREPLY))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ACTIVATETAXIREPLY);
        packet.write_u32(self.reply);
        packet
    }
}

/// Taxi activation reply codes
pub const ERR_TAXIOK: u32 = 0;
pub const ERR_TAXIUNSPECIFIEDSERVERERROR: u32 = 1;
pub const ERR_TAXINOSUCHPATH: u32 = 2;
pub const ERR_TAXINOTENOUGHMONEY: u32 = 3;
pub const ERR_TAXITOOFARAWAY: u32 = 4;
pub const ERR_TAXINOVENDORNEARBY: u32 = 5;
pub const ERR_TAXINOTVISITED: u32 = 6;
pub const ERR_TAXIPLAYERBUSY: u32 = 7;
pub const ERR_TAXIPLAYERALREADYMOUNTED: u32 = 8;
pub const ERR_TAXIPLAYERSHAPESHIFTED: u32 = 9;
pub const ERR_TAXIPLAYERMOVING: u32 = 10;
pub const ERR_TAXISAMENODE: u32 = 11;
pub const ERR_TAXINOTSTANDING: u32 = 12;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_taxinode_status() {
        let msg = SmsgTaxinodeStatus {
            creature_guid: ObjectGuid::from_low(123),
            is_known: true,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TAXINODE_STATUS);
    }

    #[test]
    fn test_smsg_show_taxinodes() {
        let mut taxi_mask = TaxiMask::new();
        taxi_mask.set(1);
        taxi_mask.set(2);
        taxi_mask.set(3);

        let msg = SmsgShowTaxinodes {
            creature_guid: ObjectGuid::from_low(123),
            current_node: 1,
            taxi_mask,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_SHOWTAXINODES);
    }

    #[test]
    fn test_smsg_new_taxi_path() {
        let msg = SmsgNewTaxiPath {};
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_NEW_TAXI_PATH);
    }

    #[test]
    fn test_smsg_activate_taxi_reply() {
        let msg = SmsgActivateTaxiReply { reply: ERR_TAXIOK };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_ACTIVATETAXIREPLY);
    }
}
