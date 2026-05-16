//! Combat Messages - SMSG_ATTACKERSTATEUPDATE and related packets

use super::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};

/// Hit info flags for SMSG_ATTACKERSTATEUPDATE
/// Values from MaNGOS UnitDefines.h (1.12.1 client)
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

/// SMSG_ATTACKERSTATEUPDATE - Main combat result packet
/// Matches MaNGOS Unit::SendAttackStateUpdate() (Unit.cpp:4567-4603)
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
    fn to_world_packet(&self) -> WorldPacket {
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
}

/// SMSG_ATTACKSTART - Notifies that an attack has started
#[derive(Debug, Clone)]
pub struct SmsgAttackStart {
    pub attacker_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgAttackStart {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ATTACKSTART);
        // MaNGOS uses full 8-byte GUIDs for SMSG_ATTACKSTART (not packed)
        packet.write_guid_raw(self.attacker_guid.raw());
        packet.write_guid_raw(self.target_guid.raw());
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ATTACKSTOP);
        packet.write_packed_guid_raw(self.attacker_guid.raw());
        packet.write_packed_guid_raw(self.target_guid.raw());
        packet.write_u32(self.unk);
        packet
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
    fn to_world_packet(&self) -> WorldPacket {
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
