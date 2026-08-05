//! SMSG_ADDON_INFO — reply to the addon-CRC block the vanilla 1.12 client appends to the end of
//! CMSG_AUTH_SESSION.
//!
//! Without this reply the client shows an "AddOn not verified" warning and disables custom
//! addons. Vanilla-only: the modern client never sends the addon block this responds to.

use crate::messages::ToWorldPacket;
use crate::protocol::{Opcode, WorldPacket};

/// Status codes for one addon entry in SMSG_ADDON_INFO.
#[repr(u8)]
enum AddonStatus {
    Visible = 1,
    Hidden = 2,
}

/// Expected CRC of the client's own addon-verification modulus. A "Blizzard"-named addon whose
/// modulus CRC doesn't match this gets the verification key blob below so the client can
/// re-derive it locally instead of flagging the addon as unverified.
const BLIZZARD_ADDON_MODULUS_CRC: u32 = 0x4C1C776D;

/// Public verification key blob for the client's addon CRC signing, sent when a reported
/// "Blizzard_*" addon's modulus CRC doesn't match [`BLIZZARD_ADDON_MODULUS_CRC`]. Fixed 256-byte
/// constant defined by the client-side verification protocol; copied byte-for-byte.
#[rustfmt::skip]
const BLIZZARD_ADDON_KEY: [u8; 256] = [
    0xC3, 0x5B, 0x50, 0x84, 0xB9, 0x3E, 0x32, 0x42, 0x8C, 0xD0, 0xC7, 0x48, 0xFA, 0x0E, 0x5D, 0x54,
    0x5A, 0xA3, 0x0E, 0x14, 0xBA, 0x9E, 0x0D, 0xB9, 0x5D, 0x8B, 0xEE, 0xB6, 0x84, 0x93, 0x45, 0x75,
    0xFF, 0x31, 0xFE, 0x2F, 0x64, 0x3F, 0x3D, 0x6D, 0x07, 0xD9, 0x44, 0x9B, 0x40, 0x85, 0x59, 0x34,
    0x4E, 0x10, 0xE1, 0xE7, 0x43, 0x69, 0xEF, 0x7C, 0x16, 0xFC, 0xB4, 0xED, 0x1B, 0x95, 0x28, 0xA8,
    0x23, 0x76, 0x51, 0x31, 0x57, 0x30, 0x2B, 0x79, 0x08, 0x50, 0x10, 0x1C, 0x4A, 0x1A, 0x2C, 0xC8,
    0x8B, 0x8F, 0x05, 0x2D, 0x22, 0x3D, 0xDB, 0x5A, 0x24, 0x7A, 0x0F, 0x13, 0x50, 0x37, 0x8F, 0x5A,
    0xCC, 0x9E, 0x04, 0x44, 0x0E, 0x87, 0x01, 0xD4, 0xA3, 0x15, 0x94, 0x16, 0x34, 0xC6, 0xC2, 0xC3,
    0xFB, 0x49, 0xFE, 0xE1, 0xF9, 0xDA, 0x8C, 0x50, 0x3C, 0xBE, 0x2C, 0xBB, 0x57, 0xED, 0x46, 0xB9,
    0xAD, 0x8B, 0xC6, 0xDF, 0x0E, 0xD6, 0x0F, 0xBE, 0x80, 0xB3, 0x8B, 0x1E, 0x77, 0xCF, 0xAD, 0x22,
    0xCF, 0xB7, 0x4B, 0xCF, 0xFB, 0xF0, 0x6B, 0x11, 0x45, 0x2D, 0x7A, 0x81, 0x18, 0xF2, 0x92, 0x7E,
    0x98, 0x56, 0x5D, 0x5E, 0x69, 0x72, 0x0A, 0x0D, 0x03, 0x0A, 0x85, 0xA2, 0x85, 0x9C, 0xCB, 0xFB,
    0x56, 0x6E, 0x8F, 0x44, 0xBB, 0x8F, 0x02, 0x22, 0x68, 0x63, 0x97, 0xBC, 0x85, 0xBA, 0xA8, 0xF7,
    0xB5, 0x40, 0x68, 0x3C, 0x77, 0x86, 0x6F, 0x4B, 0xD7, 0x88, 0xCA, 0x8A, 0xD7, 0xCE, 0x36, 0xF0,
    0x45, 0x6E, 0xD5, 0x64, 0x79, 0x0F, 0x17, 0xFC, 0x64, 0xDD, 0x10, 0x6F, 0xF3, 0xF5, 0xE0, 0xA6,
    0xC3, 0xFB, 0x1B, 0x8C, 0x29, 0xEF, 0x8E, 0xE5, 0x34, 0xCB, 0xD1, 0x2A, 0xCE, 0x79, 0xC3, 0x9A,
    0x0D, 0x36, 0xEA, 0x01, 0xE0, 0xAA, 0x91, 0x20, 0x54, 0xF0, 0x72, 0xD8, 0x1E, 0xC7, 0x89, 0xD2,
];

/// One parsed entry from the client's addon list. `flags` and `url_crc` are read off the wire for
/// completeness but don't affect the reply — only `name` (the "Blizzard" prefix check) and
/// `modulus_crc` do.
#[derive(Debug, Clone)]
pub struct AddonEntry {
    pub name: String,
    pub modulus_crc: u32,
}

/// SMSG_ADDON_INFO body: one reply entry per entry in `addons`, same order.
#[derive(Debug, Clone)]
pub struct SmsgAddonInfo<'a> {
    pub addons: &'a [AddonEntry],
}

impl ToWorldPacket for SmsgAddonInfo<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ADDON_INFO);
        for addon in self.addons {
            if addon.name.contains("Blizzard") {
                packet.write_u8(AddonStatus::Hidden as u8);
                packet.write_u8(1); // InfoProvided
                if addon.modulus_crc != BLIZZARD_ADDON_MODULUS_CRC {
                    packet.write_u8(1); // KeyProvided
                    packet.write_bytes(&BLIZZARD_ADDON_KEY);
                } else {
                    packet.write_u8(0); // KeyProvided
                }
                packet.write_u32(0); // Revision
                packet.write_u8(0); // UrlProvided
            } else {
                packet.write_u8(AddonStatus::Visible as u8);
                packet.write_u8(0); // InfoProvided
                packet.write_u8(0); // UrlProvided
            }
        }
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blizzard_addon_matching_crc_sends_no_key() {
        let addons = [AddonEntry {
            name: "Blizzard_AuctionUi".into(),
            modulus_crc: BLIZZARD_ADDON_MODULUS_CRC,
        }];
        let packet = SmsgAddonInfo { addons: &addons }.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_ADDON_INFO);
        assert_eq!(packet.contents(), &[2, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn blizzard_addon_mismatched_crc_sends_key_blob() {
        let addons = [AddonEntry {
            name: "Blizzard_DebugTools".into(),
            modulus_crc: 0,
        }];
        let packet = SmsgAddonInfo { addons: &addons }.to_vanilla();
        let mut expected = vec![2u8, 1, 1];
        expected.extend_from_slice(&BLIZZARD_ADDON_KEY);
        expected.extend_from_slice(&[0, 0, 0, 0, 0]);
        assert_eq!(packet.contents(), expected.as_slice());
    }

    #[test]
    fn custom_addon_is_visible_with_no_info() {
        let addons = [AddonEntry {
            name: "MyCoolAddon".into(),
            modulus_crc: 0xDEADBEEF,
        }];
        let packet = SmsgAddonInfo { addons: &addons }.to_vanilla();
        assert_eq!(packet.contents(), &[1, 0, 0]);
    }

    #[test]
    fn multiple_entries_concatenate_in_order() {
        let addons = [
            AddonEntry {
                name: "MyCoolAddon".into(),
                modulus_crc: 0,
            },
            AddonEntry {
                name: "Blizzard_AuctionUi".into(),
                modulus_crc: BLIZZARD_ADDON_MODULUS_CRC,
            },
        ];
        let packet = SmsgAddonInfo { addons: &addons }.to_vanilla();
        assert_eq!(packet.contents(), &[1, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0]);
    }
}
