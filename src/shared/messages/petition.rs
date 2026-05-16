//! Petition system message structs
//!
//! This module contains type-safe message structures for all petition-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgPetitionShowlist`] - Show available guild charters for purchase
//! - [`SmsgPetitionShowSignatures`] - Show signatures on a guild charter
//! - [`SmsgPetitionSignResults`] - Result of attempting to sign a charter
//! - [`SmsgTurnInPetitionResults`] - Result of turning in a completed charter
//! - [`SmsgPetitionQueryResponse`] - Response to a petition query with charter details

use crate::shared::game::petition::{PetitionResult, PetitionSignature};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

/// SMSG_PETITION_SHOWLIST - Show available guild charters for purchase from an NPC
///
/// Sent when a player interacts with a guild master NPC.
/// Shows the available guild charters that can be purchased.
#[derive(Debug, Clone)]
pub struct SmsgPetitionShowlist {
    /// GUID of the NPC offering the guild charter
    pub npc_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgPetitionShowlist {
    fn to_world_packet(&self) -> WorldPacket {
        const CHARTER_DISPLAY_ID: u32 = 16161;
        const CHARTER_ENTRY_GENERIC: u32 = 5863;
        const CHARTER_COST: u32 = 1000;

        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_SHOWLIST);
        packet.write_guid_raw(self.npc_guid.raw());
        packet.write_u8(1); // amount_of_petitions - always 1 for guild charters in Vanilla

        // Petition info (Vanilla 1.12.1 format - no signatures_required field)
        packet.write_u32(0); // index - always 0 for first/only petition
        packet.write_u32(CHARTER_ENTRY_GENERIC); // charter_entry - 5863
        packet.write_u32(CHARTER_DISPLAY_ID); // charter_display_id - 16161
        packet.write_u32(CHARTER_COST); // guild_charter_cost - 1000 copper (10 silver)
        packet.write_u32(1); // unknown1 - always 1 for guild charters

        packet
    }
}

/// SMSG_PETITION_SHOW_SIGNATURES - Show all signatures collected on a charter
///
/// Sent when a player views the signatures on a guild charter they own.
#[derive(Debug)]
pub struct SmsgPetitionShowSignatures<'a> {
    /// GUID of the charter item
    pub charter_guid: ObjectGuid,
    /// GUID of the charter owner
    pub owner_guid: ObjectGuid,
    /// Petition ID
    pub petition_guid: u32,
    /// Reference to array of signatures on the charter
    pub signatures: &'a [PetitionSignature],
}

impl ToWorldPacket for SmsgPetitionShowSignatures<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_SHOW_SIGNATURES);

        packet.write_guid_raw(self.charter_guid.raw()); // Charter item GUID
        packet.write_guid_raw(self.owner_guid.raw()); // Owner GUID
        packet.write_u32(self.petition_guid); // Petition ID

        // Write signature count
        packet.write_u8(self.signatures.len() as u8);

        // Write each signature
        for signature in self.signatures {
            packet.write_guid_raw(signature.player_guid.raw()); // Signer GUID
            packet.write_u32(0); // unknown1 - always 0
        }

        packet
    }
}

/// SMSG_PETITION_SIGN_RESULTS - Result of attempting to sign a charter
///
/// Sent when a player attempts to sign a guild charter.
#[derive(Debug, Clone)]
pub struct SmsgPetitionSignResults {
    /// GUID of the charter item
    pub charter_guid: ObjectGuid,
    /// GUID of the charter owner
    pub owner_guid: ObjectGuid,
    /// Result of the signature attempt
    pub result: PetitionResult,
}

impl ToWorldPacket for SmsgPetitionSignResults {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_SIGN_RESULTS);

        packet.write_guid_raw(self.charter_guid.raw()); // Charter item GUID
        packet.write_guid_raw(self.owner_guid.raw()); // Owner GUID
        packet.write_u32(self.result.as_u32()); // Result code

        packet
    }
}

/// SMSG_TURN_IN_PETITION_RESULTS - Result of turning in a completed charter
///
/// Sent when a player turns in a completed guild charter.
#[derive(Debug, Clone)]
pub struct SmsgTurnInPetitionResults {
    /// Result of the turn-in attempt
    pub result: PetitionResult,
}

impl ToWorldPacket for SmsgTurnInPetitionResults {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TURN_IN_PETITION_RESULTS);
        packet.write_u32(self.result.as_u32()); // Result code
        packet
    }
}

/// SMSG_PETITION_QUERY_RESPONSE - Response to a petition query with charter details
///
/// Sent in response to a petition query, contains the charter's details.
#[derive(Debug, Clone)]
pub struct SmsgPetitionQueryResponse<'a> {
    /// Petition ID
    pub petition_guid: u32,
    /// GUID of the charter owner
    pub owner_guid: ObjectGuid,
    /// Name of the guild being created
    pub guild_name: &'a str,
}

impl ToWorldPacket for SmsgPetitionQueryResponse<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_QUERY_RESPONSE);

        packet.write_u32(self.petition_guid); // Petition ID
        packet.write_guid_raw(self.owner_guid.raw()); // Charter owner GUID
        packet.write_string(self.guild_name); // Guild name
        packet.write_string(""); // Body text (empty for guild charters)
        packet.write_u32(0); // Signatures required (0 in Vanilla - client knows it's 9)
        packet.write_u32(0); // Unknown flags
        packet.write_u32(0); // Unknown
        packet.write_u32(0); // Unknown
        packet.write_u32(0); // Unknown
        packet.write_u32(0); // Unknown
        packet.write_u16(0); // Unknown
        packet.write_u32(0); // Type (0 = guild)

        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::game::petition::PetitionResult;
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_petition_showlist() {
        let msg = SmsgPetitionShowlist {
            npc_guid: ObjectGuid::from_low(123),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_SHOWLIST);
    }

    #[test]
    fn test_smsg_petition_show_signatures() {
        let signatures = vec![
            PetitionSignature {
                player_guid: ObjectGuid::from_low(123),
                player_account: 1,
                name: String::new(),
                offer_result: PetitionResult::Ok,
            },
            PetitionSignature {
                player_guid: ObjectGuid::from_low(456),
                player_account: 2,
                name: String::new(),
                offer_result: PetitionResult::Ok,
            },
        ];

        let msg = SmsgPetitionShowSignatures {
            charter_guid: ObjectGuid::from_low(789),
            owner_guid: ObjectGuid::from_low(101),
            petition_guid: 112,
            signatures: &signatures,
        };

        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_SHOW_SIGNATURES);
    }

    #[test]
    fn test_smsg_petition_sign_results() {
        let msg = SmsgPetitionSignResults {
            charter_guid: ObjectGuid::from_low(123),
            owner_guid: ObjectGuid::from_low(456),
            result: PetitionResult::Ok,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_SIGN_RESULTS);
    }

    #[test]
    fn test_smsg_turn_in_petition_results() {
        let msg = SmsgTurnInPetitionResults {
            result: PetitionResult::Ok,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_TURN_IN_PETITION_RESULTS);
    }

    #[test]
    fn test_smsg_petition_query_response() {
        let msg = SmsgPetitionQueryResponse {
            petition_guid: 123,
            owner_guid: ObjectGuid::from_low(456),
            guild_name: "MyGuild",
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_QUERY_RESPONSE);
    }
}
