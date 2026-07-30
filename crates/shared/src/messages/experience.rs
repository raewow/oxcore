//! Experience system message structs
//!
//! This module contains type-safe message structures for experience-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgLogXpGain`] - XP gain notification (kill or quest)
//! - [`SmsgLevelupInfo`] - Level up notification with stat gains

use crate::messages::update::DEFAULT_REALM_ID;
use crate::protocol::bitbuf::BitWriter;
use crate::messages::ToWorldPacket;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

/// SMSG_LOG_XPGAIN - XP gain notification
///
/// Sent when player gains experience from a kill or quest completion.
///
/// Packet structure:
/// - ObjectGuid victim (8 bytes) - creature GUID (empty for quest XP)
/// - uint32 total_xp - total XP gained
/// - uint8 xp_type - 0 = kill XP, 1 = quest XP
/// - If xp_type == 0 (kill XP):
///   - float group_bonus - group XP bonus multiplier (1.0 = no bonus)
#[derive(Debug, Clone)]
pub struct SmsgLogXpGain {
    /// Creature GUID for kill XP, empty GUID for quest XP
    pub victim_guid: ObjectGuid,
    /// Total XP gained (including rest bonus if applicable)
    pub total_xp: u32,
    /// XP type: 0 = kill, 1 = quest
    pub xp_type: u8,
    /// Group XP bonus multiplier (only used for kill XP)
    /// 1.0 = no bonus, >1.0 = group bonus
    pub group_bonus: f32,
}

impl ToWorldPacket for SmsgLogXpGain {
    /// `LogXPGain::Write` for build 42597.
    ///
    /// Gains an `Original` amount ahead of the reason and a recruit-a-friend bonus byte after the
    /// group multiplier. `Original` is the pre-bonus figure; vanilla sends only the total, so the two
    /// are the same here — the client shows the difference as a bonus, and claiming a bonus we did not
    /// compute would misreport it.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.victim_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_i32(self.total_xp as i32); // Original
        writer.write_u8(self.xp_type); // Reason
        writer.write_i32(self.total_xp as i32); // Amount
        writer.write_f32(self.group_bonus);
        writer.write_u8(0); // RAFBonus
        Some(writer.finish(Opcode::SMSG_LOG_XPGAIN))
    }


    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOG_XPGAIN);

        // Write victim GUID (full 8 bytes, not packed)
        packet.write_u64(self.victim_guid.raw());

        // Write total XP
        packet.write_u32(self.total_xp);

        // Write XP type (0 = kill, 1 = quest)
        packet.write_u8(self.xp_type);

        // If kill XP, write group bonus
        if self.xp_type == 0 {
            packet.write_f32(self.group_bonus);
        }

        packet
    }
}

/// SMSG_LEVELUP_INFO - Level up notification
///
/// Sent when player levels up. Contains new level and all stat/power gains.
///
/// Packet structure:
/// - uint32 level - new level
/// - uint32 hp_gain - max health increase
/// - uint32 mana_gain - max mana increase (0 for non-mana classes)
/// - uint32[4] power_gains - unused power type gains (rage, focus, energy, happiness)
/// - uint32[5] stat_gains - stat increases (STR, AGI, STA, INT, SPI)
#[derive(Debug, Clone)]
pub struct SmsgLevelupInfo {
    /// New player level
    pub level: u32,
    /// Health gain (max health increase)
    pub hp_gain: u32,
    /// Mana gain (max mana increase, 0 for non-mana classes)
    pub mana_gain: u32,
    /// Stat gains: [Strength, Agility, Stamina, Intellect, Spirit]
    pub stat_gains: [u32; 5],
}

impl ToWorldPacket for SmsgLevelupInfo {
    /// `LevelUpInfo::Write` for build 42597.
    ///
    /// The power array widens from vanilla's single mana delta to **seven** entries — one per power
    /// type — for build 1.14.1 and later (; ours is 1.14.2). Sending fewer
    /// leaves the client reading the stat deltas as powers. Two talent counts are appended, which
    /// Classic Era has no source for.
    fn to_modern(&self) -> Option<WorldPacket> {
        /// Power types the client reads a delta for, from `GetPowerCountForClientVersion`.
        const MODERN_POWER_COUNT: usize = 7;
        /// Mana is power type 0, which is the only one vanilla reports a level-up gain for.
        const MANA: usize = 0;

        let mut writer = BitWriter::new();
        writer.write_i32(self.level as i32);
        writer.write_i32(self.hp_gain as i32); // HealthDelta

        for index in 0..MODERN_POWER_COUNT {
            let delta = if index == MANA { self.mana_gain } else { 0 };
            writer.write_i32(delta as i32);
        }

        for stat in &self.stat_gains {
            writer.write_i32(*stat as i32);
        }

        writer.write_i32(0); // NumNewTalents
        writer.write_i32(0); // NumNewPvpTalentSlots
        Some(writer.finish(Opcode::SMSG_LEVELUP_INFO))
    }


    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LEVELUP_INFO);

        // Write new level
        packet.write_u32(self.level);

        // Write HP gain
        packet.write_u32(self.hp_gain);

        // Write mana gain
        packet.write_u32(self.mana_gain);

        // Write power gains (4 zeros for unused power types: Rage, Focus, Energy, Happiness)
        for _ in 0..4 {
            packet.write_u32(0);
        }

        // Write stat gains (Strength, Agility, Stamina, Intellect, Spirit)
        for &gain in &self.stat_gains {
            packet.write_u32(gain);
        }

        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smsg_log_xpgain_kill() {
        let msg = SmsgLogXpGain {
            victim_guid: ObjectGuid::from_raw(0x12345678_00000001),
            total_xp: 150,
            xp_type: 0, // kill
            group_bonus: 1.0,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOG_XPGAIN);
        // Size: 8 (guid) + 4 (xp) + 1 (type) + 4 (bonus) = 17 bytes
        assert_eq!(packet.size(), 17);
    }

    #[test]
    fn test_smsg_log_xpgain_quest() {
        let msg = SmsgLogXpGain {
            victim_guid: ObjectGuid::empty(),
            total_xp: 500,
            xp_type: 1,       // quest
            group_bonus: 1.0, // ignored for quest
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LOG_XPGAIN);
        // Size: 8 (guid) + 4 (xp) + 1 (type) = 13 bytes (no bonus for quest)
        assert_eq!(packet.size(), 13);
    }

    #[test]
    fn test_smsg_levelup_info() {
        let msg = SmsgLevelupInfo {
            level: 10,
            hp_gain: 50,
            mana_gain: 30,
            stat_gains: [2, 2, 3, 2, 2], // STR, AGI, STA, INT, SPI
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_LEVELUP_INFO);
        // Size: 4 (level) + 4 (hp) + 4 (mana) + 4*4 (powers) + 5*4 (stats) = 48 bytes
        assert_eq!(packet.size(), 48);
    }
}

#[cfg(test)]
mod modern_experience_tests {
    use super::*;

    /// 1.14 reads **seven** power deltas where vanilla sends one mana figure. Sending fewer makes the
    /// client read the stat deltas as powers, so the size is the thing worth pinning.
    #[test]
    fn level_up_carries_a_delta_for_every_power_type() {
        let packet = SmsgLevelupInfo {
            level: 2,
            hp_gain: 12,
            mana_gain: 34,
            stat_gains: [1, 2, 3, 4, 5],
        }
        .to_modern()
        .expect("level up must encode for modern");

        // level + health + 7 powers + 5 stats + 2 talent counts, all i32.
        assert_eq!(packet.size(), (1 + 1 + 7 + 5 + 2) * 4);
    }

    /// The mana gain must land in the mana slot (power type 0), not spill into another power.
    #[test]
    fn the_mana_gain_lands_in_the_mana_slot() {
        let packet = SmsgLevelupInfo {
            level: 2,
            hp_gain: 0,
            mana_gain: 34,
            stat_gains: [0; 5],
        }
        .to_modern()
        .unwrap();
        let bytes = packet.contents();

        // level(0..4), health(4..8), then power 0 at 8..12.
        assert_eq!(&bytes[8..12], &34i32.to_le_bytes());
        assert_eq!(&bytes[12..16], &0i32.to_le_bytes(), "power 1 untouched");
    }
}
