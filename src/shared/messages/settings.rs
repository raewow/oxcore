//! Settings message structs
//!
//! This module contains type-safe message structures for settings-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgUpdateAccountData`] - Response to account data update or request
//! - [`SmsgAccountDataTimes`] - Account data timestamps (sent during login)

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::game::player::settings::account_data::compress_account_data;

/// SMSG_UPDATE_ACCOUNT_DATA - Response to account data operations
///
/// Sent in response to:
/// - CMSG_UPDATE_ACCOUNT_DATA (echo back to confirm receipt)
/// - CMSG_REQUEST_ACCOUNT_DATA (provide requested data)
#[derive(Debug, Clone)]
pub struct SmsgUpdateAccountData {
    /// Account data type (0-7)
    pub data_type: u32,
    /// Decompressed data blob
    pub data: Vec<u8>,
}

impl ToWorldPacket for SmsgUpdateAccountData {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_ACCOUNT_DATA);
        packet.write_u32(self.data_type);

        if self.data.is_empty() {
            // Empty data - just write size 0
            packet.write_u32(0);
        } else {
            // Compress data like MaNGOS does
            match compress_account_data(&self.data) {
                Ok(compressed) => {
                    packet.write_bytes(&compressed);
                }
                Err(e) => {
                    // Fallback: write uncompressed with size prefix
                    tracing::warn!("Failed to compress account data: {}", e);
                    packet.write_u32(self.data.len() as u32);
                    packet.write_bytes(&self.data);
                }
            }
        }
        packet
    }
}

/// SMSG_ACCOUNT_DATA_TIMES - Account data timestamps
///
/// Sent during login to inform the client of the last modification time
/// for each of the 8 account data types. The client compares these to
/// its local cache and requests updates for stale data.
#[derive(Debug, Clone)]
pub struct SmsgAccountDataTimes {
    /// Unix timestamps for each of the 8 account data types
    pub timestamps: [u32; 8],
}

impl SmsgAccountDataTimes {
    /// Create with all zeros (no cached data)
    pub fn new(timestamps: [u32; 8]) -> Self {
        Self { timestamps }
    }

    /// Create with all zeros (no cached data on server)
    pub fn empty() -> Self {
        Self { timestamps: [0; 8] }
    }
}

impl Default for SmsgAccountDataTimes {
    fn default() -> Self {
        Self::empty()
    }
}

impl ToWorldPacket for SmsgAccountDataTimes {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ACCOUNT_DATA_TIMES);
        for &timestamp in &self.timestamps {
            packet.write_u32(timestamp);
        }
        packet
    }
}

/// SMSG_UPDATE_ACCOUNT_DATA_COMPLETE - Confirmation of account data update
///
/// Sent to confirm that account data has been successfully processed.
#[derive(Debug, Clone)]
pub struct SmsgUpdateAccountDataComplete {
    /// Account data type (0-7)
    pub data_type: u32,
    /// Status code (0 = success)
    pub status: u32,
}

impl ToWorldPacket for SmsgUpdateAccountDataComplete {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_ACCOUNT_DATA_COMPLETE);
        packet.write_u32(self.data_type);
        packet.write_u32(self.status);
        packet
    }
}
