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

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::game::taxi::{TaxiMask, TAXI_MASK_SIZE};

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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_taxinode_status() {
        let msg = SmsgTaxinodeStatus {
            creature_guid: ObjectGuid::from_low(123),
            is_known: true,
        };
        let packet = msg.to_world_packet();
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
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SHOWTAXINODES);
    }

    #[test]
    fn test_smsg_new_taxi_path() {
        let msg = SmsgNewTaxiPath {};
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_NEW_TAXI_PATH);
    }

    #[test]
    fn test_smsg_activate_taxi_reply() {
        let msg = SmsgActivateTaxiReply { reply: ERR_TAXIOK };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_ACTIVATETAXIREPLY);
    }
}
