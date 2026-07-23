//! The individual patches applied to the client executable.
//!
//! Every patch is length-preserving. The executable's headers, relocations and any embedded
//! signatures all assume fixed offsets, so nothing may grow or shrink the file — replacements
//! that are shorter than the region they overwrite are NUL-padded instead.

use anyhow::{bail, Context, Result};

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

/// Replace the RSA modulus the client uses to verify the modern world server's signatures
/// (`SMSG_ENTER_ENCRYPTED_MODE` and `SMSG_CONNECT_TO`).
///
/// Without this the client rejects the server's request to switch to encrypted mode. `modulus`
/// must be exactly [`patterns::MODULUS_LEN`] bytes, in the byte order the client stores it — the
/// world server reverses its signatures to match, so this is the little-endian modulus produced by
/// `bnet gen-certs` (`connect_to_modulus.bin`). Paired with the world server's private key.
pub fn connect_to_modulus(data: &[u8], modulus: &[u8]) -> Result<Patch> {
    if modulus.len() != patterns::MODULUS_LEN {
        bail!(
            "connect-to modulus must be exactly {} bytes, got {}",
            patterns::MODULUS_LEN,
            modulus.len()
        );
    }

    let offset = find_unique(data, patterns::CONNECT_TO_MODULUS, "connect-to modulus")?;

    // As with the signature modulus, the searched prefix is the start of the 256-byte modulus.
    if offset + patterns::MODULUS_LEN > data.len() {
        bail!("connect-to modulus at 0x{offset:08x} runs past the end of the file");
    }

    Ok(Patch {
        name: "connect-to modulus",
        offset,
        bytes: modulus.to_vec(),
    })
}

/// Replace the embedded certificate bundle with our signed one.
///
/// The client stores the bundle as a JSON document (`{"Created":...}`) immediately followed by
/// its RSA signature, occupying a fixed region of the executable. We locate that region by its
/// `{"Created":` marker and by brace-matching the JSON, then overwrite it with our own
/// `JSON || signature` blob, NUL-padding the remainder.
///
/// Blizzard's original bundle lists many certificates and is far larger than ours, so our blob
/// virtually always fits; if it somehow does not, this errors rather than overrun into
/// neighbouring data.
pub fn cert_bundle(data: &[u8], bundle_blob: &[u8]) -> Result<Patch> {
    let offset = find_unique(data, patterns::CERT_BUNDLE, "cert bundle")?;

    let json_len = json_object_len(&data[offset..]).context(
        "could not find the end of the embedded certificate bundle JSON — the file may be \
         compressed or the format has changed",
    )?;

    // The original region is the JSON, the 4-byte "NGIS" magic, then the 256-byte signature; that
    // whole span is ours to overwrite.
    let region_len = json_len + 4 + patterns::MODULUS_LEN;
    if offset + region_len > data.len() {
        bail!("certificate bundle at 0x{offset:08x} runs past the end of the file");
    }

    let bytes = replace_padded(offset, bundle_blob, region_len)?;

    Ok(Patch {
        name: "cert bundle",
        offset,
        bytes,
    })
}

/// Length of the JSON object beginning at `data[0]`, found by brace matching.
///
/// The bundle's string values are hex hashes and base64 PEM, none of which contain braces, so
/// a plain depth counter is sufficient and avoids a JSON parser over a partially-binary buffer.
/// Returns `None` if the braces never balance before the buffer ends.
fn json_object_len(data: &[u8]) -> Option<usize> {
    if data.first() != Some(&b'{') {
        return None;
    }

    let mut depth = 0u32;
    for (i, &byte) in data.iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
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
    /// A large stand-in for Blizzard's original embedded bundle: a JSON object followed by a
    /// 256-byte signature. Deliberately bigger than anything `gen-certs` produces.
    fn original_bundle_json() -> &'static [u8] {
        br#"{"Created":1600000000,"Certificates":[{"Uri":"*.*","ShaHashPublicKeyInfo":"AA"},{"Uri":"*.*","ShaHashPublicKeyInfo":"BB"}],"PublicKeys":[],"SigningCertificates":[]}"#
    }

    fn fixture() -> Vec<u8> {
        let mut data = vec![0xCC; 64];
        data.extend_from_slice(patterns::PORTAL);
        data.extend_from_slice(&[0xCC; 64]);
        data.extend_from_slice(patterns::SIGNATURE_MODULUS);
        // The remaining 248 bytes of the 256-byte modulus.
        data.extend_from_slice(&[0xAB; patterns::MODULUS_LEN - patterns::SIGNATURE_MODULUS.len()]);
        data.extend_from_slice(&[0xCC; 64]);
        // Connect-to modulus: its 8-byte prefix then the remaining 248 bytes.
        data.extend_from_slice(patterns::CONNECT_TO_MODULUS);
        data.extend_from_slice(
            &[0xCD; patterns::MODULUS_LEN - patterns::CONNECT_TO_MODULUS.len()],
        );
        data.extend_from_slice(&[0xCC; 64]);
        // Embedded certificate bundle: JSON, the "NGIS" magic, then its 256-byte signature.
        data.extend_from_slice(original_bundle_json());
        data.extend_from_slice(b"NGIS");
        data.extend_from_slice(&[0x99; patterns::MODULUS_LEN]);
        // Neighbouring data that must survive patching.
        data.extend_from_slice(&[0x77; 32]);
        data
    }

    fn connect_to_offset() -> usize {
        64 + patterns::PORTAL.len() + 64 + patterns::MODULUS_LEN + 64
    }

    /// A minimal signed bundle blob: small JSON + a 256-byte signature.
    fn small_bundle_blob() -> Vec<u8> {
        let mut blob = br#"{"Created":1,"Certificates":[],"PublicKeys":[],"SigningCertificates":[]}"#.to_vec();
        blob.extend_from_slice(&[0x42; patterns::MODULUS_LEN]);
        blob
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
    fn connect_to_patch_rewrites_the_full_modulus() {
        let data = fixture();
        let modulus = vec![0x24; patterns::MODULUS_LEN];
        let patch = connect_to_modulus(&data, &modulus).unwrap();

        assert_eq!(patch.offset, connect_to_offset());
        assert_eq!(patch.bytes.len(), patterns::MODULUS_LEN);
        assert!(patch.bytes.iter().all(|&b| b == 0x24));
    }

    #[test]
    fn connect_to_patch_rejects_a_wrong_sized_modulus() {
        let data = fixture();
        let err = connect_to_modulus(&data, &[0x24; 200]).unwrap_err();
        assert!(err.to_string().contains("exactly 256 bytes"));
    }

    #[test]
    fn connect_to_and_signature_moduli_land_at_distinct_offsets() {
        let data = fixture();
        let sig = signature_modulus(&data, &[0x42; patterns::MODULUS_LEN]).unwrap();
        let con = connect_to_modulus(&data, &[0x24; patterns::MODULUS_LEN]).unwrap();
        assert_ne!(sig.offset, con.offset);
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

    #[test]
    fn json_object_len_matches_the_balanced_brace() {
        let json = original_bundle_json();
        // The whole fixture region begins with the JSON, so measuring from there must return
        // exactly the JSON length regardless of what follows.
        let mut buf = json.to_vec();
        buf.extend_from_slice(&[0x99; 300]);
        assert_eq!(json_object_len(&buf), Some(json.len()));
    }

    #[test]
    fn cert_bundle_patch_covers_json_plus_signature_and_is_nul_padded() {
        let data = fixture();
        let blob = small_bundle_blob();
        let patch = cert_bundle(&data, &blob).unwrap();

        let region_len = original_bundle_json().len() + 4 + patterns::MODULUS_LEN;
        assert_eq!(patch.bytes.len(), region_len);
        assert_eq!(&patch.bytes[..blob.len()], &blob[..]);
        assert!(patch.bytes[blob.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn cert_bundle_patch_preserves_neighbouring_bytes() {
        let mut data = fixture();
        let original_len = data.len();
        let trailing = &data[data.len() - 32..].to_vec();

        let patch = cert_bundle(&data, &small_bundle_blob()).unwrap();
        apply(&mut data, &[patch]).unwrap();

        assert_eq!(data.len(), original_len);
        // The 32 bytes of neighbour data past the bundle region are untouched.
        assert_eq!(&data[data.len() - 32..], &trailing[..]);
    }

    #[test]
    fn cert_bundle_patch_rejects_a_blob_bigger_than_the_region() {
        let data = fixture();
        let region_len = original_bundle_json().len() + 4 + patterns::MODULUS_LEN;
        let oversized = vec![0x42; region_len + 1];

        let err = cert_bundle(&data, &oversized).unwrap_err();
        assert!(err.to_string().contains("use a shorter value"));
    }
}
