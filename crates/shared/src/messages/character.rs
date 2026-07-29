//! Character management message structs

use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{Opcode, WorldPacket};

// Hermes maps the legacy response enum by name before serializing it for the modern client.
// Oxcore's handlers retain legacy result values, so modern replies perform the same conversion.
fn modern_response_code(result: u8) -> u8 {
    match result {
        // CHAR_CREATE_*: legacy 45..55, modern 23..33.
        0x2D..=0x37 => result - 22,
        // CHAR_DELETE_*: legacy 56..59, modern 52..55.
        0x38..=0x3B => result - 4,
        // CHAR_LOGIN_*: legacy 60..68, modern 63..71.
        0x3C..=0x44 => result + 3,
        // CHAR_NAME validation errors: legacy 69..79, modern 82..92.
        0x45..=0x4F => result + 13,
        // CHAR_NAME_SUCCESS and CHAR_NAME_FAILURE have the same values in both tables.
        _ => result,
    }
}

/// SMSG_CHAR_CREATE - Character creation response
#[derive(Debug, Clone)]
pub struct SmsgCharCreate {
    pub result: u8,
    pub guid: (u64, u64),
}

impl ToWorldPacket for SmsgCharCreate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_CREATE);
        packet.write_u8(self.result);
        packet
    }
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(modern_response_code(self.result));
        writer.write_packed_guid_128(self.guid.0, self.guid.1);
        Some(writer.finish(Opcode::SMSG_CHAR_CREATE))
    }
}

/// SMSG_CHAR_DELETE - Character deletion response
#[derive(Debug, Clone)]
pub struct SmsgCharDelete {
    pub result: u8,
}

impl ToWorldPacket for SmsgCharDelete {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_DELETE);
        packet.write_u8(self.result);
        packet
    }
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHAR_DELETE);
        packet.write_u8(modern_response_code(self.result));
        Some(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_create_includes_result_and_packed_guid128() {
        let packet = SmsgCharCreate {
            result: 0,
            guid: (0x0100, 0x020001),
        }
        .to_modern()
        .unwrap();
        assert_eq!(packet.contents(), &[0, 0x05, 0x02, 1, 2, 1]);
    }

    #[test]
    fn modern_rename_uses_bit_length_and_raw_name() {
        let packet = SmsgCharRename {
            result: 0,
            guid: Some(1),
            new_name: Some("Bob".into()),
            guid128: Some((0, 1)),
        }
        .to_modern()
        .unwrap();
        assert_eq!(packet.contents(), &[0, 0x86, 1, 0, 1, b'B', b'o', b'b']);
    }

    #[test]
    fn modern_delete_translates_legacy_success() {
        let packet = SmsgCharDelete { result: 0x39 }.to_modern().unwrap();
        assert_eq!(packet.contents(), &[53]); // Hermes CharDeleteSuccess
    }
}

/// SMSG_CHAR_RENAME - Character rename response
#[derive(Debug, Clone)]
pub struct SmsgCharRename {
    pub result: u8,
    pub guid: Option<u64>,        // Only on success (result = 0x00)
    pub new_name: Option<String>, // Only on success
    pub guid128: Option<(u64, u64)>,
}

impl ToWorldPacket for SmsgCharRename {
    fn to_vanilla(&self) -> WorldPacket {
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
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(modern_response_code(self.result));
        writer.write_bit(self.guid128.is_some());
        writer.write_bits(
            self.new_name.as_ref().map_or(0, |name| name.len() as u32),
            6,
        );
        writer.flush_bits();
        if let Some(guid) = self.guid128 {
            writer.write_packed_guid_128(guid.0, guid.1);
        }
        if let Some(name) = &self.new_name {
            writer.write_string_raw(name);
        }
        Some(writer.finish(Opcode::SMSG_CHAR_RENAME))
    }
}

/// SMSG_LOGOUT_RESPONSE - Response to logout request
#[derive(Debug, Clone)]
pub struct SmsgLogoutResponse {
    pub reason: u8,    // 0 = can logout, 1 = in combat, etc.
    pub instant: bool, // true = instant logout, false = timer countdown
}

impl ToWorldPacket for SmsgLogoutResponse {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LOGOUT_RESPONSE);
        packet.write_u8(self.reason);
        packet.write_u8(if self.instant { 1 } else { 0 });
        packet
    }

    /// The reason widens to an i32 and `instant` becomes a flushed bit.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(self.reason as i32);
        writer.write_bit(self.instant);
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_LOGOUT_RESPONSE))
    }
}

/// SMSG_LOGOUT_COMPLETE - Logout complete notification
#[derive(Debug, Clone)]
pub struct SmsgLogoutComplete;

impl ToWorldPacket for SmsgLogoutComplete {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOGOUT_COMPLETE)
    }

    /// Empty in both protocols.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(WorldPacket::new(Opcode::SMSG_LOGOUT_COMPLETE))
    }
}

/// SMSG_LOGOUT_CANCEL_ACK - Logout cancellation acknowledgment
#[derive(Debug, Clone)]
pub struct SmsgLogoutCancelAck;

impl ToWorldPacket for SmsgLogoutCancelAck {
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_LOGOUT_CANCEL_ACK)
    }

    /// Empty in both protocols.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(WorldPacket::new(Opcode::SMSG_LOGOUT_CANCEL_ACK))
    }
}
