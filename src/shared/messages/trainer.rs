//! Trainer system message structs
//!
//! ## Server Messages (SMSG)
//! - [`SmsgTrainerList`] - Trainer spell list
//! - [`SmsgTrainerBuySucceeded`] - Spell purchase success
//! - [`SmsgTrainerBuyFailed`] - Spell purchase failure

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::shared::protocol::packet::WorldPacketGuidExt;

/// Per-spell data for SMSG_TRAINER_LIST
#[derive(Debug, Clone)]
pub struct TrainerSpellData {
    /// The spell ID (the "wrapper" spell in npc_trainer)
    pub spell_id: u32,
    /// Trainer spell state: 0=green (available), 1=red (unavailable), 2=grey (known)
    pub state: u8,
    /// Cost in copper
    pub cost: u32,
    /// Primary profession learn (1 if this is a primary prof first rank AND player has free slot, else 0)
    pub primary_prof_first_rank_available: u32,
    /// Is first rank of primary profession (1 or 0)
    pub primary_prof_first_rank: u32,
    /// Required level
    pub req_level: u8,
    /// Required skill ID
    pub req_skill: u32,
    /// Required skill value
    pub req_skill_value: u32,
    /// Prerequisite spell 1 (chain node req or prev)
    pub req_spell_1: u32,
    /// Prerequisite spell 2 (chain node prev if req set, else 0)
    pub req_spell_2: u32,
    /// Unknown (always 0 in reference)
    pub unknown: u32,
}

/// SMSG_TRAINER_LIST (0x1B1) - Trainer spell list
#[derive(Debug, Clone)]
pub struct SmsgTrainerList {
    pub trainer_guid: ObjectGuid,
    pub trainer_type: u32,
    pub spells: Vec<TrainerSpellData>,
    pub greeting: String,
}

impl ToWorldPacket for SmsgTrainerList {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRAINER_LIST);

        packet.write_guid(self.trainer_guid);
        packet.write_u32(self.trainer_type);
        packet.write_u32(self.spells.len() as u32);

        for spell in &self.spells {
            packet.write_u32(spell.spell_id);
            packet.write_u8(spell.state);
            packet.write_u32(spell.cost);
            packet.write_u32(spell.primary_prof_first_rank_available);
            packet.write_u32(spell.primary_prof_first_rank);
            packet.write_u8(spell.req_level);
            packet.write_u32(spell.req_skill);
            packet.write_u32(spell.req_skill_value);
            packet.write_u32(spell.req_spell_1);
            packet.write_u32(spell.req_spell_2);
            packet.write_u32(spell.unknown);
        }

        packet.write_cstring(&self.greeting);

        packet
    }
}

/// SMSG_TRAINER_BUY_SUCCEEDED (0x1B3)
#[derive(Debug, Clone)]
pub struct SmsgTrainerBuySucceeded {
    pub trainer_guid: ObjectGuid,
    pub spell_id: u32,
}

impl ToWorldPacket for SmsgTrainerBuySucceeded {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRAINER_BUY_SUCCEEDED);
        packet.write_guid(self.trainer_guid);
        packet.write_u32(self.spell_id);
        packet
    }
}

/// Trainer buy failure codes
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum TrainerBuyError {
    Unavailable = 0,
    NotEnoughMoney = 1,
    SkillNotMet = 2,
}

/// SMSG_TRAINER_BUY_FAILED (0x1B4)
#[derive(Debug, Clone)]
pub struct SmsgTrainerBuyFailed {
    pub trainer_guid: ObjectGuid,
    pub spell_id: u32,
    pub error: TrainerBuyError,
}

impl ToWorldPacket for SmsgTrainerBuyFailed {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_TRAINER_BUY_FAILED);
        packet.write_guid(self.trainer_guid);
        packet.write_u32(self.spell_id);
        packet.write_u32(self.error as u32);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::ObjectGuid;

    fn read_u32_le(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
    }

    fn read_u64_le(data: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
    }

    fn read_cstring(data: &[u8], offset: usize) -> (String, usize) {
        let end = data[offset..].iter().position(|&b| b == 0).unwrap() + offset;
        let s = std::str::from_utf8(&data[offset..end]).unwrap().to_string();
        (s, end + 1)
    }

    fn trainer_guid() -> ObjectGuid {
        ObjectGuid::from_raw(0xF130_0000_00C6_0001)
    }

    fn one_spell() -> TrainerSpellData {
        TrainerSpellData {
            spell_id: 1142,
            state: 0,
            cost: 100,
            primary_prof_first_rank_available: 0,
            primary_prof_first_rank: 0,
            req_level: 4,
            req_skill: 0,
            req_skill_value: 0,
            req_spell_1: 0,
            req_spell_2: 0,
            unknown: 0,
        }
    }

    #[test]
    fn smsg_trainer_list_guid_is_unpacked() {
        // Trainer GUID must be fixed 8 bytes (unpacked), matching vmangos
        // NPCHandler.cpp: data << ObjectGuid(guid)
        let msg = SmsgTrainerList {
            trainer_guid: trainer_guid(),
            trainer_type: 0,
            spells: vec![],
            greeting: String::new(),
        };
        let pkt = msg.to_world_packet();
        let data = pkt.data();

        assert_eq!(read_u64_le(data, 0), 0xF130_0000_00C6_0001,
            "Trainer GUID must be unpacked (fixed 8 bytes)");
    }

    #[test]
    fn smsg_trainer_list_field_order() {
        // Full per-spell layout (vmangos SendTrainerSpellHelper):
        // spell_id(u32) | state(u8) | cost(u32) | prof_avail(u32) | first_rank(u32)
        // | req_level(u8) | req_skill(u32) | req_skill_value(u32)
        // | req_spell_1(u32) | req_spell_2(u32) | unknown(u32)
        let msg = SmsgTrainerList {
            trainer_guid: trainer_guid(),
            trainer_type: 0,
            spells: vec![one_spell()],
            greeting: "Hi".to_string(),
        };
        let pkt = msg.to_world_packet();
        let data = pkt.data();

        let mut pos = 0;
        // Header
        assert_eq!(read_u64_le(data, pos), trainer_guid().raw()); pos += 8; // guid
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;  // trainer_type
        assert_eq!(read_u32_le(data, pos), 1); pos += 4;  // spell count

        // Spell entry
        assert_eq!(read_u32_le(data, pos), 1142); pos += 4; // spell_id
        assert_eq!(data[pos], 0); pos += 1;                 // state (u8)
        assert_eq!(read_u32_le(data, pos), 100); pos += 4;  // cost
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // primary_prof_first_rank_available
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // primary_prof_first_rank
        assert_eq!(data[pos], 4); pos += 1;                 // req_level (u8)
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // req_skill
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // req_skill_value
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // req_spell_1
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // req_spell_2
        assert_eq!(read_u32_le(data, pos), 0); pos += 4;    // unknown

        // Greeting cstring
        let (greeting, _) = read_cstring(data, pos);
        assert_eq!(greeting, "Hi");
    }

    #[test]
    fn smsg_trainer_list_greeting_null_terminated() {
        let msg = SmsgTrainerList {
            trainer_guid: trainer_guid(),
            trainer_type: 0,
            spells: vec![],
            greeting: "Hello trainer".to_string(),
        };
        let pkt = msg.to_world_packet();
        let data = pkt.data();

        // After GUID(8) + trainer_type(4) + count(4) = offset 16
        let (greeting, end) = read_cstring(data, 16);
        assert_eq!(greeting, "Hello trainer");
        // Null byte must be present (read_cstring consumes it, so end == data.len())
        assert_eq!(end, data.len(), "Greeting must be the last field, null-terminated");
    }
}
