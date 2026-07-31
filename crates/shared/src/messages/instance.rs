//! Instance system message structs
//!
//! This module contains type-safe message structures for all instance-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgInstanceResetWarning`] - Warning before an instance is reset
//! - [`SmsgInstanceReset`] - Notification that an instance has been reset
//! - [`SmsgInstanceResetFailed`] - Notification that an instance reset failed

use crate::game::instance::{InstanceResetFailReason, InstanceResetWarningType};
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

/// SMSG_INSTANCE_RESET_WARNING - Warning before an instance is reset
///
/// Sent to players in the instance to warn them that the instance is about to reset.
#[derive(Debug, Clone)]
pub struct SmsgInstanceResetWarning {
    /// Map ID of the instance
    pub map_id: u32,
    /// Instance ID
    pub instance_id: u32,
    /// Warning type (unused in Vanilla)
    pub warning_type: InstanceResetWarningType,
    /// Time remaining in seconds until the instance resets
    pub time_remaining: u64,
}

impl ToWorldPacket for SmsgInstanceResetWarning {
    /// The 1.14 raid-info lock record, which is a different shape from vanilla's.
    ///
    /// Per lock, 1.14 reads: map id, difficulty id, a **64-bit** instance id, time remaining, a
    /// completed-encounter mask, then two bits. Vanilla writes a 32-bit instance id and a trailing
    /// "permanent" byte with no difficulty or encounter mask at all. The instance id is the trap:
    /// leaving it 32 bits shifts every field after it by four bytes, and the client reads the
    /// time remaining out of the high half of the id.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(1); // one lock in this message

        writer.write_u32(self.map_id);
        writer.write_u32(MODERN_DIFFICULTY_RAID40);
        writer.write_u64(self.instance_id as u64);
        writer.write_i32(self.time_remaining as i32);
        // Vanilla tracks no per-boss encounter state; 1 is the client's "nothing cleared" default.
        writer.write_u32(1); // CompletedMask

        // Vanilla's trailing permanent byte folds into these. Anything we warn about is a live
        // lock, and vanilla has no lockout extension.
        writer.write_bit(true); // Locked
        writer.write_bit(false); // Extended
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_RAID_INSTANCE_INFO))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_RAID_INSTANCE_INFO);
        packet.write_u32(1); // Number of instances (1)
        packet.write_u32(self.map_id);
        packet.write_u32(self.instance_id);
        packet.write_u32(self.time_remaining as u32); // Time left in seconds
        packet.write_u8(0); // Permanent flag (0 = temporary)
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
    use crate::game::instance::{InstanceResetFailReason, InstanceResetWarningType};
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_instance_reset_warning() {
        let msg = SmsgInstanceResetWarning {
            map_id: 100,
            instance_id: 1,
            warning_type: InstanceResetWarningType::Hours15Min,
            time_remaining: 900,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_RAID_INSTANCE_INFO);
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
