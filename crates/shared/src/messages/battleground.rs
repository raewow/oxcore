//! Battleground system message structs
//!
//! This module contains type-safe message structures for all battleground-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgBattlefieldStatus`] - Status of a battleground
//! - [`SmsgBattlefieldList`] - List of available battleground instances

use crate::game::battleground::{BattleGroundStatus, BattleGroundTypeId};
use crate::messages::ToWorldPacket;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

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

/// Not ported to 1.14: the message no longer exists as one packet, and this struct is missing the
/// data all of its replacements need.
///
/// 1.14 splits the single status packet into five opcodes chosen by status — none, queued, needs
/// confirmation, active and failed — and every one of them opens with a "ride ticket": the queuing
/// player's GUID, the queue slot id and the time they joined the queue. This struct carries none of
/// those three, and the queue slot in particular is not derivable here — the client echoes it back
/// when the player accepts or leaves, so a wrong one desynchronises the queue rather than merely
/// mis-displaying it.
///
/// The header also wants a `BattlemasterList` id, which 1.14 uses in place of a map id. Vanilla has
/// no source for that mapping.
///
/// Separately, [`BattleGroundStatus`] is renumbered against the wire values 1.14 expects: 1.14 uses
/// `None = 0, WaitQueue = 1, WaitJoin = 2, InProgress = 3, WaitLeave = 4`, while this enum omits
/// `WaitQueue` entirely and numbers `WaitJoin = 1, InProgress = 2, WaitLeave = 3`. Any future port
/// must dispatch on the variant name, never on the discriminant.
impl ToWorldPacket for SmsgBattlefieldStatus {
    fn to_vanilla(&self) -> WorldPacket {
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

/// Not ported to 1.14: the modern body opens with the battlemaster's GUID, which this struct does
/// not carry.
///
/// 1.14 reads, in order, a packed GUID for the battlemaster NPC, a verification word, the
/// `BattlemasterList` id, a level bracket as two bytes, then the instance list and two trailing
/// bits. Only the instance list survives from vanilla unchanged.
///
/// The GUID is load-bearing rather than cosmetic: the client sends it straight back in the join
/// request, so an empty one would render a battleground list the player cannot queue from. The
/// battleground type id here does line up with the `BattlemasterList` ids (1 = Alterac Valley,
/// 2 = Warsong Gulch, 3 = Arathi Basin), so that part is available — only the GUID and the level
/// bracket are missing.
impl ToWorldPacket for SmsgBattlefieldList<'_> {
    fn to_vanilla(&self) -> WorldPacket {
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
    use crate::game::battleground::{BattleGroundStatus, BattleGroundTypeId};
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_battlefield_status() {
        let msg = SmsgBattlefieldStatus {
            bg_type_id: BattleGroundTypeId::WarsongGulch,
            status: BattleGroundStatus::None,
            time1: 0,
            time2: 0,
            client_instance_id: 123,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_BATTLEFIELD_STATUS);
    }

    #[test]
    fn test_smsg_battlefield_list() {
        let msg = SmsgBattlefieldList {
            bg_type_id: BattleGroundTypeId::WarsongGulch,
            instance_ids: &[1, 2, 3],
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_BATTLEFIELD_LIST);
    }
}
