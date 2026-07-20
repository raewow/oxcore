//! The individual patches applied to the client executable.
//!
//! Every patch is length-preserving. The executable's headers, relocations and any embedded
//! signatures all assume fixed offsets, so nothing may grow or shrink the file — replacements
//! that are shorter than the region they overwrite are NUL-padded instead.

use anyhow::{bail, Result};

use crate::patterns;
use crate::scan::find_unique;

/// A single planned modification, resolved but not yet applied.
#[derive(Debug, Clone)]
pub struct Patch {
    pub name: &'static str,
    pub offset: usize,
    pub bytes: Vec<u8>,
}

impl Patch {
    /// Human-readable one-liner for `--dry-run`.
    pub fn describe(&self) -> String {
        format!(
            "{:<20} offset 0x{:08x}  {} bytes",
            self.name,
            self.offset,
            self.bytes.len()
        )
    }
}

/// Overwrite `data[offset..]` with `bytes`, NUL-padding out to `region_len`.
///
/// `region_len` is the size of the thing being replaced — the original string or modulus — and
/// bounds how much we are allowed to touch.
fn replace_padded(offset: usize, replacement: &[u8], region_len: usize) -> Result<Vec<u8>> {
    if replacement.len() > region_len {
        bail!(
            "replacement is {} bytes but only {} are available at offset 0x{:08x}; \
             use a shorter value",
            replacement.len(),
            region_len,
            offset
        );
    }

    let mut bytes = replacement.to_vec();
    bytes.resize(region_len, 0);
    Ok(bytes)
}

/// Redirect the login portal by replacing `.actual.battle.net` with `suffix`.
///
/// The client concatenates the `portal` value from `WTF/Config.wtf` with this suffix, so
/// `SET portal "myserver"` plus a suffix of `.localhost` resolves `myserver.localhost`.
pub fn portal(data: &[u8], suffix: &str) -> Result<Patch> {
    if !suffix.starts_with('.') {
        bail!("portal suffix must start with a dot, got '{suffix}'");
    }

    let offset = find_unique(data, patterns::PORTAL, "portal")?;
    let bytes = replace_padded(offset, suffix.as_bytes(), patterns::PORTAL.len())?;

    Ok(Patch {
        name: "portal",
        offset,
        bytes,
    })
}

/// Replace the RSA modulus the client uses to verify the certificate bundle's signature.
///
/// This is what lets us sign our own bundle. `modulus` must be exactly
/// [`patterns::MODULUS_LEN`] bytes, in the same byte order the client stores it.
pub fn signature_modulus(data: &[u8], modulus: &[u8]) -> Result<Patch> {
    if modulus.len() != patterns::MODULUS_LEN {
        bail!(
            "signature modulus must be exactly {} bytes, got {}",
            patterns::MODULUS_LEN,
            modulus.len()
        );
    }

    let offset = find_unique(data, patterns::SIGNATURE_MODULUS, "signature modulus")?;

    // The prefix we searched for is the start of the modulus itself, so the whole 256-byte
    // region begins at the match offset.
    if offset + patterns::MODULUS_LEN > data.len() {
        bail!("signature modulus at 0x{offset:08x} runs past the end of the file");
    }

    Ok(Patch {
        name: "signature modulus",
        offset,
        bytes: modulus.to_vec(),
    })
}

/// Apply patches in place. Offsets are absolute file offsets.
pub fn apply(data: &mut [u8], patches: &[Patch]) -> Result<()> {
    for patch in patches {
        let end = patch.offset + patch.bytes.len();
        if end > data.len() {
            bail!(
                "patch '{}' at 0x{:08x} runs past the end of the file",
                patch.name,
                patch.offset
            );
        }
        data[patch.offset..end].copy_from_slice(&patch.bytes);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic executable containing each pattern exactly once at a known offset.
    fn fixture() -> Vec<u8> {
        let mut data = vec![0xCC; 64];
        data.extend_from_slice(patterns::PORTAL);
        data.extend_from_slice(&[0xCC; 64]);
        data.extend_from_slice(patterns::SIGNATURE_MODULUS);
        // The remaining 248 bytes of the 256-byte modulus.
        data.extend_from_slice(&[0xAB; patterns::MODULUS_LEN - patterns::SIGNATURE_MODULUS.len()]);
        data.extend_from_slice(&[0xCC; 64]);
        data
    }

    #[test]
    fn portal_patch_is_length_preserving_and_nul_padded() {
        let data = fixture();
        let patch = portal(&data, ".localhost").unwrap();

        assert_eq!(patch.offset, 64);
        assert_eq!(patch.bytes.len(), patterns::PORTAL.len());
        assert_eq!(&patch.bytes[..10], b".localhost");
        assert!(patch.bytes[10..].iter().all(|&b| b == 0));
    }

    #[test]
    fn portal_patch_rejects_a_suffix_that_does_not_fit() {
        let data = fixture();
        let err = portal(&data, ".a-very-long-hostname-suffix.example").unwrap_err();
        assert!(err.to_string().contains("only 18 are available"));
    }

    #[test]
    fn portal_patch_requires_a_leading_dot() {
        let data = fixture();
        assert!(portal(&data, "localhost").is_err());
    }

    #[test]
    fn signature_patch_rewrites_the_full_modulus() {
        let data = fixture();
        let modulus = vec![0x42; patterns::MODULUS_LEN];
        let patch = signature_modulus(&data, &modulus).unwrap();

        assert_eq!(patch.offset, 64 + patterns::PORTAL.len() + 64);
        assert_eq!(patch.bytes.len(), patterns::MODULUS_LEN);
    }

    #[test]
    fn signature_patch_rejects_a_wrong_sized_modulus() {
        let data = fixture();
        let err = signature_modulus(&data, &[0x42; 128]).unwrap_err();
        assert!(err.to_string().contains("exactly 256 bytes"));
    }

    #[test]
    fn apply_preserves_file_length_and_writes_expected_bytes() {
        let mut data = fixture();
        let original_len = data.len();

        let patches = vec![
            portal(&data, ".localhost").unwrap(),
            signature_modulus(&data, &[0x42; patterns::MODULUS_LEN]).unwrap(),
        ];
        apply(&mut data, &patches).unwrap();

        assert_eq!(data.len(), original_len);
        assert_eq!(&data[64..74], b".localhost");
        assert!(data[74..64 + patterns::PORTAL.len()].iter().all(|&b| b == 0));

        let mod_offset = 64 + patterns::PORTAL.len() + 64;
        assert!(data[mod_offset..mod_offset + patterns::MODULUS_LEN]
            .iter()
            .all(|&b| b == 0x42));
    }

    #[test]
    fn a_twice_patched_file_is_rejected_rather_than_double_patched() {
        let mut data = fixture();
        let patch = portal(&data, ".localhost").unwrap();
        apply(&mut data, &[patch]).unwrap();

        // The signature is gone after the first pass, so a second run must refuse.
        let err = portal(&data, ".localhost").unwrap_err();
        assert!(err.to_string().contains("already been patched"));
    }

    #[test]
    fn ambiguous_pattern_is_rejected() {
        let mut data = fixture();
        data.extend_from_slice(patterns::PORTAL);

        let err = portal(&data, ".localhost").unwrap_err();
        assert!(err.to_string().contains("matched 2 times"));
    }
}
