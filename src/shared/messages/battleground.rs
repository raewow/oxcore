//! Battleground system message structs
//!
//! This module contains type-safe message structures for all battleground-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgBattlefieldStatus`] - Status of a battleground
//! - [`SmsgBattlefieldList`] - List of available battleground instances

use crate::shared::game::battleground::{BattleGroundStatus, BattleGroundTypeId};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

/// SMSG_BATTLEFIELD_STATUS - Status of a battleground
///
/// Sent to players to notify them of the current status of a battleground.
#[derive(Debug, Clone)]
pub struct SmsgBattlefieldStatus {
    /// Battleground type ID
    pub bg_type_id: BattleGroundTypeId,
    /// Current status of the battleground
    pub status: BattleGroundStatus,
    /// Time related to the status (e.g., time until next battle)
    pub time1: u32,
    /// Additional time information
    pub time2: u32,
    /// Client instance ID
    pub client_instance_id: u32,
}

impl ToWorldPacket for SmsgBattlefieldStatus {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BATTLEFIELD_STATUS);
        packet.write_u32(self.bg_type_id.map_id());
        packet.write_u8(self.status as u8);
        packet.write_u32(self.time1);
        packet.write_u32(self.time2);
        packet.write_u8(0); // Arena type (0 for BG)
        packet.write_u8(0); // Unknown
        packet.write_u32(self.client_instance_id);
        packet
    }
}

/// SMSG_BATTLEFIELD_LIST - List of available battleground instances
///
/// Sent to players when they query the battleground queue.
#[derive(Debug)]
pub struct SmsgBattlefieldList<'a> {
    /// Battleground type ID
    pub bg_type_id: BattleGroundTypeId,
    /// Reference to array of available instance IDs
    pub instance_ids: &'a [u32],
}

impl ToWorldPacket for SmsgBattlefieldList<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_BATTLEFIELD_LIST);
        packet.write_u32(self.bg_type_id.map_id());
        packet.write_u8(self.instance_ids.len() as u8);
        for &instance_id in self.instance_ids {
            packet.write_u32(instance_id);
        }
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::game::battleground::{BattleGroundStatus, BattleGroundTypeId};
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_battlefield_status() {
        let msg = SmsgBattlefieldStatus {
            bg_type_id: BattleGroundTypeId::WarsongGulch,
            status: BattleGroundStatus::None,
            time1: 0,
            time2: 0,
            client_instance_id: 123,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_BATTLEFIELD_STATUS);
    }

    #[test]
    fn test_smsg_battlefield_list() {
        let msg = SmsgBattlefieldList {
            bg_type_id: BattleGroundTypeId::WarsongGulch,
            instance_ids: &[1, 2, 3],
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_BATTLEFIELD_LIST);
    }
}
