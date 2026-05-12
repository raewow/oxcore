use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;
use crate::world::core::common::packet::WorldPacketGuidExt;
use anyhow::Result;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
use tracing::info;

pub mod config {
    pub const COMPRESSION_THRESHOLD: usize = 128;
    pub const COMPRESSION_LEVEL: u32 = 1;
    pub const MAX_SAFE_PACKET_SIZE: usize = 900000;
}

pub fn compress_update_packet_if_needed(packet: WorldPacket) -> Result<WorldPacket> {
    let packet_bytes = packet.data();
    let uncompressed_size = packet_bytes.len();

    if uncompressed_size <= config::COMPRESSION_THRESHOLD {
        return Ok(packet);
    }

    if uncompressed_size >= config::MAX_SAFE_PACKET_SIZE {
        tracing::warn!(
            "[CRASH-CLIENT] Too large packet: {} bytes",
            uncompressed_size
        );
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(packet_bytes.as_ref())
        .map_err(|e| anyhow::anyhow!("Failed to compress packet: {}", e))?;
    let compressed_data = encoder
        .finish()
        .map_err(|e| anyhow::anyhow!("Failed to finish compression: {}", e))?;

    let mut compressed_packet = WorldPacket::new(Opcode::SMSG_COMPRESSED_UPDATE_OBJECT);
    compressed_packet.write_u32(uncompressed_size as u32);
    compressed_packet.write_bytes(&compressed_data);

    Ok(compressed_packet)
}

pub fn compress_update_object<T: ToWorldPacket + Clone + Send>(msg: T) -> Result<WorldPacket> {
    let packet = msg.to_world_packet();
    compress_update_packet_if_needed(packet)
}
