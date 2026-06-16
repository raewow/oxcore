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

impl ToWorldPacket for SmsgMeetingstoneSetqueue {
    fn to_world_packet(&self) -> WorldPacket {
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
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_MEETINGSTONE_SETQUEUE);
    }
}
