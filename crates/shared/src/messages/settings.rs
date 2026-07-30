//! Settings message structs
//!
//! This module contains type-safe message structures for settings-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgUpdateAccountData`] - Response to account data update or request
//! - [`SmsgAccountDataTimes`] - Account data timestamps (sent during login)

use crate::game::account_data::compress_account_data;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};

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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_ACCOUNT_DATA);
        packet.write_u32(self.data_type);

        if self.data.is_empty() {
            // Empty data - just write size 0
            packet.write_u32(0);
        } else {
            // Compress data
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
    /// Owning player, and the realm to qualify it with. Only the modern body carries these; the
    /// vanilla body is a bare list of timestamps.
    pub player_guid: ObjectGuid,
    pub realm_id: u16,
}

impl SmsgAccountDataTimes {
    /// Create with all zeros (no cached data)
    pub fn new(timestamps: [u32; 8], player_guid: ObjectGuid, realm_id: u16) -> Self {
        Self {
            timestamps,
            player_guid,
            realm_id,
        }
    }

    /// Create with all zeros (no cached data on server)
    pub fn empty() -> Self {
        Self {
            timestamps: [0; 8],
            player_guid: ObjectGuid::empty(),
            realm_id: 1,
        }
    }
}

impl Default for SmsgAccountDataTimes {
    fn default() -> Self {
        Self::empty()
    }
}

/// How many per-type timestamps the modern client expects.
///
/// Grew past vanilla's 8 as newer data types were added; 1.14.1 and later use 13. We only track
/// the original 8, so the tail is sent as zeros — "never cached", which makes the client ask.
const MODERN_ACCOUNT_DATA_COUNT: usize = 13;

impl ToWorldPacket for SmsgAccountDataTimes {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ACCOUNT_DATA_TIMES);
        for &timestamp in &self.timestamps {
            packet.write_u32(timestamp);
        }
        packet
    }

    /// The modern body is a different message: it names the player, carries a server time, and
    /// widens every timestamp to `i64`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.player_guid.to_guid128(self.realm_id);
        writer.write_packed_guid_128(high, low);
        writer.write_i64(chrono::Utc::now().timestamp());
        for index in 0..MODERN_ACCOUNT_DATA_COUNT {
            let timestamp = self.timestamps.get(index).copied().unwrap_or(0);
            writer.write_i64(timestamp as i64);
        }
        Some(writer.finish(Opcode::SMSG_ACCOUNT_DATA_TIMES))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_ACCOUNT_DATA_COMPLETE);
        packet.write_u32(self.data_type);
        packet.write_u32(self.status);
        packet
    }
}
