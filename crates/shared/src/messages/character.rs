//! Character management message structs

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{Opcode, WorldPacket};

/// SMSG_CHAR_CREATE - Character creation response
#[derive(Debug, Clone)]
pub struct SmsgCharCreate {
    pub result: u8,
}

impl ToWorldPacket for SmsgCharCreate {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_CREATE);
        packet.write_u8(self.result);
        packet
    }
}

/// SMSG_CHAR_DELETE - Character deletion response
#[derive(Debug, Clone)]
pub struct SmsgCharDelete {
    pub result: u8,
}

impl ToWorldPacket for SmsgCharDelete {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_DELETE);
        packet.write_u8(self.result);
        packet
    }
}

/// SMSG_CHAR_RENAME - Character rename response
#[derive(Debug, Clone)]
pub struct SmsgCharRename {
    pub result: u8,
    pub guid: Option<u64>,        // Only on success (result = 0x00)
    pub new_name: Option<String>, // Only on success
}

impl ToWorldPacket for SmsgCharRename {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_RENAME);
        packet.write_u8(self.result);

        // Send GUID and name only on success
        if self.result == 0x00 {
            if let (Some(guid), Some(name)) = (self.guid, &self.new_name) {
                packet.write_u64(guid);
                packet.write_cstring(name);
            }
        }

        packet
    }
}

/// SMSG_LOGOUT_RESPONSE - Response to logout request
#[derive(Debug, Clone)]
pub struct SmsgLogoutResponse {
    pub reason: u8,    // 0 = can logout, 1 = in combat, etc.
    pub instant: bool, // true = instant logout, false = timer countdown
}

impl ToWorldPacket for SmsgLogoutResponse {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOGOUT_RESPONSE);
        packet.write_u8(self.reason);
        packet.write_u8(if self.instant { 1 } else { 0 });
        packet
    }
}

/// SMSG_LOGOUT_COMPLETE - Logout complete notification
#[derive(Debug, Clone)]
pub struct SmsgLogoutComplete;

impl ToWorldPacket for SmsgLogoutComplete {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOGOUT_COMPLETE)
    }
}

/// SMSG_LOGOUT_CANCEL_ACK - Logout cancellation acknowledgment
#[derive(Debug, Clone)]
pub struct SmsgLogoutCancelAck;

impl ToWorldPacket for SmsgLogoutCancelAck {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOGOUT_CANCEL_ACK)
    }
}
