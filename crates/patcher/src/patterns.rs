//! Byte signatures located in the client executable.
//!
//! The two modulus signatures are the *first eight bytes* of a 256-byte RSA modulus; the scan
//! finds the prefix and the patch rewrites the full 256 bytes starting at that offset.
//!
//! Only [`PORTAL`] and [`SIGNATURE_MODULUS`] are required for 1.14.x. The rest are kept here
//! because they are needed for 2.5/3.4/4.4 and for the world-side `SMSG_CONNECT_TO` work, and
//! rediscovering them later is expensive.

/// The domain suffix the client appends to the `portal` value from `WTF/Config.wtf`.
pub const PORTAL: &[u8] = b".actual.battle.net";

/// Length of an RSA modulus in the client, in bytes.
pub const MODULUS_LEN: usize = 256;

/// Prefix of the modulus used to verify the certificate bundle's signature.
pub const SIGNATURE_MODULUS: &[u8] = &[0x35, 0xFF, 0x17, 0xE7, 0x33, 0xC4, 0xD3, 0xD4];

/// Prefix of the modulus used by the world-side `SMSG_CONNECT_TO` redirect. Not patched yet.
pub const CONNECT_TO_MODULUS: &[u8] = &[0x91, 0xD5, 0x9B, 0xB7, 0xD4, 0xE1, 0x83, 0xA5];

/// Start of the certificate bundle, which is embedded in the executable as a JSON blob.
pub const CERT_BUNDLE: &[u8] = br#"{"Created":"#;

/// Version and CDN endpoints, patched only on builds that fetch certificates at startup.
pub const VERSION_URL_V2: &[u8] = b"https://%s.version.battle.net/v2/products/%s/versions";
pub const CDNS_URL: &[u8] = b"http://%s.patch.battle.net:1119/%s/cdns";
