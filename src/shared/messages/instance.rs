//! Instance system message structs
//!
//! This module contains type-safe message structures for all instance-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgInstanceResetWarning`] - Warning before an instance is reset
//! - [`SmsgInstanceReset`] - Notification that an instance has been reset
//! - [`SmsgInstanceResetFailed`] - Notification that an instance reset failed

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::world::game::instance::{InstanceResetFailReason, InstanceResetWarningType};

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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INSTANCE_RESET_FAILED);
        packet.write_u32(self.reason as u32);
        packet.write_u32(self.map_id);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Opcode;
    use crate::world::game::instance::{InstanceResetFailReason, InstanceResetWarningType};

    #[test]
    fn test_smsg_instance_reset_warning() {
        let msg = SmsgInstanceResetWarning {
            map_id: 100,
            instance_id: 1,
            warning_type: InstanceResetWarningType::Hours15Min,
            time_remaining: 900,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_RAID_INSTANCE_INFO);
    }

    #[test]
    fn test_smsg_instance_reset() {
        let msg = SmsgInstanceReset { map_id: 100 };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_INSTANCE_RESET);
    }

    #[test]
    fn test_smsg_instance_reset_failed() {
        let msg = SmsgInstanceResetFailed {
            reason: InstanceResetFailReason::General,
            map_id: 100,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_INSTANCE_RESET_FAILED);
    }
}
