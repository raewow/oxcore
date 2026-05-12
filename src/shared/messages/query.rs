//! Query message structs - name, creature, item queries

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};

// =========================================================================
// CREATURE QUERY RESPONSE
// =========================================================================

/// SMSG_CREATURE_QUERY_RESPONSE - Response to creature template query
///
/// Sent when a client requests creature info (name, type, family, etc.)
/// The client sends CMSG_CREATURE_QUERY when it needs to display a creature.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - entry (u32) - Creature entry (high bit set if not found)
/// - name (cstring) - Creature name
/// - name2 (u8) - Always 0 (null terminator)
/// - name3 (u8) - Always 0 (null terminator)
/// - name4 (u8) - Always 0 (null terminator)
/// - subname (cstring) - Creature subname/title (e.g. "General Goods")
/// - type_flags (u32) - Static flags
/// - creature_type (u32) - Beast, Humanoid, Undead, etc.
/// - creature_family (u32) - For hunter pets (Wolf, Cat, etc.)
/// - rank (u32) - Normal, Elite, Rare, Rare Elite, Boss
/// - unknown (u32) - Always 0
/// - pet_spell_data_id (u32) - For pets with spells
/// - display_id (u32) - Model to display
/// - civilian (u8) - Civilian flag (no PvP flagging)
/// - racial_leader (u8) - Racial leader flag
#[derive(Debug, Clone)]
pub struct SmsgCreatureQueryResponse<'a> {
    /// Creature entry ID
    pub entry: u32,
    /// Creature name
    pub name: &'a str,
    /// Creature subname/title (e.g. "General Goods", "Quest Giver")
    pub subname: &'a str,
    /// Type flags (static_flags)
    pub type_flags: u32,
    /// Creature type (0=None, 1=Beast, 2=Dragon, 3=Demon, 4=Elemental, 5=Giant,
    /// 6=Undead, 7=Humanoid, 8=Critter, 9=Mechanical, 10=Not specified)
    pub creature_type: u8,
    /// Creature family for hunter pets (0=None, 1=Wolf, 2=Cat, etc.)
    pub creature_family: u8,
    /// Creature rank (0=Normal, 1=Elite, 2=Rare Elite, 3=Boss, 4=Rare)
    pub rank: u8,
    /// Pet spell data ID (for pets with special abilities)
    pub pet_spell_data_id: u32,
    /// Display ID (model to render)
    pub display_id: u32,
    /// Civilian flag (prevents PvP flagging)
    pub civilian: u8,
    /// Racial leader flag
    pub racial_leader: u8,
}

impl<'a> SmsgCreatureQueryResponse<'a> {
    /// Create a new creature query response
    pub fn new(
        entry: u32,
        name: &'a str,
        subname: &'a str,
        type_flags: u32,
        creature_type: u8,
        display_id: u32,
    ) -> Self {
        Self {
            entry,
            name,
            subname,
            type_flags,
            creature_type,
            creature_family: 0,
            rank: 0,
            pet_spell_data_id: 0,
            display_id,
            civilian: 0,
            racial_leader: 0,
        }
    }

    /// Create a "not found" response
    ///
    /// When entry is not found, we set the high bit (0x80000000) and send minimal data.
    pub fn not_found(entry: u32) -> Self {
        Self {
            entry: entry | 0x80000000, // High bit indicates not found
            name: "",
            subname: "",
            type_flags: 0,
            creature_type: 0,
            creature_family: 0,
            rank: 0,
            pet_spell_data_id: 0,
            display_id: 0,
            civilian: 0,
            racial_leader: 0,
        }
    }
}

impl ToWorldPacket for SmsgCreatureQueryResponse<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CREATURE_QUERY_RESPONSE);
        packet.write_u32(self.entry);

        // Only write remaining fields if entry high bit is not set (found)
        if (self.entry & 0x80000000) == 0 {
            packet.write_cstring(self.name);
            packet.write_u8(0); // name2 (null terminator)
            packet.write_u8(0); // name3 (null terminator)
            packet.write_u8(0); // name4 (null terminator)
            packet.write_cstring(self.subname);
            packet.write_u32(self.type_flags);
            packet.write_u32(self.creature_type as u32);
            packet.write_u32(self.creature_family as u32);
            packet.write_u32(self.rank as u32);
            packet.write_u32(0); // unknown (always 0)
            packet.write_u32(self.pet_spell_data_id);
            packet.write_u32(self.display_id);
            packet.write_u8(self.civilian);
            packet.write_u8(self.racial_leader);
        }

        packet
    }
}

// =========================================================================
// NAME QUERY RESPONSE
// =========================================================================

/// SMSG_NAME_QUERY_RESPONSE - Response to name query
///
/// Sent when a client requests the name/info for a player GUID.
/// Used for chat messages, target frames, etc.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - guid (u64) - Player GUID (NOT packed)
/// - name (cstring) - Player name
/// - realm (u8) - Realm name (0 for same realm)
/// - race (u32) - Player race
/// - gender (u32) - Player gender
/// - class (u32) - Player class
#[derive(Debug, Clone)]
pub struct SmsgNameQueryResponse<'a> {
    /// Player GUID
    pub guid: ObjectGuid,
    /// Player name
    pub name: &'a str,
    /// Realm name (empty string for same realm)
    pub realm: &'a str,
    /// Player race
    pub race: u8,
    /// Player gender
    pub gender: u8,
    /// Player class
    pub class: u8,
}

impl<'a> SmsgNameQueryResponse<'a> {
    /// Create a new name query response
    pub fn new(guid: ObjectGuid, name: &'a str, race: u8, gender: u8, class: u8) -> Self {
        Self {
            guid,
            name,
            realm: "",
            race,
            gender,
            class,
        }
    }
}

impl ToWorldPacket for SmsgNameQueryResponse<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_NAME_QUERY_RESPONSE);

        // Write GUID as u64 (NOT packed - vanilla 1.12.1 format)
        packet.write_u64(self.guid.raw());

        // Write name (null-terminated string)
        packet.write_cstring(self.name);

        // Write realm name
        if self.realm.is_empty() {
            packet.write_u8(0); // Empty string (null terminator only)
        } else {
            packet.write_cstring(self.realm);
        }

        // Write race, gender, class (for cross-realm support)
        packet.write_u32(self.race as u32);
        packet.write_u32(self.gender as u32);
        packet.write_u32(self.class as u32);

        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smsg_name_query_response() {
        let guid = ObjectGuid::from_low(42);
        let msg = SmsgNameQueryResponse::new(guid, "TestPlayer", 1, 0, 1);
        let packet = msg.to_world_packet();

        assert_eq!(packet.opcode(), Opcode::SMSG_NAME_QUERY_RESPONSE);

        // Verify the packet contains the correct data
        let data = packet.contents();

        // First 8 bytes should be the GUID (little-endian u64)
        // GUID counter is 42, so raw() should be 0x0000000000000042 in little-endian
        assert_eq!(data[0], 0x2A); // 42 in hex = 0x2A
        assert_eq!(data[1], 0x00);
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        assert_eq!(data[4], 0x00);
        assert_eq!(data[5], 0x00);
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 0x00);
    }

    #[test]
    fn test_smsg_name_query_response_guid_writing() {
        // Test that GUID is written as u64, not packed
        let guid = ObjectGuid::from_low(1000);
        let msg = SmsgNameQueryResponse::new(guid, "Player", 1, 0, 1);
        let packet = msg.to_world_packet();

        let data = packet.contents();

        // Verify that the first 8 bytes are the full u64 GUID (little-endian)
        // 1000 = 0x3E8 in hex, so little-endian is [E8 03 00 00 00 00 00 00]
        // This is critical for things like chat to work.*
        assert_eq!(data[0], 0xE8);
        assert_eq!(data[1], 0x03);
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        assert_eq!(data[4], 0x00);
        assert_eq!(data[5], 0x00);
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 0x00);
    }

    #[test]
    fn test_smsg_name_query_response_with_realm() {
        let guid = ObjectGuid::from_low(100);
        let mut msg = SmsgNameQueryResponse::new(guid, "TestPlayer", 1, 0, 1);
        msg.realm = "TestRealm";

        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_NAME_QUERY_RESPONSE);
    }
}
