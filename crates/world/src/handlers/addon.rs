//! CMSG_AUTH_SESSION addon-block parsing — builds SMSG_ADDON_INFO.
//!
//! The vanilla 1.12 client appends a zlib-compressed addon list to the end of
//! CMSG_AUTH_SESSION, after the SRP6 digest. Vanilla-only: the modern client never sends this.
//! A missing or malformed block is not fatal to login — the client just won't get addon CRC
//! verification, which at worst shows an in-game warning.

use bytes::{Buf, BytesMut};
use flate2::read::ZlibDecoder;
use std::io::Read;
use tracing::{debug, warn};

use oxcore_shared::messages::addon::{AddonEntry, SmsgAddonInfo};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{Opcode, WorldPacket};

/// Cap on the claimed decompressed size, matching what a real 1.12 client can send. Rejected
/// before any decompression is attempted.
const MAX_ADDON_INFO_SIZE: u32 = 0xFFFFF;

/// Parse the addon block trailing the current cursor position of `packet` (expected to be
/// immediately after the CMSG_AUTH_SESSION digest) and build the SMSG_ADDON_INFO reply.
///
/// Returns `None` — logging why — if the block is missing, empty, oversized, or fails to
/// decompress. Never errors out to the caller: a bad addon block must not fail login.
pub fn build_addon_info_response(packet: &mut WorldPacket) -> Option<WorldPacket> {
    let real_size = packet.read_u32()?; // no bytes left -> client sent no addon block

    if real_size == 0 {
        debug!("CMSG_AUTH_SESSION: empty addon block, skipping SMSG_ADDON_INFO");
        return None;
    }
    if real_size > MAX_ADDON_INFO_SIZE {
        warn!("CMSG_AUTH_SESSION: addon info too big, size {}", real_size);
        return None;
    }

    let compressed = packet.contents();
    let mut decoder = ZlibDecoder::new(compressed).take(real_size as u64);
    let mut decompressed = Vec::with_capacity(real_size as usize);
    if let Err(e) = decoder.read_to_end(&mut decompressed) {
        warn!("CMSG_AUTH_SESSION: addon block zlib decompress failed: {}", e);
        return None;
    }

    let mut reader = WorldPacket::from_data(Opcode::NONE, BytesMut::from(&decompressed[..]));
    let mut addons = Vec::new();
    loop {
        if reader.data().remaining() == 0 {
            break;
        }
        let Some(name) = reader.read_string() else {
            break;
        };
        let Some(_flags) = reader.read_u8() else {
            break;
        };
        let Some(modulus_crc) = reader.read_u32() else {
            break;
        };
        let Some(_url_crc) = reader.read_u32() else {
            break;
        };
        addons.push(AddonEntry { name, modulus_crc });
    }

    debug!("CMSG_AUTH_SESSION: parsed {} addon entries", addons.len());
    Some(SmsgAddonInfo { addons: &addons }.to_vanilla())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn build_synthetic_addon_block(entries: &[(&str, u8, u32, u32)]) -> WorldPacket {
        let mut raw = WorldPacket::new(Opcode::NONE);
        for (name, flags, modulus_crc, url_crc) in entries {
            raw.write_cstring(name);
            raw.write_u8(*flags);
            raw.write_u32(*modulus_crc);
            raw.write_u32(*url_crc);
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(raw.contents()).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut packet = WorldPacket::new(Opcode::CMSG_AUTH_SESSION);
        packet.write_u32(raw.contents().len() as u32);
        packet.write_bytes(&compressed);
        packet
    }

    #[test]
    fn parses_blizzard_and_custom_addon_entries() {
        let mut packet = build_synthetic_addon_block(&[
            ("Blizzard_AuctionUi", 0, 0x4C1C_776D, 0),
            ("MyCustomAddon", 0, 0xDEAD_BEEF, 0),
        ]);
        let reply = build_addon_info_response(&mut packet).expect("valid block parses");
        assert_eq!(reply.opcode(), Opcode::SMSG_ADDON_INFO);
        assert_eq!(reply.contents(), &[2, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0]);
    }

    #[test]
    fn missing_addon_block_is_graceful_noop() {
        let mut packet = WorldPacket::new(Opcode::CMSG_AUTH_SESSION); // no trailing bytes
        assert!(build_addon_info_response(&mut packet).is_none());
    }

    #[test]
    fn zero_claimed_size_is_graceful_noop() {
        let mut packet = WorldPacket::new(Opcode::CMSG_AUTH_SESSION);
        packet.write_u32(0);
        assert!(build_addon_info_response(&mut packet).is_none());
    }

    #[test]
    fn oversized_claimed_size_is_rejected_before_decompressing() {
        let mut packet = WorldPacket::new(Opcode::CMSG_AUTH_SESSION);
        packet.write_u32(0x100000); // > 0xFFFFF cap
        assert!(build_addon_info_response(&mut packet).is_none());
    }

    #[test]
    fn corrupt_zlib_stream_is_graceful_noop() {
        let mut packet = WorldPacket::new(Opcode::CMSG_AUTH_SESSION);
        packet.write_u32(16);
        packet.write_bytes(&[0xFF; 8]); // not a valid zlib stream
        assert!(build_addon_info_response(&mut packet).is_none());
    }
}
