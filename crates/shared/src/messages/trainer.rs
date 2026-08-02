//! Trainer system message structs
//!
//! ## Server Messages (SMSG)
//! - [`SmsgTrainerList`] - Trainer spell list
//! - [`SmsgTrainerBuySucceeded`] - Spell purchase success
//! - [`SmsgTrainerBuyFailed`] - Spell purchase failure

use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::ObjectGuid;
use crate::protocol::{Opcode, WorldPacket};

/// The three states a trainer spell can be in, as a name rather than a number.
///
/// Both protocols have exactly these three states and both encode them as a byte -- but they number
/// them differently, so the vanilla byte must never be cast straight into the 1.14 field:
///
/// | state       | 1.12 | 1.14 |
/// |-------------|------|------|
/// | available   | 0    | 1    |
/// | unavailable | 1    | 2    |
/// | known       | 2    | 0    |
///
/// Copying the number across swaps "already known" and "buyable" on every row: known spells render
/// as trainable and the player pays for spells they have, while genuinely available spells grey out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrainerSpellState {
    Available,
    Unavailable,
    Known,
}

impl TrainerSpellState {
    /// 1.12 `TRAINER_SPELL_GREEN` -- the player can buy this now.
    const VANILLA_AVAILABLE: u8 = 0;
    /// 1.12 `TRAINER_SPELL_RED` -- requirements not met.
    const VANILLA_UNAVAILABLE: u8 = 1;
    /// 1.12 `TRAINER_SPELL_GRAY` -- already learned.
    const VANILLA_KNOWN: u8 = 2;

    /// A byte outside the three defined values is treated as unavailable: the client would render
    /// an unknown state as buyable, and offering a spell the server will refuse is worse than
    /// hiding one it would have sold.
    fn from_vanilla(state: u8) -> Self {
        match state {
            Self::VANILLA_AVAILABLE => Self::Available,
            Self::VANILLA_UNAVAILABLE => Self::Unavailable,
            Self::VANILLA_KNOWN => Self::Known,
            _ => Self::Unavailable,
        }
    }

    /// Exhaustive on purpose -- a new state must be given a 1.14 number here, not silently
    /// defaulted, so adding a variant above breaks the build.
    fn to_modern(self) -> u8 {
        match self {
            Self::Known => 0,
            Self::Available => 1,
            Self::Unavailable => 2,
        }
    }
}

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
    /// Unknown (always 0)
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
    /// 1.14 keeps the same per-spell facts but reorders them and drops the profession pair.
    ///
    /// Three things differ in ways that are silent if you get them wrong:
    ///
    /// * The state byte is **renumbered** -- see `TrainerSpellState`. It also moves from the
    ///   front of the entry to just before the required level.
    /// * `req_level` and the state byte trade places with the skill requirement: 1.14 writes
    ///   `spell, cost, skill line, skill rank, three prerequisite spells, state, level`, where
    ///   vanilla writes `spell, state, cost, profession pair, level, skill line, skill rank,
    ///   prerequisites`. Every field is still a u32 or a u8, so a wrong order produces plausible
    ///   numbers -- costs shown as skill ranks, levels shown as spell ids -- rather than a parse
    ///   failure.
    /// * A `TrainerID` is added ahead of the spell count. The client echoes it back in
    ///   `CMSG_TRAINER_BUY_SPELL`, so it has to be something stable and per-trainer; the creature
    ///   entry encoded in the trainer's own GUID is exactly that.
    ///
    /// The profession-dialog pair vanilla sends has no 1.14 field. The client derives the "you must
    /// drop a profession first" case from its own data, so dropping the pair loses nothing.
    ///
    /// The greeting moves from a trailing null-terminated string to an 11-bit length followed by
    /// the raw bytes, capping it at 2047 bytes -- far above any trainer greeting in the 1.12 data.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.trainer_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_i32(self.trainer_type as i32);
        writer.write_u32(self.trainer_guid.entry()); // TrainerID -- see above

        writer.write_i32(self.spells.len() as i32);
        for spell in &self.spells {
            writer.write_u32(spell.spell_id);
            writer.write_u32(spell.cost); // MoneyCost
            writer.write_u32(spell.req_skill); // ReqSkillLine
            writer.write_u32(spell.req_skill_value); // ReqSkillRank
                                                     // ReqAbility[3]: vanilla's two prerequisite spells plus its always-zero third slot,
                                                     // which is the same fixed-width triple 1.14 expects.
            writer.write_u32(spell.req_spell_1);
            writer.write_u32(spell.req_spell_2);
            writer.write_u32(spell.unknown);
            writer.write_u8(TrainerSpellState::from_vanilla(spell.state).to_modern());
            writer.write_u8(spell.req_level);
        }

        writer.write_bits(self.greeting.len() as u32, 11);
        writer.flush_bits();
        writer.write_string_raw(&self.greeting);

        Some(writer.finish(Opcode::SMSG_TRAINER_LIST))
    }

    fn to_vanilla(&self) -> WorldPacket {
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

/// No `to_modern`: 1.14 has no success counterpart to this message.
///
/// The opcode was removed outright -- the 1.14 opcode table has `CMSG_TRAINER_BUY_SPELL` and
/// `SMSG_TRAINER_BUY_FAILED` but nothing for the success case. A 1.14 client learns the spell from
/// the spell-learned update and repaints its trainer frame only when a fresh `SMSG_TRAINER_LIST`
/// arrives, so the caller that would send this must re-send the trainer list instead. Fabricating a
/// body here is impossible: there is no opcode to send it under.
impl ToWorldPacket for SmsgTrainerBuySucceeded {
    fn to_vanilla(&self) -> WorldPacket {
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
    /// The same three fields, with the GUID widened; only the reason is worth a note.
    ///
    /// Unlike the spell *state* in `SMSG_TRAINER_LIST`, this reason code is not known to have been
    /// renumbered between the two protocols, and no 1.14 enumeration of it was available to check
    /// against. It is therefore passed through unchanged. If a 1.14 client ever shows the wrong
    /// failure text here, this cast is the first thing to suspect.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.trainer_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.spell_id);
        writer.write_u32(self.error as u32); // TrainerFailedReason -- see above
        Some(writer.finish(Opcode::SMSG_TRAINER_BUY_FAILED))
    }

    fn to_vanilla(&self) -> WorldPacket {
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
    use crate::protocol::ObjectGuid;

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
        // Trainer GUID must be fixed 8 bytes (unpacked)
        let msg = SmsgTrainerList {
            trainer_guid: trainer_guid(),
            trainer_type: 0,
            spells: vec![],
            greeting: String::new(),
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        assert_eq!(
            read_u64_le(data, 0),
            0xF130_0000_00C6_0001,
            "Trainer GUID must be unpacked (fixed 8 bytes)"
        );
    }

    #[test]
    fn smsg_trainer_list_field_order() {
        // Full per-spell layout:
        // spell_id(u32) | state(u8) | cost(u32) | prof_avail(u32) | first_rank(u32)
        // | req_level(u8) | req_skill(u32) | req_skill_value(u32)
        // | req_spell_1(u32) | req_spell_2(u32) | unknown(u32)
        let msg = SmsgTrainerList {
            trainer_guid: trainer_guid(),
            trainer_type: 0,
            spells: vec![one_spell()],
            greeting: "Hi".to_string(),
        };
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        let mut pos = 0;
        // Header
        assert_eq!(read_u64_le(data, pos), trainer_guid().raw());
        pos += 8; // guid
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // trainer_type
        assert_eq!(read_u32_le(data, pos), 1);
        pos += 4; // spell count

        // Spell entry
        assert_eq!(read_u32_le(data, pos), 1142);
        pos += 4; // spell_id
        assert_eq!(data[pos], 0);
        pos += 1; // state (u8)
        assert_eq!(read_u32_le(data, pos), 100);
        pos += 4; // cost
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // primary_prof_first_rank_available
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // primary_prof_first_rank
        assert_eq!(data[pos], 4);
        pos += 1; // req_level (u8)
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // req_skill
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // req_skill_value
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // req_spell_1
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // req_spell_2
        assert_eq!(read_u32_le(data, pos), 0);
        pos += 4; // unknown

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
        let pkt = msg.to_vanilla();
        let data = pkt.data();

        // After GUID(8) + trainer_type(4) + count(4) = offset 16
        let (greeting, end) = read_cstring(data, 16);
        assert_eq!(greeting, "Hello trainer");
        // Null byte must be present (read_cstring consumes it, so end == data.len())
        assert_eq!(
            end,
            data.len(),
            "Greeting must be the last field, null-terminated"
        );
    }
}
