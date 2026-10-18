//! Instance system message structs
//!
//! This module contains type-safe message structures for all instance-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgRaidInstanceInfo`] - The player's permanent raid saves
//! - [`SmsgRaidInstanceMessage`] - Raid welcome and reset warnings
//! - [`SmsgInstanceSaveCreated`] - A permanent raid save was just created
//! - [`SmsgRaidGroupOnly`] - Raid-group-required notice / removal countdown
//! - [`SmsgAreaTriggerMessage`] - Why an area trigger refused to let the player through
//! - [`SmsgUpdateInstanceOwnership`] / [`SmsgUpdateLastInstance`] - Saved raids, after a transfer
//! - [`SmsgInstanceReset`] - Notification that an instance has been reset
//! - [`SmsgInstanceResetFailed`] - Notification that an instance reset failed

use crate::game::instance::InstanceResetFailReason;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// Difficulty id the 1.14 client expects on a classic-era raid lock.
///
/// The 1.14 lock record carries a difficulty id that vanilla has no field for — per-difficulty
/// saves only arrived with heroic dungeons. Every raid lock a vanilla server can produce is 40-man
/// normal, which is difficulty 9. This is not a lookup we are inventing: 0 is the client's `None`
/// difficulty, and a lock tagged with it is dropped from the raid info window rather than shown.
const MODERN_DIFFICULTY_RAID40: u32 = 9;

/// One permanent raid save in the raid information window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidLock {
    pub map_id: u32,
    pub instance_id: u32,
    /// Seconds until the save resets.
    pub time_left: u32,
}

/// SMSG_RAID_INSTANCE_INFO - the player's permanent raid saves.
#[derive(Debug, Clone, Default)]
pub struct SmsgRaidInstanceInfo {
    pub locks: Vec<RaidLock>,
}

impl ToWorldPacket for SmsgRaidInstanceInfo {
    /// The 1.14 lock record is a different shape from vanilla's.
    ///
    /// Per lock, 1.14 reads: map id, difficulty id, a **64-bit** instance id, time remaining, a
    /// completed-encounter mask, then two bits flushed per lock. Vanilla writes map id, time left
    /// and a 32-bit instance id. Leaving the instance id 32 bits shifts every field after it by four
    /// bytes, and the client reads the time remaining out of the high half of the id.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(self.locks.len() as i32);
        for lock in &self.locks {
            writer.write_u32(lock.map_id);
            writer.write_u32(MODERN_DIFFICULTY_RAID40);
            writer.write_u64(u64::from(lock.instance_id));
            writer.write_i32(lock.time_left as i32);
            // Per-boss progress is not tracked; 1 is the client's "nothing cleared" default.
            writer.write_u32(1); // CompletedMask
            writer.write_bit(true); // Locked
            writer.write_bit(false); // Extended: vanilla has no lockout extension
            writer.flush_bits();
        }
        Some(writer.finish(Opcode::SMSG_RAID_INSTANCE_INFO))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_RAID_INSTANCE_INFO);
        packet.write_u32(self.locks.len() as u32);
        for lock in &self.locks {
            packet.write_u32(lock.map_id);
            packet.write_u32(lock.time_left);
            packet.write_u32(lock.instance_id);
        }
        packet
    }
}

/// Kind of raid reset notice, chosen from the time left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RaidInstanceMessageType {
    /// More than an hour left: shown on entering the raid.
    Welcome = 4,
    /// Up to an hour left.
    WarningHours = 1,
    /// Up to 15 minutes left.
    WarningMin = 2,
    /// Up to 5 minutes left.
    WarningMinSoon = 3,
}

impl RaidInstanceMessageType {
    pub fn for_time_left(seconds: u64) -> Self {
        match seconds {
            s if s > 3600 => Self::Welcome,
            s if s > 900 => Self::WarningHours,
            s if s > 300 => Self::WarningMin,
            _ => Self::WarningMinSoon,
        }
    }
}

/// SMSG_RAID_INSTANCE_MESSAGE - raid welcome / reset warning.
#[derive(Debug, Clone)]
pub struct SmsgRaidInstanceMessage {
    pub kind: RaidInstanceMessageType,
    pub map_id: u32,
    /// Seconds until the raid resets.
    pub time_left: u32,
}

impl ToWorldPacket for SmsgRaidInstanceMessage {
    /// 1.14 shrinks the type to a byte, adds a difficulty id and the locked/extended bits, and
    /// drops the time left: the client computes it from its own copy of the reset schedule.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(self.kind as u8);
        writer.write_u32(self.map_id);
        writer.write_u32(MODERN_DIFFICULTY_RAID40);
        writer.write_bit(true); // Locked
        writer.write_bit(false); // Extended
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_RAID_INSTANCE_MESSAGE))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_RAID_INSTANCE_MESSAGE);
        packet.write_u32(self.kind as u32);
        packet.write_u32(self.map_id);
        packet.write_u32(self.time_left);
        packet
    }
}

/// SMSG_INSTANCE_SAVE_CREATED - the player was just permanently saved to a raid.
#[derive(Debug, Clone, Copy)]
pub struct SmsgInstanceSaveCreated;

impl ToWorldPacket for SmsgInstanceSaveCreated {
    /// 1.14 replaces vanilla's u32 with a single flushed "gm" bit.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bit(false); // Gm
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_INSTANCE_SAVE_CREATED))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INSTANCE_SAVE_CREATED);
        packet.write_u32(0);
        packet
    }
}

/// Why a player is told they need a raid group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidGroupOnlyReason {
    /// "You must be in a raid group to enter this instance."
    RaidGroupRequired,
}

/// SMSG_RAID_GROUP_ONLY - the raid-group-required notice. With a non-zero
/// `timer_ms` the client shows a countdown before the player is sent home.
#[derive(Debug, Clone, Copy)]
pub struct SmsgRaidGroupOnly {
    pub timer_ms: u32,
    pub reason: RaidGroupOnlyReason,
}

impl ToWorldPacket for SmsgRaidGroupOnly {
    /// Same body as vanilla, but 1.14 renumbers the reason: "raid group
    /// required" is 2 there (1 means "too low level"), so map it by name.
    fn to_modern(&self) -> Option<WorldPacket> {
        let reason: u32 = match self.reason {
            RaidGroupOnlyReason::RaidGroupRequired => 2,
        };
        let mut packet = WorldPacket::new(Opcode::SMSG_RAID_GROUP_ONLY);
        packet.write_u32(self.timer_ms);
        packet.write_u32(reason);
        Some(packet)
    }

    fn to_vanilla(&self) -> WorldPacket {
        let reason: u32 = match self.reason {
            RaidGroupOnlyReason::RaidGroupRequired => 1,
        };
        let mut packet = WorldPacket::new(Opcode::SMSG_RAID_GROUP_ONLY);
        packet.write_u32(self.timer_ms);
        packet.write_u32(reason);
        packet
    }
}

/// SMSG_AREA_TRIGGER_MESSAGE - an area trigger refused the player (level,
/// key or attunement missing, ghost form).
///
/// Vanilla shows `text` on screen. 1.14 carries only the trigger id and the
/// client words the refusal itself.
#[derive(Debug, Clone)]
pub struct SmsgAreaTriggerMessage {
    pub trigger_id: u32,
    pub text: String,
}

impl ToWorldPacket for SmsgAreaTriggerMessage {
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_AREA_TRIGGER_MESSAGE);
        packet.write_u32(self.trigger_id);
        Some(packet)
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AREA_TRIGGER_MESSAGE);
        // Length counts the terminating NUL.
        packet.write_u32(self.text.len() as u32 + 1);
        packet.write_cstring(&self.text);
        packet
    }
}

/// SMSG_UPDATE_INSTANCE_OWNERSHIP - whether the player holds any permanent
/// raid save; the client enables its raid information button on it.
#[derive(Debug, Clone)]
pub struct SmsgUpdateInstanceOwnership {
    pub has_been_saved: bool,
}

impl ToWorldPacket for SmsgUpdateInstanceOwnership {
    /// Identical in 1.14: one u32.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_INSTANCE_OWNERSHIP);
        packet.write_u32(u32::from(self.has_been_saved));
        packet
    }
}

/// SMSG_UPDATE_LAST_INSTANCE - one of the player's saved raid maps. Vanilla
/// only.
#[derive(Debug, Clone)]
pub struct SmsgUpdateLastInstance {
    pub map_id: u32,
}

impl ToWorldPacket for SmsgUpdateLastInstance {
    fn to_modern(&self) -> Option<WorldPacket> {
        None
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_LAST_INSTANCE);
        packet.write_u32(self.map_id);
        packet
    }
}

/// SMSG_INSTANCE_RESET - Notification that an instance has been reset
///
/// Sent to players in the instance to confirm that the instance has been reset.
#[derive(Debug, Clone)]
pub struct SmsgInstanceReset {
    /// Map ID of the reset instance
    pub map_id: u32,
}

impl ToWorldPacket for SmsgInstanceReset {
    /// Identical to vanilla in 1.14: one u32 map id.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INSTANCE_RESET);
        packet.write_u32(self.map_id);
        packet
    }
}

/// SMSG_INSTANCE_RESET_FAILED - Notification that an instance reset failed
///
/// Sent to the player who requested the instance reset if the operation fails.
#[derive(Debug, Clone)]
pub struct SmsgInstanceResetFailed {
    /// Reason for the reset failure
    pub reason: InstanceResetFailReason,
    /// Map ID of the instance
    pub map_id: u32,
}

impl ToWorldPacket for SmsgInstanceResetFailed {
    /// 1.14 writes the map id **first** and then the reason as 2 bits — the reverse of vanilla's
    /// two u32s.
    ///
    /// Sending vanilla's order makes the client name the reason code as the map and then take the
    /// reason from the low two bits of the real map id, so a failed Molten Core reset reports some
    /// unrelated zone with an arbitrary excuse.
    fn to_modern(&self) -> Option<WorldPacket> {
        let reason = modern_reset_fail_reason(self.reason)?;
        let mut writer = BitWriter::new();
        writer.write_u32(self.map_id);
        writer.write_bits(reason, 2);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_INSTANCE_RESET_FAILED))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INSTANCE_RESET_FAILED);
        packet.write_u32(self.reason as u32);
        packet.write_u32(self.map_id);
        packet
    }
}

/// Translate a reset failure reason to its 1.14 code, or `None` if 1.14 cannot express it.
///
/// **This enum is renumbered.** 1.14 orders the reasons `Failed = 0, Zoning = 1, Offline = 2`;
/// vanilla orders them `General = 0, Offline = 1, Zoning = 2`. Offline and Zoning are swapped, so
/// casting the vanilla discriminant tells a player their party is zoning into an instance when in
/// fact someone is offline, and vice versa — a message that sends them looking for the wrong
/// problem.
///
/// Matched by name with no catch-all arm on purpose: a new vanilla variant must fail to compile
/// here rather than inherit whatever number it happened to be given.
fn modern_reset_fail_reason(reason: InstanceResetFailReason) -> Option<u32> {
    match reason {
        InstanceResetFailReason::General => Some(0),
        InstanceResetFailReason::Zoning => Some(1),
        InstanceResetFailReason::Offline => Some(2),
        // 1.14 reads exactly 2 bits and has no "fail without telling the player" code, so there is
        // no value that means silence. Dropping the packet is what silence looks like on the wire.
        InstanceResetFailReason::Silently => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::instance::InstanceResetFailReason;
    use crate::protocol::Opcode;

    #[test]
    fn raid_instance_info_lists_each_lock() {
        let msg = SmsgRaidInstanceInfo {
            locks: vec![RaidLock {
                map_id: 409,
                instance_id: 7,
                time_left: 900,
            }],
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_RAID_INSTANCE_INFO);
        assert_eq!(
            packet.contents(),
            &[1, 0, 0, 0, 0x99, 0x01, 0, 0, 0x84, 0x03, 0, 0, 7, 0, 0, 0]
        );
    }

    #[test]
    fn raid_group_only_renumbers_its_reason_for_modern_clients() {
        let msg = SmsgRaidGroupOnly {
            timer_ms: 60_000,
            reason: RaidGroupOnlyReason::RaidGroupRequired,
        };
        assert_eq!(
            msg.to_vanilla().contents(),
            &[0x60, 0xEA, 0, 0, 1, 0, 0, 0]
        );
        let modern = msg.to_modern().unwrap();
        assert_eq!(modern.opcode().modern(), 0x27A7);
        assert_eq!(modern.contents(), &[0x60, 0xEA, 0, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn area_trigger_message_carries_text_for_vanilla_and_the_trigger_for_modern() {
        let msg = SmsgAreaTriggerMessage {
            trigger_id: 78,
            text: "Hi".to_string(),
        };
        let vanilla = msg.to_vanilla();
        assert_eq!(vanilla.opcode().vanilla(), 0x02B8);
        assert_eq!(vanilla.contents(), &[3, 0, 0, 0, b'H', b'i', 0]);
        let modern = msg.to_modern().unwrap();
        assert_eq!(modern.opcode().modern(), 0x288F);
        assert_eq!(modern.contents(), &[78, 0, 0, 0]);
    }

    #[test]
    fn raid_message_type_follows_time_left() {
        assert_eq!(
            RaidInstanceMessageType::for_time_left(7200),
            RaidInstanceMessageType::Welcome
        );
        assert_eq!(
            RaidInstanceMessageType::for_time_left(3600),
            RaidInstanceMessageType::WarningHours
        );
        assert_eq!(
            RaidInstanceMessageType::for_time_left(900),
            RaidInstanceMessageType::WarningMin
        );
        assert_eq!(
            RaidInstanceMessageType::for_time_left(60),
            RaidInstanceMessageType::WarningMinSoon
        );
    }

    #[test]
    fn test_smsg_instance_reset() {
        let msg = SmsgInstanceReset { map_id: 100 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_INSTANCE_RESET);
    }

    #[test]
    fn test_smsg_instance_reset_failed() {
        let msg = SmsgInstanceResetFailed {
            reason: InstanceResetFailReason::General,
            map_id: 100,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_INSTANCE_RESET_FAILED);
    }
}
