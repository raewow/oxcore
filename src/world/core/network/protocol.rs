//! Packet protocol - header formats and parsing
//!
//! World of Warcraft 1.12.x packet format:
//!
//! Client to Server (CMSG):
//! - [2 bytes] Size (big endian) - includes opcode but not size field itself
//! - [4 bytes] Opcode (little endian)
//! - [N bytes] Payload
//!
//! Server to Client (SMSG):
//! - [2 bytes] Size (big endian) - includes opcode but not size field itself
//! - [2 bytes] Opcode (little endian)
//! - [N bytes] Payload

use crate::shared::protocol::Opcode;

/// Client packet header size (6 bytes)
/// [2 bytes] Size (big endian) - includes opcode
/// [4 bytes] Opcode (little endian)
pub const CLIENT_HEADER_SIZE: usize = 6;

/// Server packet header size (4 bytes)
/// [2 bytes] Size (big endian) - includes opcode
/// [2 bytes] Opcode (little endian)
pub const SERVER_HEADER_SIZE: usize = 4;

/// Maximum packet size (64 KB)
/// Vanilla WoW CMSG_AUTH_SESSION includes addon data which can exceed 40KB with many addons
pub const MAX_PACKET_SIZE: usize = 65536;

/// Minimum packet size (just the header)
pub const MIN_CLIENT_PACKET_SIZE: usize = CLIENT_HEADER_SIZE;
pub const MIN_SERVER_PACKET_SIZE: usize = SERVER_HEADER_SIZE;

/// Parse client packet header
///
/// Returns (size, opcode) where size includes the opcode bytes.
/// The actual payload size is: size - 4 (opcode size)
pub fn parse_client_header(data: &[u8]) -> Option<(u16, Opcode)> {
    if data.len() < CLIENT_HEADER_SIZE {
        return None;
    }

    // Size is big-endian
    let size = u16::from_be_bytes([data[0], data[1]]);

    // Opcode is little-endian, 4 bytes
    let opcode = u32::from_le_bytes([data[2], data[3], data[4], data[5]]);

    Some((size, Opcode::new(opcode)))
}

/// Build server packet header
///
/// Returns a 4-byte header: [size BE] [opcode LE]
/// Size should include the opcode (2 bytes) plus payload size
pub fn build_server_header(size: u16, opcode: Opcode) -> [u8; SERVER_HEADER_SIZE] {
    let mut header = [0u8; SERVER_HEADER_SIZE];
    header[0..2].copy_from_slice(&size.to_be_bytes());
    header[2..4].copy_from_slice(&opcode.as_u16().to_le_bytes());
    header
}

/// Calculate the total packet size for a server packet
///
/// Returns the value to put in the size field: opcode (2 bytes) + payload length
pub fn server_packet_size(payload_len: usize) -> u16 {
    // Size field includes opcode (2 bytes) + payload
    (2 + payload_len) as u16
}

/// Calculate the payload size from client packet header size field
///
/// Client size field includes the opcode (4 bytes)
pub fn client_payload_size(header_size: u16) -> usize {
    // Size includes opcode (4 bytes), so payload is size - 4
    header_size.saturating_sub(4) as usize
}
