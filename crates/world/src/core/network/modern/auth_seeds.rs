//! Per-(build, OS) auth seeds mixed into the `CMSG_AUTH_SESSION` digest.
//!
//! The client hashes its session key with a seed that depends on its exact build and operating
//! system, so the server must know that seed to reproduce the digest. Values transcribed from
//! HermesProxy's `BuildAuthSeeds.csv`. The build and OS are not in `CMSG_AUTH_SESSION` itself —
//! they come from the session context established during the bnet flow.
//!
//! **Unverified against a live client**: correct per the reference table, but the mapping to a
//! given client is only confirmed once a real one connects.

/// OS strings the modern client reports (`AuthSession`/bnet logon `platform`).
pub mod os {
    pub const WINDOWS_X64: &str = "Wn64";
    pub const MAC_INTEL: &str = "Mc64";
    pub const MAC_ARM: &str = "MacA";
}

/// One `(build, os) -> seed` row.
struct SeedRow {
    build: u32,
    os: &'static str,
    seed: [u8; 16],
}

/// The known 1.14.x seeds. Extend as more builds/platforms are supported.
static SEEDS: &[SeedRow] = &[
    SeedRow {
        build: 40618,
        os: os::WINDOWS_X64,
        seed: hexlit(*b"1278EB34F243ED7898D614C0E278EAC0"),
    },
    SeedRow {
        build: 40618,
        os: os::MAC_INTEL,
        seed: hexlit(*b"7528AB80D693E149907757BC9540A6A6"),
    },
    SeedRow {
        build: 41794,
        os: os::WINDOWS_X64,
        seed: hexlit(*b"91D3C1D62CD20FCCD4D0A71E051CE7CA"),
    },
    SeedRow {
        build: 42597,
        os: os::WINDOWS_X64,
        seed: hexlit(*b"2C76A6CDD32F651E940B5F682D8E15CE"),
    },
    SeedRow {
        build: 42597,
        os: os::MAC_ARM,
        seed: hexlit(*b"3B31A4F4C25382131A8FB95A1317412B"),
    },
];

/// The seed for a `(build, os)` pair, if known.
pub fn lookup(build: u32, os: &str) -> Option<[u8; 16]> {
    SEEDS
        .iter()
        .find(|row| row.build == build && row.os == os)
        .map(|row| row.seed)
}

/// Decode a 32-char uppercase-hex ASCII literal into 16 bytes, at compile time.
const fn hexlit(ascii: [u8; 32]) -> [u8; 16] {
    const fn nibble(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'F' => c - b'A' + 10,
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("invalid hex digit in auth seed literal"),
        }
    }
    let mut out = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = (nibble(ascii[i * 2]) << 4) | nibble(ascii[i * 2 + 1]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_windows_seed_decodes_correctly() {
        // 1.14.1 on Windows x64 (build 41794).
        let seed = lookup(41794, os::WINDOWS_X64).unwrap();
        assert_eq!(
            seed,
            [
                0x91, 0xD3, 0xC1, 0xD6, 0x2C, 0xD2, 0x0F, 0xCC, 0xD4, 0xD0, 0xA7, 0x1E, 0x05, 0x1C,
                0xE7, 0xCA
            ]
        );
    }

    #[test]
    fn unknown_build_or_os_returns_none() {
        assert!(lookup(99999, os::WINDOWS_X64).is_none());
        assert!(lookup(41794, "Xx99").is_none());
    }

    #[test]
    fn mac_arm_seed_is_distinct_from_windows() {
        assert_ne!(
            lookup(42597, os::WINDOWS_X64).unwrap(),
            lookup(42597, os::MAC_ARM).unwrap()
        );
    }
}
