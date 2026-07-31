//! LFG system message structs
//!
//! This module contains type-safe message structures for all LFG-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgMeetingstoneSetqueue`] - Set meeting stone queue status

use crate::messages::ToWorldPacket;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// SMSG_MEETINGSTONE_SETQUEUE - Set meeting stone queue status
///
/// Sent to players when they interact with a meeting stone.
#[derive(Debug, Clone)]
pub struct SmsgMeetingstoneSetqueue {
    /// Queue status (0 = not in queue, 1 = in queue)
    pub in_queue: bool,
}

/// Not ported to 1.14: the meeting stone queue has no 1.14 counterpart.
///
/// The whole `SMSG_MEETINGSTONE_*` family was removed when meeting stones stopped being a queueing
/// system and became plain summoning stones. 1.14's group finder speaks a different set of opcodes
/// entirely (`SMSG_LFG_*`), built around dungeon lists, role checks and proposals — there is no
/// packet that means "you are now in a meeting stone queue" for this to map onto.
///
/// A 1.14 client will not miss it: it never asks to be put in this queue, so the message is only
/// ever sent in response to something a 1.12 client did.
impl ToWorldPacket for SmsgMeetingstoneSetqueue {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_MEETINGSTONE_SETQUEUE);
        packet.write_u8(if self.in_queue { 1 } else { 0 });
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_meetingstone_setqueue() {
        let msg = SmsgMeetingstoneSetqueue { in_queue: true };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_MEETINGSTONE_SETQUEUE);
    }
}
