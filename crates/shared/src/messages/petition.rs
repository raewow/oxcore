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

use crate::game::petition::{PetitionResult, PetitionSignature, GUILD_CHARTER_SIGNATURES};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

// ========== MODERN ENCODING HELPERS ==========

/// A signing outcome as the 1.14 sign-result enum numbers it: `Ok` 0, `AlreadySigned` 1,
/// `AlreadyInGuild` 2, `CantSignOwn` 3, `NotServer` 5. "Needs more signatures" is not a signing
/// outcome; it folds onto the generic refusal.
///
/// The result rides in a **4-bit** field, so every value here must stay under 16.
fn modern_sign_result(result: PetitionResult) -> u32 {
    match result {
        PetitionResult::Ok => 0,
        PetitionResult::AlreadySigned => 1,
        PetitionResult::AlreadyInGuild => 2,
        PetitionResult::CantSignOwn => 3,
        PetitionResult::NeedMoreSignatures | PetitionResult::NotServer => 5,
    }
}

/// A turn-in outcome as the 1.14 turn-in enum numbers it — a *different* enum from the signing
/// one: `Ok` 0, `AlreadyInGuild` 2, `NeedMoreSignatures` 4, `HasRestriction` 13. Feeding a sign
/// result into a turn-in reply produces a wrong but plausible error message.
fn modern_turn_in_result(result: PetitionResult) -> u32 {
    match result {
        PetitionResult::Ok => 0,
        PetitionResult::AlreadyInGuild => 2,
        PetitionResult::NeedMoreSignatures => 4,
        PetitionResult::AlreadySigned | PetitionResult::CantSignOwn | PetitionResult::NotServer => {
            13
        }
    }
}

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
    fn to_vanilla(&self) -> WorldPacket {
        const CHARTER_DISPLAY_ID: u32 = 16161;
        const CHARTER_ENTRY_GENERIC: u32 = 5863;
        const CHARTER_COST: u32 = 1000;

        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_SHOWLIST);
        packet.write_guid_raw(self.npc_guid.raw());
        packet.write_u8(1); // amount_of_petitions - always 1 for guild charters in Vanilla

        // Petition info (Vanilla 1.12.1 format - no signatures_required field)
        packet.write_u32(1); // index of the one charter on offer
        packet.write_u32(CHARTER_ENTRY_GENERIC); // charter_entry - 5863
        packet.write_u32(CHARTER_DISPLAY_ID); // charter_display_id - 16161
        packet.write_u32(CHARTER_COST); // guild_charter_cost - 1000 copper (10 silver)
        packet.write_u32(1); // unknown1 - always 1 for guild charters

        packet
    }

    /// `ServerPetitionShowList` in 1.14.
    ///
    /// The per-petition record is **reordered and re-membered**, not merely widened: the cost moves
    /// ahead of the charter item, the display id is gone, and an arena flag and a required-signature
    /// count take its place. Writing vanilla's five fields in vanilla's order puts the item entry
    /// where the client reads the price, so the charter is offered at 5,863 copper.
    ///
    /// The count is an i32 rather than a byte, and the NPC's GUID is packed.
    fn to_modern(&self) -> Option<WorldPacket> {
        const CHARTER_ENTRY_GENERIC: u32 = 5863;
        const CHARTER_COST: u32 = 1000;

        let mut writer = BitWriter::new();
        let (high, low) = self.npc_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Unit
        writer.write_i32(1); // Classic Era offers exactly one charter

        writer.write_u32(0); // Index
        writer.write_u32(CHARTER_COST);
        writer.write_u32(CHARTER_ENTRY_GENERIC);
        writer.write_u32(0); // IsArena -- the generic charter is a guild charter
        writer.write_u32(GUILD_CHARTER_SIGNATURES);

        Some(writer.finish(Opcode::SMSG_PETITION_SHOWLIST))
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `ServerPetitionShowSignatures` in 1.14.
    ///
    /// 1.14 inserts a **third** GUID — the owner's game account — between the owner and the petition
    /// id, and widens the signature count from a byte to an i32. Sending vanilla's two GUIDs makes
    /// the client read the petition id and count out of the middle of the account GUID and then
    /// walk off the end of the body.
    ///
    /// That account GUID goes out empty. The struct knows each *signer's* account but never the
    /// owner's, and the field only backs an account-level duplicate-signature check the 1.12 server
    /// does not perform anyway.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.charter_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Item
        let (high, low) = self.owner_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Owner
        writer.write_packed_guid_128(0, 0); // OwnerAccountID -- see above
        writer.write_i32(self.petition_guid as i32); // PetitionID

        writer.write_i32(self.signatures.len() as i32);
        for signature in self.signatures {
            let (high, low) = signature.player_guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low); // Signer
                                                     // Which of the petition's choices was signed for. Guild charters have none.
            writer.write_i32(0); // Choice
        }

        Some(writer.finish(Opcode::SMSG_PETITION_SHOW_SIGNATURES))
    }
}

/// SMSG_PETITION_SIGN_RESULTS - Result of attempting to sign a charter
///
/// Sent when a player attempts to sign a guild charter.
#[derive(Debug, Clone)]
pub struct SmsgPetitionSignResults {
    /// GUID of the charter item
    pub charter_guid: ObjectGuid,
    /// GUID of the player who signed
    pub player_guid: ObjectGuid,
    /// Result of the signature attempt
    pub result: PetitionResult,
}

impl ToWorldPacket for SmsgPetitionSignResults {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_SIGN_RESULTS);

        packet.write_guid_raw(self.charter_guid.raw()); // Charter item GUID
        packet.write_guid_raw(self.player_guid.raw()); // Signer GUID
        packet.write_u32(self.result.as_u32()); // Result code

        packet
    }

    /// `PetitionSignResults` in 1.14.
    ///
    /// The result stops being a u32 and becomes a **4-bit** field — the body is two packed GUIDs
    /// and a single byte holding four meaningful bits. It is also a different enum; see
    /// [`modern_sign_result`], which is where the actual risk in this message lives.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.charter_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Item
        let (high, low) = self.player_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Player
        writer.write_bits(modern_sign_result(self.result), 4);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_PETITION_SIGN_RESULTS))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TURN_IN_PETITION_RESULTS);
        packet.write_u32(self.result.as_u32()); // Result code
        packet
    }

    /// `TurnInPetitionResult` in 1.14: a single byte carrying a 4-bit result, where vanilla sends a
    /// u32. The codes come from the turn-in enum, which is *not* the signing enum — see
    /// [`modern_turn_in_result`].
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bits(modern_turn_in_result(self.result), 4);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_TURN_IN_PETITION_RESULTS))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PETITION_QUERY_RESPONSE);

        packet.write_u32(self.petition_guid); // Petition ID
        packet.write_guid_raw(self.owner_guid.raw()); // Charter owner GUID
        packet.write_string(self.guild_name); // Guild name
        packet.write_string(""); // Body text (empty for guild charters)
        packet.write_u32(1); // Flags
        packet.write_u32(GUILD_CHARTER_SIGNATURES); // Minimum signatures
        packet.write_u32(GUILD_CHARTER_SIGNATURES); // Maximum signatures
        packet.write_u32(0); // Deadline
        packet.write_u32(0); // Issue date
        packet.write_u32(0); // Allowed guild
        packet.write_u32(0); // Allowed classes
        packet.write_u32(0); // Allowed races
        packet.write_u16(0); // Allowed gender
        packet.write_u32(0); // Allowed minimum level
        packet.write_u32(0); // Allowed maximum level
        packet.write_u32(0); // Choice count
        packet.write_u32(0); // Default choice

        packet
    }

    /// `QueryPetitionResponse` in 1.14.
    ///
    /// The whole body hangs off an `Allow` bit, and every string moves to the end behind a bit run:
    /// title at 7 bits, body at 12, then **ten** 6-bit choice lengths that must all be written even
    /// though a guild charter has no choices. Skipping the ten zero lengths shortens the bit run and
    /// the client reads the title out of the middle of the numeric block.
    ///
    /// The signature requirement is the one value sent as something other than what vanilla sends:
    /// vanilla writes zero and lets the client apply the rule it knows, 1.14 displays the number it
    /// is given.
    ///
    /// Everything else — deadline, issue date, the class/race/level restrictions, the choice count
    /// — is zero here because it is zero in the vanilla body too; 1.12 has no source for any of it.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.petition_guid); // PetitionID
        writer.write_bit(true); // Allow
        writer.flush_bits();

        writer.write_u32(self.petition_guid); // Info.PetitionID
        let (high, low) = self.owner_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Petitioner

        writer.write_u32(GUILD_CHARTER_SIGNATURES); // MinSignatures
        writer.write_u32(GUILD_CHARTER_SIGNATURES); // MaxSignatures
        writer.write_i32(0); // DeadLine
        writer.write_i32(0); // IssueDate
        writer.write_i32(0); // AllowedGuildID
        writer.write_i32(0); // AllowedClasses
        writer.write_i32(0); // AllowedRaces
        writer.write_i16(0); // AllowedGender
        writer.write_i32(0); // AllowedMinLevel
        writer.write_i32(0); // AllowedMaxLevel
        writer.write_i32(0); // NumChoices
        writer.write_i32(0); // StaticType
        writer.write_u32(0); // Muid

        let title = self.guild_name.as_bytes();
        writer.write_bits(title.len() as u32, 7);
        writer.write_bits(0, 12); // BodyText -- guild charters carry none
        for _ in 0..10 {
            writer.write_bits(0, 6); // Choicetext, fixed ten slots
        }
        writer.flush_bits();

        // The ten choice strings were all zero-length, so the title follows directly.
        writer.write_bytes(title);

        Some(writer.finish(Opcode::SMSG_PETITION_QUERY_RESPONSE))
    }
}

/// MSG_PETITION_RENAME - the charter's guild name was changed.
#[derive(Debug, Clone)]
pub struct MsgPetitionRename<'a> {
    pub charter_guid: ObjectGuid,
    pub new_name: &'a str,
}

impl ToWorldPacket for MsgPetitionRename<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_PETITION_RENAME);
        packet.write_guid_raw(self.charter_guid.raw());
        packet.write_string(self.new_name);
        packet
    }

    /// `SMSG_PETITION_RENAME_GUILD_RESPONSE` in 1.14: the GUID packed, then the name behind a
    /// 7-bit length.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.charter_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        let name = self.new_name.as_bytes();
        writer.write_bits(name.len() as u32, 7);
        writer.flush_bits();
        writer.write_bytes(name);
        Some(writer.finish(Opcode::SMSG_PETITION_RENAME_GUILD_RESPONSE))
    }
}

/// MSG_PETITION_DECLINE - tells a charter's owner that a player would not sign it.
#[derive(Debug, Clone)]
pub struct MsgPetitionDecline<'a> {
    pub player_guid: ObjectGuid,
    /// The decliner's name, for the 1.14 client.
    pub player_name: &'a str,
}

impl ToWorldPacket for MsgPetitionDecline<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_PETITION_DECLINE);
        packet.write_guid_raw(self.player_guid.raw());
        packet
    }

    /// 1.14 has no decline notice, so the owner reads it as a system message.
    fn to_modern(&self) -> Option<WorldPacket> {
        let text = format!("{} has declined your guild invitation.", self.player_name);
        crate::messages::chat::SmsgMessageChat::system(ObjectGuid::empty(), &text).to_modern()
    }
}

/// Result of saving a guild tabard design, numbered as both clients read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildEmblemResult {
    Success = 0,
    InvalidTabardColors = 1,
    NoGuild = 2,
    NotGuildMaster = 3,
    NotEnoughMoney = 4,
    /// Fails without telling the player why (not at a tabard designer).
    FailNoMessage = 5,
}

/// MSG_SAVE_GUILD_EMBLEM - the result of saving a tabard design.
#[derive(Debug, Clone)]
pub struct MsgSaveGuildEmblem {
    pub result: GuildEmblemResult,
}

impl ToWorldPacket for MsgSaveGuildEmblem {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_SAVE_GUILD_EMBLEM);
        packet.write_u32(self.result as u32);
        packet
    }

    /// `SMSG_PLAYER_SAVE_GUILD_EMBLEM` in 1.14: the same u32 result.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.result as u32);
        Some(writer.finish(Opcode::SMSG_PLAYER_SAVE_GUILD_EMBLEM))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::petition::PetitionResult;
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_petition_showlist() {
        let msg = SmsgPetitionShowlist {
            npc_guid: ObjectGuid::from_low(123),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_SHOWLIST);
    }

    #[test]
    fn test_smsg_petition_show_signatures() {
        let signatures = vec![
            PetitionSignature {
                player_guid: ObjectGuid::from_low(123),
                player_account: 1,
            },
            PetitionSignature {
                player_guid: ObjectGuid::from_low(456),
                player_account: 2,
            },
        ];

        let msg = SmsgPetitionShowSignatures {
            charter_guid: ObjectGuid::from_low(789),
            owner_guid: ObjectGuid::from_low(101),
            petition_guid: 112,
            signatures: &signatures,
        };

        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_SHOW_SIGNATURES);
    }

    #[test]
    fn test_smsg_petition_sign_results() {
        let msg = SmsgPetitionSignResults {
            charter_guid: ObjectGuid::from_low(123),
            player_guid: ObjectGuid::from_low(456),
            result: PetitionResult::Ok,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_SIGN_RESULTS);
    }

    #[test]
    fn test_smsg_turn_in_petition_results() {
        let msg = SmsgTurnInPetitionResults {
            result: PetitionResult::Ok,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_TURN_IN_PETITION_RESULTS);
    }

    #[test]
    fn test_smsg_petition_query_response() {
        let msg = SmsgPetitionQueryResponse {
            petition_guid: 123,
            owner_guid: ObjectGuid::from_low(456),
            guild_name: "MyGuild",
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_PETITION_QUERY_RESPONSE);
    }

    #[test]
    fn vanilla_sign_results_use_the_client_codes() {
        let msg = SmsgPetitionSignResults {
            charter_guid: ObjectGuid::from_low(1),
            player_guid: ObjectGuid::from_low(2),
            result: PetitionResult::CantSignOwn,
        };
        let packet = msg.to_vanilla();
        assert_eq!(&packet.data()[16..20], &3u32.to_le_bytes());
    }

    #[test]
    fn vanilla_query_response_carries_signature_counts() {
        let msg = SmsgPetitionQueryResponse {
            petition_guid: 7,
            owner_guid: ObjectGuid::from_low(9),
            guild_name: "A",
        };
        let packet = msg.to_vanilla();
        let data = packet.data();
        // id(4) owner(8) "A\0"(2) ""(1) flags(4) min(4) max(4)
        assert_eq!(&data[15..19], &1u32.to_le_bytes());
        assert_eq!(&data[19..23], &9u32.to_le_bytes());
        assert_eq!(&data[23..27], &9u32.to_le_bytes());
        assert_eq!(data.len(), 15 + 4 * 8 + 2 + 4 * 4);
    }
}
