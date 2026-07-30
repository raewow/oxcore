use crate::protocol::opcodes::Opcode;
use crate::protocol::WorldPacket;
use anyhow::Result;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

/// Compression configuration
pub mod config {
    /// Compression threshold in bytes
    pub const COMPRESSION_THRESHOLD: usize = 128;

    /// Compression level (1 = Z_BEST_SPEED)
    pub const COMPRESSION_LEVEL: u32 = 1;

    /// Maximum safe packet size before warning
    pub const MAX_SAFE_PACKET_SIZE: usize = 900000;
}

/// Compress update packet if it exceeds the threshold
///
/// **Compression Process**:
/// 1. Check if packet size > threshold (default: 128 bytes)
/// 2. If yes:
///    - Compress using zlib deflate with level 1 (Z_BEST_SPEED)
///    - Format: `[uncompressed_size: uint32] + [compressed_data]`
///    - Change opcode to `SMSG_COMPRESSED_UPDATE_OBJECT`
/// 3. If no:
///    - Return packet unchanged with `SMSG_UPDATE_OBJECT` opcode
///
/// **Compression Algorithm**:
/// - Uses zlib deflate
/// - Compression level: 1 (Z_BEST_SPEED)
///
/// **Format**:
/// - Uncompressed: `SMSG_UPDATE_OBJECT` opcode + packet data
/// - Compressed: `SMSG_COMPRESSED_UPDATE_OBJECT` opcode + `[uncompressed_size: u32] + [compressed_data]`
pub fn compress_update_packet_if_needed(packet: WorldPacket) -> Result<WorldPacket> {
    let packet_bytes = packet.data();
    let uncompressed_size = packet_bytes.len();

    // Check threshold
    if uncompressed_size <= config::COMPRESSION_THRESHOLD {
        return Ok(packet);
    }

    // Warn if packet is very large
    if uncompressed_size >= config::MAX_SAFE_PACKET_SIZE {
        tracing::warn!(
            "[CRASH-CLIENT] Too large packet: {} bytes",
            uncompressed_size
        );
    }

    // Compress using zlib deflate with level 1 (Z_BEST_SPEED)
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(packet_bytes.as_ref())
        .map_err(|e| anyhow::anyhow!("Failed to compress packet: {}", e))?;
    let compressed_data = encoder
        .finish()
        .map_err(|e| anyhow::anyhow!("Failed to finish compression: {}", e))?;

    // Create compressed packet: [uncompressed_size uint32] + [compressed_data]
    let mut compressed_packet = WorldPacket::new(Opcode::SMSG_COMPRESSED_UPDATE_OBJECT);
    compressed_packet.write_u32(uncompressed_size as u32);
    compressed_packet.write_bytes(&compressed_data);

    Ok(compressed_packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_packet_not_compressed() {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        // Write data smaller than threshold
        for _ in 0..50 {
            packet.write_u8(0);
        }

        let result = compress_update_packet_if_needed(packet).unwrap();
        assert_eq!(result.opcode(), Opcode::SMSG_UPDATE_OBJECT);
    }

    #[test]
    fn test_large_packet_compressed() {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
        // Write data larger than threshold
        for _ in 0..200 {
            packet.write_u8(0);
        }

        let result = compress_update_packet_if_needed(packet).unwrap();
        assert_eq!(result.opcode(), Opcode::SMSG_COMPRESSED_UPDATE_OBJECT);

        let data = result.data();
        assert!(data.len() >= 4); // At least the uncompressed size
        let uncompressed_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(uncompressed_size, 200);
    }
}
