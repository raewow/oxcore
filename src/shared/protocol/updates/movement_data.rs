use anyhow::Result;
use bytes::{BufMut, BytesMut};

use crate::shared::protocol::opcodes::Opcode;
use crate::shared::protocol::{WorldPacket, packet::WorldPacketGuidExt};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

/// Compress data using zlib deflate with Z_BEST_SPEED (level 1)
/// Returns the compressed data on success
fn compress_movement_data(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder
        .finish()
        .map_err(|e| anyhow::anyhow!("Compression failed: {}", e))
}

/// MovementData handles compressed movement update packets
/// Equivalent to the C++ MovementData class
/// Available for SUPPORTED_CLIENT_BUILD > CLIENT_BUILD_1_7_1 (which includes 1.12.1)
#[derive(Debug, Clone)]
pub struct MovementData {
    /// Buffer containing accumulated movement packets
    m_buffer: BytesMut,
}

impl MovementData {
    /// Create a new MovementData instance
    pub fn new() -> Self {
        Self {
            m_buffer: BytesMut::with_capacity(1024),
        }
    }

    /// Check if a packet can be added to the buffer
    /// Returns false if the packet would exceed size limits
    pub fn can_add_packet(&self, data: &WorldPacket) -> bool {
        let packet_size = data.size();
        let opcode_size = 2; // uint16 opcode

        // Since packet size is stored with a u8, packet size is limited for compressed packets
        if (packet_size + opcode_size) > 0xFF {
            return false;
        }

        let total_size = self.m_buffer.len() + packet_size + opcode_size;
        if total_size >= 900000 {
            return false;
        }

        true
    }

    /// Add a packet to the movement buffer
    /// Panics if can_add_packet() would return false
    pub fn add_packet(&mut self, data: &WorldPacket) {
        let packet_size = data.size();
        let opcode_size = 2; // uint16 opcode

        assert!(
            packet_size + opcode_size <= 0xFF,
            "Max packet size exceeded for compressed packets"
        );

        // Write packet size + opcode size
        self.m_buffer.put_u8((packet_size + opcode_size) as u8);
        // Write opcode
        self.m_buffer.put_u16_le(data.opcode().as_u16());
        // Append packet data
        self.m_buffer.extend_from_slice(data.data());
    }

    /// Build a compressed movement packet from the accumulated data
    /// Returns the built packet on success, error on compression failure
    pub fn build_packet(&mut self) -> Result<WorldPacket> {
        let packet_size = self.m_buffer.len();

        if packet_size >= 900000 {
            tracing::warn!(
                "[CRASH-CLIENT] Too large packet size {} (SMSG_COMPRESSED_MOVES)",
                packet_size
            );
        }

        // Compress the buffer
        let compressed_data = compress_movement_data(&self.m_buffer)?;

        // Create the final packet
        let mut packet = WorldPacket::new(Opcode::SMSG_COMPRESSED_MOVES);
        let packet_data = packet.data_mut();

        // Write uncompressed size
        packet_data.put_u32_le(packet_size as u32);
        // Append compressed data
        packet_data.extend_from_slice(&compressed_data);

        Ok(packet)
    }

    /// Check if there's any data in the buffer
    pub fn has_data(&self) -> bool {
        !self.m_buffer.is_empty()
    }

    /// Clear the buffer
    pub fn clear_buffer(&mut self) {
        self.m_buffer.clear();
    }
}

impl Default for MovementData {
    fn default() -> Self {
        Self::new()
    }
}
