//! Account data types and compression utilities
//!
//! The 1.12 client manages 8 distinct account data blobs. Some are stored per-account
//! (shared across all characters), others per-character.

use anyhow::Result;

/// Account data type indices as used by the 1.12 protocol.
///
/// The client sends and receives these as u32 type discriminants.
/// Account-wide types use `account_id` as the storage key.
/// Per-character types use `character_guid` as the storage key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AccountDataType {
    GlobalConfig = 0,    // Account-wide: general config (video, sound)
    PerCharConfig = 1,   // Per-character: character-specific config
    GlobalBindings = 2,  // Account-wide: key bindings
    PerCharBindings = 3, // Per-character: character-specific bindings
    GlobalMacros = 4,    // Account-wide: macros shared across characters
    PerCharMacros = 5,   // Per-character: character-specific macros
    PerCharLayout = 6,   // Per-character: UI layout (action bar positions)
    PerCharChat = 7,     // Per-character: chat settings (channels, filters)
}

impl AccountDataType {
    /// Convert from raw u32. Returns None if out of range.
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::GlobalConfig),
            1 => Some(Self::PerCharConfig),
            2 => Some(Self::GlobalBindings),
            3 => Some(Self::PerCharBindings),
            4 => Some(Self::GlobalMacros),
            5 => Some(Self::PerCharMacros),
            6 => Some(Self::PerCharLayout),
            7 => Some(Self::PerCharChat),
            _ => None,
        }
    }

    /// Whether this data type is account-wide (true) or per-character (false).
    pub fn is_global(&self) -> bool {
        matches!(
            self,
            Self::GlobalConfig | Self::GlobalBindings | Self::GlobalMacros
        )
    }

    /// Get the u32 value for this account data type.
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Decompress a zlib-compressed account data blob received from the client.
///
/// The 1.12 client sends CMSG_UPDATE_ACCOUNT_DATA as:
///   u32 data_type, u32 decompressed_size, u8[] raw_zlib_data
///
/// The handler reads data_type and decompressed_size from the packet, then
/// passes the remaining raw zlib bytes here along with the size.
pub fn decompress_account_data(compressed: &[u8], decompressed_size: u32) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let decompressed_size = decompressed_size as usize;

    // Safety cap: account data blobs should never exceed 64KB
    const MAX_ACCOUNT_DATA_SIZE: usize = 65536;
    if decompressed_size > MAX_ACCOUNT_DATA_SIZE {
        anyhow::bail!(
            "Account data decompressed size {} exceeds maximum {}",
            decompressed_size,
            MAX_ACCOUNT_DATA_SIZE
        );
    }

    let mut decoder = ZlibDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(decompressed_size);
    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}

/// Compress an account data blob for sending to the client.
///
/// Returns the compressed payload prefixed with the decompressed size u32.
pub fn compress_account_data(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut result = Vec::new();
    // Write decompressed size first
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());

    let mut encoder = ZlibEncoder::new(&mut result, Compression::default());
    encoder.write_all(data)?;
    encoder.finish()?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_data_type_from_u32() {
        assert_eq!(
            AccountDataType::from_u32(0),
            Some(AccountDataType::GlobalConfig)
        );
        assert_eq!(
            AccountDataType::from_u32(7),
            Some(AccountDataType::PerCharChat)
        );
        assert_eq!(AccountDataType::from_u32(8), None);
    }

    #[test]
    fn test_account_data_type_is_global() {
        assert!(AccountDataType::GlobalConfig.is_global());
        assert!(AccountDataType::GlobalBindings.is_global());
        assert!(AccountDataType::GlobalMacros.is_global());
        assert!(!AccountDataType::PerCharConfig.is_global());
        assert!(!AccountDataType::PerCharLayout.is_global());
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"Hello, World! This is some test data for account data compression.";
        let compressed = compress_account_data(original).unwrap();
        // compress_account_data prefixes [decompressed_size: 4 bytes][zlib data]
        // decompress_account_data expects raw zlib + separate size parameter
        let size = u32::from_le_bytes([compressed[0], compressed[1], compressed[2], compressed[3]]);
        let decompressed = decompress_account_data(&compressed[4..], size).unwrap();
        assert_eq!(original.to_vec(), decompressed);
    }
}
