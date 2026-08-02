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
    /// Owning player. Only the modern body names it — the 1.14 client keys its cache on this GUID.
    pub player_guid: ObjectGuid,
    /// Realm qualifying `player_guid`'s 128-bit form; must match what the character list used.
    pub realm_id: u16,
    /// Modification time. Only the modern body carries it; the client compares it against what
    /// `SMSG_ACCOUNT_DATA_TIMES` promised.
    pub time: u32,
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

    /// The 1.14 body is a different message from the vanilla one (this is why the shared opcode is
    /// used): it names the owner, carries a modification time, and moves the data type into the
    /// bit tail. Layout from `ClientConfigPackets.cs` `UpdateAccountData`:
    ///
    /// ```text
    /// PackedGuid128 Player
    /// i64  Time
    /// u32  Size        // decompressed size
    /// bits DataType    // 4 bits — this build tracks 13 account data types
    /// u32  length      // 0 when there is no blob, else compressed length
    /// u8[] data        // zlib
    /// ```
    ///
    /// Both leading fields are load-bearing: the client keys its cache on the GUID and refuses to
    /// take the blob unless the timestamp matches what `SMSG_ACCOUNT_DATA_TIMES` promised. A zero
    /// `Time` makes it re-request forever; a wrong owner makes it drop the data as foreign.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.player_guid.to_guid128(self.realm_id);
        writer.write_packed_guid_128(high, low);
        writer.write_i64(self.time as i64);
        writer.write_u32(self.data.len() as u32);
        // 4 bits for a 13-type client (3 would do for the original 8).
        writer.write_bits(self.data_type, 4);

        if self.data.is_empty() {
            writer.write_u32(0);
        } else {
            // The compressor already prefixes the decompressed size; the modern body wants the
            // zlib payload and its length separately.
            let compressed = compress_account_data(&self.data).ok()?;
            writer.write_u32((compressed.len() - 4) as u32);
            writer.write_bytes(&compressed[4..]);
        }
        Some(writer.finish(Opcode::SMSG_UPDATE_ACCOUNT_DATA))
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

/// 1.14.1 has thirteen account-data slots, including five types that did not exist in vanilla.
///
/// The server stores the original eight types, so the modern-only tail is zero-filled. It must
/// still be present: the client uses this count to choose the four-bit account-data type layout.
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

/// Not ported to 1.14: the opcode was retired and has no replacement.
///
/// 1.14 has no "account data update complete" message — the client treats the echoed
/// `SMSG_UPDATE_ACCOUNT_DATA` as its acknowledgement — so there is no body to encode and no opcode
/// to send it under.
impl ToWorldPacket for SmsgUpdateAccountDataComplete {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_ACCOUNT_DATA_COMPLETE);
        packet.write_u32(self.data_type);
        packet.write_u32(self.status);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::account_data::decompress_account_data;
    use crate::protocol::bitbuf::BitReader;
    use crate::protocol::HighGuid;

    fn player_guid(counter: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, counter)
    }

    #[test]
    fn modern_account_data_body_round_trips() {
        let guid = player_guid(1);
        let payload = b"keybindings\0and macros in here".to_vec();
        let msg = SmsgUpdateAccountData {
            data_type: 2, // GlobalBindings
            data: payload.clone(),
            player_guid: guid,
            realm_id: 1,
            time: 1_700_000_000,
        };

        let packet = msg.to_modern().expect("modern body exists");
        assert_eq!(packet.opcode(), Opcode::SMSG_UPDATE_ACCOUNT_DATA);
        assert_eq!(packet.opcode().modern(), 0x26FF);

        let mut reader = BitReader::new(packet.contents());
        let (high, low) = reader.read_packed_guid_128().expect("owner guid present");
        assert_eq!((high, low), guid.to_guid128(1));
        assert_eq!(reader.read_i64(), Some(1_700_000_000));
        assert_eq!(reader.read_u32(), Some(payload.len() as u32));
        assert_eq!(reader.read_bits(4), Some(2));
        let compressed_len = reader.read_u32().expect("compressed length") as usize;
        let compressed = reader
            .read_bytes(compressed_len)
            .expect("compressed payload");
        assert_eq!(
            decompress_account_data(compressed, payload.len() as u32).expect("inflates"),
            payload
        );
    }

    #[test]
    fn modern_account_data_body_with_empty_blob_has_zero_length() {
        let msg = SmsgUpdateAccountData {
            data_type: 6,
            data: Vec::new(),
            player_guid: player_guid(42),
            realm_id: 1,
            time: 0,
        };

        let packet = msg.to_modern().expect("modern body exists");
        let mut reader = BitReader::new(packet.contents());
        let (high, low) = reader.read_packed_guid_128().expect("owner guid present");
        assert_eq!((high, low), player_guid(42).to_guid128(1));
        assert_eq!(reader.read_i64(), Some(0));
        assert_eq!(reader.read_u32(), Some(0));
        assert_eq!(reader.read_bits(4), Some(6));
        assert_eq!(reader.read_u32(), Some(0));
        assert!(reader.read_bytes(1).is_none(), "nothing left over");
    }

    #[test]
    fn modern_account_data_times_has_all_modern_account_data_slots() {
        let guid = player_guid(42);
        let timestamps = [1, 2, 3, 4, 5, 6, 7, 8];
        let packet = SmsgAccountDataTimes::new(timestamps, guid, 1)
            .to_modern()
            .expect("modern body exists");
        let mut reader = BitReader::new(packet.contents());

        assert_eq!(reader.read_packed_guid_128(), Some(guid.to_guid128(1)));
        assert!(reader.read_i64().is_some(), "server time");
        for timestamp in timestamps {
            assert_eq!(reader.read_i64(), Some(i64::from(timestamp)));
        }
        for _ in timestamps.len()..MODERN_ACCOUNT_DATA_COUNT {
            assert_eq!(reader.read_i64(), Some(0), "modern-only timestamp is empty");
        }
        assert!(
            reader.read_i64().is_none(),
            "no timestamps beyond the modern account-data slots"
        );
    }
}
