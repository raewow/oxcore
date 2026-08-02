//! Combat Messages - SMSG_ATTACKERSTATEUPDATE and related packets

use super::ToWorldPacket;
use crate::messages::update::DEFAULT_REALM_ID;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

/// Hit info flags for SMSG_ATTACKERSTATEUPDATE
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitInfo {
    NormalSwing = 0x00000000,
    Unk0 = 0x00000001,
    AffectsVictim = 0x00000002,
    OffHand = 0x00000004,
    Miss = 0x00000010,
    Absorb = 0x00000020,
    Resist = 0x00000040,
    CriticalHit = 0x00000080,
    Glancing = 0x00004000,
    Crushing = 0x00008000,
    NoAction = 0x00010000,
    SwingNoHitSound = 0x00080000,
}

/// Victim state for SMSG_ATTACKERSTATEUPDATE
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictimState {
    Intact = 0,
    Hit = 1,
    Dodge = 2,
    Parry = 3,
    Interrupt = 4,
    Block = 5,
    Evades = 6,
    Immune = 7,
    Deflects = 8,
}

/// Translate a vanilla `HitInfo` word to its 1.14 equivalent.
///
/// The two words agree only up to `Miss` (0x10); above it they diverge in a way that silently
/// corrupts the packet rather than just mislabelling it. Vanilla `FullResist` is 0x40, which 1.14
/// reads as `PartialAbsorb`; vanilla `CriticalHit` is 0x80, which 1.14 reads as `FullResist`; vanilla
/// `Block` is 0x800, which 1.14 reads as `Unk11`.
///
/// That matters more here than for movement flags, because the flags **gate which fields the body
/// contains** — absorbed is written only under an absorb flag, resisted only under a resist flag,
/// block amount only under `Block`. A mistranslated word makes the client read the wrong number of
/// bytes and desynchronises everything after this packet.
///
/// Translates by name, the way `to_modern_movement_flags` does. Tables from
/// `HitInfoVanilla` and `HitInfo`.
pub fn to_modern_hit_info(vanilla: u32) -> u32 {
    HIT_INFO_FLAGS
        .iter()
        .filter(|(from, _)| vanilla & from != 0)
        .fold(0, |acc, (_, to)| acc | to)
}

/// (vanilla bit, modern bit) for every hit-info flag whose name survived into 1.14.
///
/// `None` is deliberately absent: it is zero on both sides. Vanilla has no `Partial*` variants — it
/// signals a partial absorb with the same bit as a full one — so those modern bits are never set from
/// a vanilla source, which is why the body checks both members of each pair.
const HIT_INFO_FLAGS: [(u32, u32); 11] = [
    (0x0000_0001, 0x0000_0001), // Unk0
    (0x0000_0002, 0x0000_0002), // AffectsVictim
    (0x0000_0004, 0x0000_0004), // OffHand
    (0x0000_0010, 0x0000_0010), // Miss
    (0x0000_0020, 0x0000_0020), // FullAbsorb
    (0x0000_0040, 0x0000_0080), // FullResist -- 0x40 in vanilla, 0x80 in 1.14
    (0x0000_0080, 0x0000_0200), // CriticalHit -- 0x80 in vanilla, 0x200 in 1.14
    (0x0000_0800, 0x0000_2000), // Block -- 0x800 in vanilla, 0x2000 in 1.14
    (0x0000_4000, 0x0001_0000), // Glancing
    (0x0000_8000, 0x0002_0000), // Crushing
    (0x0001_0000, 0x0004_0000), // NoAnimation
];

/// 1.14 `HitInfo` bits the body's conditional fields are keyed on.
mod modern_hit_info {
    pub const FULL_ABSORB: u32 = 0x0000_0020;
    pub const PARTIAL_ABSORB: u32 = 0x0000_0040;
    pub const FULL_RESIST: u32 = 0x0000_0080;
    pub const PARTIAL_RESIST: u32 = 0x0000_0100;
    pub const BLOCK: u32 = 0x0000_2000;
    pub const RAGE_GAIN: u32 = 0x0080_0000;
    /// Gates the twelve-float debug block. Vanilla never sets it, so we never write that block.
    pub const UNK0: u32 = 0x0000_0001;
    pub const UNK12: u32 = 0x0000_1000;
}

/// SMSG_ATTACKERSTATEUPDATE - Main combat result packet
#[derive(Debug, Clone)]
pub struct SmsgAttackerStateUpdate {
    pub hit_info: u32,
    pub attacker_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub total_damage: u32,
    pub damage_school: u32,
    pub absorbed: u32,
    pub resisted: i32,
    pub victim_state: u32,
    pub blocked: u32,
}

impl Default for SmsgAttackerStateUpdate {
    fn default() -> Self {
        Self {
            hit_info: HitInfo::NormalSwing as u32,
            attacker_guid: ObjectGuid::empty(),
            target_guid: ObjectGuid::empty(),
            total_damage: 0,
            damage_school: 0,
            absorbed: 0,
            resisted: 0,
            victim_state: VictimState::Hit as u32,
            blocked: 0,
        }
    }
}

impl ToWorldPacket for SmsgAttackerStateUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ATTACKERSTATEUPDATE);

        packet.write_u32(self.hit_info);
        packet.write_packed_guid_raw(self.attacker_guid.raw());
        packet.write_packed_guid_raw(self.target_guid.raw());
        packet.write_u32(self.total_damage);
        // Sub-damage entries (1 for melee)
        packet.write_u8(1); // subDamageCount
        packet.write_u32(self.damage_school); // school mask
        packet.write_f32(self.total_damage as f32); // damage as float
        packet.write_u32(self.total_damage); // damage as u32
        packet.write_u32(self.absorbed);
        packet.write_i32(self.resisted);
        // Post sub-damage fields
        packet.write_u32(self.victim_state);
        packet.write_u32(0); // unk1
        packet.write_u32(0); // spellId (0 for melee)
        packet.write_u32(self.blocked);

        packet
    }

    /// `AttackerStateUpdate::Write`, per the 1.14 wire format.
    ///
    /// Structurally unlike vanilla: the whole payload is built into a sub-buffer, and the packet is
    /// a `HasLogData` bit, a flush, then the sub-buffer's **length** followed by its bytes. The
    /// client uses that length to skip the round info it does not need, so it has to be exact.
    ///
    /// The flags are translated by name first — see [`to_modern_hit_info`] — because they gate which
    /// of the conditional fields are present.
    ///
    /// **Note:** the trailing `ContentTuning` block is written *inline* here as
    /// `(u8 type, u8 targetLevel, u8 expansion, i16 levelDelta, f32 playerItemLevel, f32
    /// targetItemLevel)`. The inline form is used here because this message is on a path the fork
    /// exercises against a real 1.14 client.
    fn to_modern(&self) -> Option<WorldPacket> {
        let flags = to_modern_hit_info(self.hit_info);

        // The round info goes into its own buffer so its length can be written ahead of it.
        let mut round = BitWriter::new();
        round.write_u32(flags);
        let (high, low) = self.attacker_guid.to_guid128(DEFAULT_REALM_ID);
        round.write_packed_guid_128(high, low);
        let (high, low) = self.target_guid.to_guid128(DEFAULT_REALM_ID);
        round.write_packed_guid_128(high, low);
        round.write_i32(self.total_damage as i32); // Damage
        round.write_i32(self.total_damage as i32); // OriginalDamage -- vanilla sends only one
                                                   // -1 means "the victim survived". Vanilla has no overkill field, and claiming zero overkill
                                                   // is not the same statement.
        round.write_i32(-1); // OverDamage

        // One sub-damage entry, matching the vanilla body above.
        round.write_u8(1);
        round.write_u32(self.damage_school);
        round.write_f32(self.total_damage as f32);
        round.write_i32(self.total_damage as i32);
        // Both members of each pair are checked: vanilla signals a partial absorb with the same bit
        // as a full one, so only the `Full*` modern bit is ever set from a vanilla source -- but a
        // future caller setting the partial bit directly must not silently drop the field.
        if flags & (modern_hit_info::FULL_ABSORB | modern_hit_info::PARTIAL_ABSORB) != 0 {
            round.write_i32(self.absorbed as i32);
        }
        if flags & (modern_hit_info::FULL_RESIST | modern_hit_info::PARTIAL_RESIST) != 0 {
            round.write_i32(self.resisted);
        }

        round.write_u8(self.victim_state as u8); // widened to u32 in vanilla
        round.write_i32(0); // AttackerState
        round.write_u32(0); // MeleeSpellID -- 0 for a melee swing

        if flags & modern_hit_info::BLOCK != 0 {
            round.write_i32(self.blocked as i32);
        }
        if flags & modern_hit_info::RAGE_GAIN != 0 {
            round.write_i32(0); // RageGained -- vanilla carries no rage in this packet
        }
        // The twelve-float debug block hangs off Unk0, which no vanilla source sets.
        debug_assert_eq!(
            flags & modern_hit_info::UNK0,
            self.hit_info & 0x1,
            "Unk0 must survive translation unchanged, since it gates a 48-byte block"
        );
        if flags & modern_hit_info::UNK0 != 0 {
            for _ in 0..11 {
                round.write_u32(0);
            }
            round.write_u32(0); // SinceLastSwing
        }
        if flags & (modern_hit_info::BLOCK | modern_hit_info::UNK12) != 0 {
            round.write_f32(0.0); // BlockRoll
        }

        // ContentTuning, in the inline order noted above. All zero: Classic Era has no level or
        // item-level scaling for the client to apply.
        round.write_u8(0); // TuningType
        round.write_u8(0); // TargetLevel
        round.write_u8(0); // Expansion
        round.write_i16(0); // PlayerLevelDelta
        round.write_f32(0.0); // PlayerItemLevel
        round.write_f32(0.0); // TargetItemLevel

        let round = round.into_bytes();

        let mut writer = BitWriter::new();
        writer.write_bit(false); // HasLogData
        writer.flush_bits();
        writer.write_u32(round.len() as u32);
        writer.write_bytes(&round);

        Some(writer.finish(Opcode::SMSG_ATTACKERSTATEUPDATE))
    }
}

/// SMSG_ATTACKSTART - Notifies that an attack has started
#[derive(Debug, Clone)]
pub struct SmsgAttackStart {
    pub attacker_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgAttackStart {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ATTACKSTART);
        // Full 8-byte GUIDs for SMSG_ATTACKSTART (not packed)
        packet.write_guid_raw(self.attacker_guid.raw());
        packet.write_guid_raw(self.target_guid.raw());
        packet
    }

    /// `SAttackStart::Write`, per the 1.14 wire format.
    ///
    /// Same two GUIDs, packed as guid128 instead of written raw.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.attacker_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        let (high, low) = self.target_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        Some(writer.finish(Opcode::SMSG_ATTACKSTART))
    }
}

/// SMSG_ATTACKSTOP - Notifies that an attack has stopped
#[derive(Debug, Clone)]
pub struct SmsgAttackStop {
    pub attacker_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub unk: u32, // Usually 0
}

impl ToWorldPacket for SmsgAttackStop {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ATTACKSTOP);
        packet.write_packed_guid_raw(self.attacker_guid.raw());
        packet.write_packed_guid_raw(self.target_guid.raw());
        packet.write_u32(self.unk);
        packet
    }

    /// `SAttackStop::Write`, per the 1.14 wire format.
    ///
    /// Vanilla's trailing u32 becomes a single `NowDead` bit. It is nonzero in vanilla exactly when
    /// the target died, so the flag carries the same meaning.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.attacker_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        let (high, low) = self.target_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_bit(self.unk != 0); // NowDead
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_ATTACKSTOP))
    }
}

/// SMSG_SPELLDAMAGELOG - Damage from thorns, etc.
#[derive(Debug, Clone)]
pub struct SmsgSpellDamageLog {
    pub victim_guid: ObjectGuid,
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub damage: u32,
    pub school_mask: u32,
}

impl ToWorldPacket for SmsgSpellDamageLog {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELLDAMAGELOG);
        packet.write_packed_guid_raw(self.victim_guid.raw());
        packet.write_packed_guid_raw(self.caster_guid.raw());
        packet.write_u32(self.spell_id);
        packet.write_u32(self.damage);
        packet.write_u32(self.school_mask);
        packet
    }
}

/// Helper to convert outcome to hit info
pub fn outcome_to_hit_info(
    miss: bool,
    dodge: bool,
    parry: bool,
    block: bool,
    glancing: bool,
    crit: bool,
    crushing: bool,
) -> u32 {
    if miss {
        return HitInfo::Miss as u32;
    }
    // Dodge/Parry: no AFFECTS_VICTIM (no hit animation on victim)
    if dodge {
        return HitInfo::NormalSwing as u32; // victim state handles dodge
    }
    if parry {
        return HitInfo::NormalSwing as u32; // victim state handles parry
    }
    // All damage-dealing outcomes include AFFECTS_VICTIM
    let affects = HitInfo::AffectsVictim as u32;
    if block {
        return affects;
    }
    if glancing {
        return affects | HitInfo::Glancing as u32;
    }
    if crit {
        return affects | HitInfo::CriticalHit as u32;
    }
    if crushing {
        return affects | HitInfo::Crushing as u32;
    }
    affects // Normal hit
}

/// Helper to convert outcome to victim state
pub fn outcome_to_victim_state(miss: bool, dodge: bool, parry: bool, block: bool) -> u8 {
    if miss {
        return VictimState::Intact as u8;
    }
    if dodge {
        return VictimState::Dodge as u8;
    }
    if parry {
        return VictimState::Parry as u8;
    }
    if block {
        return VictimState::Block as u8;
    }
    VictimState::Hit as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_info_flags() {
        assert_eq!(HitInfo::Miss as u32, 0x00000010);
        assert_eq!(HitInfo::AffectsVictim as u32, 0x00000002);
        assert_eq!(HitInfo::CriticalHit as u32, 0x00000080);
    }

    #[test]
    fn test_outcome_to_hit_info() {
        let hit_info = outcome_to_hit_info(false, false, false, false, false, true, false);
        assert_eq!(
            hit_info,
            HitInfo::AffectsVictim as u32 | HitInfo::CriticalHit as u32
        );
        // Normal hit should only have AFFECTS_VICTIM
        let normal = outcome_to_hit_info(false, false, false, false, false, false, false);
        assert_eq!(normal, HitInfo::AffectsVictim as u32);
    }
}

#[cfg(test)]
mod modern_combat_tests {
    use super::*;

    fn swing(hit_info: u32, absorbed: u32, resisted: i32, blocked: u32) -> WorldPacket {
        SmsgAttackerStateUpdate {
            hit_info,
            attacker_guid: ObjectGuid::new_player(4),
            target_guid: ObjectGuid::new_creature(299, 464),
            total_damage: 12,
            damage_school: 1,
            absorbed,
            resisted,
            victim_state: VictimState::Hit as u32,
            blocked,
        }
        .to_modern()
        .expect("a melee swing must encode for modern")
    }

    /// The flags diverge above `Miss`, and a value-preserving copy turns a crit into a resist — which
    /// also changes which fields the body contains, so the client mis-reads everything after it.
    #[test]
    fn hit_info_flags_are_translated_by_name_not_by_value() {
        // vanilla CriticalHit is 0x80; 1.14 reads 0x80 as FullResist and puts CriticalHit at 0x200.
        assert_eq!(to_modern_hit_info(HitInfo::CriticalHit as u32), 0x200);
        // vanilla Resist is 0x40; 1.14 reads 0x40 as PartialAbsorb and puts FullResist at 0x80.
        assert_eq!(to_modern_hit_info(HitInfo::Resist as u32), 0x080);
        // Everything up to Miss is identical, which is why a value copy looks plausible at first.
        assert_eq!(to_modern_hit_info(HitInfo::Miss as u32), 0x010);
        assert_eq!(to_modern_hit_info(HitInfo::OffHand as u32), 0x004);
    }

    /// A crit must not gain a resist field, which is what a value-copied flag word would cause.
    #[test]
    fn a_crit_carries_no_resist_field() {
        let crit = swing(HitInfo::CriticalHit as u32, 0, 0, 0);
        let plain = swing(0, 0, 0, 0);
        assert_eq!(
            crit.size(),
            plain.size(),
            "CriticalHit gates no conditional field"
        );
    }

    /// Each conditional field must appear only under its own flag, and add exactly four bytes.
    #[test]
    fn conditional_fields_follow_their_flags() {
        let plain = swing(0, 0, 0, 0).size();
        let absorbed = swing(HitInfo::Absorb as u32, 5, 0, 0).size();
        let resisted = swing(HitInfo::Resist as u32, 0, 5, 0).size();

        assert_eq!(absorbed, plain + 4, "absorb adds one i32");
        assert_eq!(resisted, plain + 4, "resist adds one i32");
    }

    /// The body is length-prefixed, and the client uses that length to skip the round info. If it
    /// disagrees with the payload the client loses the rest of the stream.
    #[test]
    fn the_round_info_length_matches_the_payload() {
        let packet = swing(0, 0, 0, 0);
        let bytes = packet.contents();

        // HasLogData bit flushed to one byte, then the u32 length, then the payload.
        let declared = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
        assert_eq!(declared, bytes.len() - 5, "declared length must be exact");
    }
}
