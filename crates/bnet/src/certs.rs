//! Certificate bundle generation and signing.
//!
//! Modern clients pin the bnet TLS connection against a *certificate bundle* baked into the
//! executable. The bundle is a JSON document listing the SHA-256 of each trusted certificate's
//! `SubjectPublicKeyInfo`, followed by an RSA signature the client verifies against a 256-byte
//! modulus that is *also* baked into the executable.
//!
//! To connect a modern client to a server we control we therefore need to replace both pieces:
//!
//! - the **signature modulus** — so the client verifies bundles against *our* key, and
//! - the **bundle** — so it lists *our* TLS certificate as trusted.
//!
//! Both are produced together by [`generate`] from a single freshly minted signing key, and
//! written out as opaque files the patcher embeds into the client (see `crates/patcher`). The
//! server keeps the matching TLS certificate. Because both artifacts come from one generation
//! step, the modulus in the client and the signature on the bundle can never drift apart.
//!
//! ## Bundle schema
//!
//! The top-level keys and per-entry keys mirror Blizzard's format (as reproduced by
//! TrinityCore's connection patcher): `Created`, `Certificates`, `PublicKeys`,
//! `SigningCertificates`, each entry carrying a `Uri` and a `ShaHashPublicKeyInfo`.
//!
//! ## Unverified against a live client
//!
//! The bundle *structure* is confirmed, but the exact signature scheme the 1.14.x client
//! applies (hash + padding) is not verified end to end here — that needs a real client, which
//! is the documented stopping point for this work. The scheme lives in one place
//! ([`sign_bundle`]) so it can be corrected in isolation once observed. We sign with
//! RSA-2048 PKCS#1 v1.5 over SHA-256, which is the scheme these clients are documented to use.

use anyhow::{Context, Result};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::sha2::Sha256;
use rsa::RsaPrivateKey;
use serde::Serialize;
use sha2::Digest;

/// Bits in the bundle signing key. The client's modulus slot is 256 bytes, i.e. 2048 bits.
pub const SIGNING_KEY_BITS: usize = 2048;
/// Length of the RSA modulus and signature, in bytes.
pub const MODULUS_LEN: usize = SIGNING_KEY_BITS / 8;

/// One trusted certificate or key, identified by the hash of its public key info.
#[derive(Debug, Clone, Serialize)]
pub struct BundleEntry {
    /// URI pattern the entry applies to. `*.*` matches any host, which is what we want for a
    /// self-hosted server reachable under an arbitrary patched hostname.
    #[serde(rename = "Uri")]
    pub uri: String,
    /// Uppercase hex SHA-256 of the certificate's DER `SubjectPublicKeyInfo`.
    #[serde(rename = "ShaHashPublicKeyInfo")]
    pub sha_hash_public_key_info: String,
}

/// The certificate bundle, before signing.
#[derive(Debug, Clone, Serialize)]
pub struct CertBundle {
    #[serde(rename = "Created")]
    pub created: i64,
    #[serde(rename = "Certificates")]
    pub certificates: Vec<BundleEntry>,
    #[serde(rename = "PublicKeys")]
    pub public_keys: Vec<BundleEntry>,
    #[serde(rename = "SigningCertificates")]
    pub signing_certificates: Vec<BundleEntry>,
}

impl CertBundle {
    /// Build a bundle trusting a single TLS certificate, identified by the DER encoding of its
    /// `SubjectPublicKeyInfo` (what `rcgen`'s `KeyPair::public_key_der` returns, and what a
    /// server presents at handshake time).
    pub fn trusting(spki_der: &[u8], created: i64) -> Self {
        let entry = BundleEntry {
            uri: "*.*".to_string(),
            sha_hash_public_key_info: spki_sha256_hex(spki_der),
        };
        // Blizzard lists the pinned server certs under both Certificates and PublicKeys; the
        // client checks the presented cert's public-key hash against these, so listing our one
        // cert in both is enough. SigningCertificates is for a different chain we don't use.
        Self {
            created,
            certificates: vec![entry.clone()],
            public_keys: vec![entry],
            signing_certificates: Vec::new(),
        }
    }

    /// Canonical JSON bytes that get signed. Serialization must be deterministic, so this is
    /// the single serialization used for both signing and embedding.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).context("failed to serialize certificate bundle")
    }
}

/// Uppercase-hex SHA-256 of a DER `SubjectPublicKeyInfo`.
pub fn spki_sha256_hex(spki_der: &[u8]) -> String {
    let digest = sha2::Sha256::digest(spki_der);
    hex::encode_upper(digest)
}

/// A freshly generated bundle-signing key plus everything derived from it.
pub struct Artifacts {
    /// The signing key, PEM (PKCS#1). Keep this secret; it is not needed at runtime.
    pub signing_key_pem: String,
    /// The 256-byte big-endian RSA modulus to patch into the client.
    pub modulus: Vec<u8>,
    /// The signed bundle blob (`JSON || signature`) to embed into the client.
    pub signed_bundle: Vec<u8>,
}

/// Sign a bundle: `JSON || signature`, where the signature is RSA PKCS#1 v1.5 over SHA-256 of
/// the JSON. The signature is a fixed [`MODULUS_LEN`] bytes appended after the JSON.
pub fn sign_bundle(bundle: &CertBundle, key: &RsaPrivateKey) -> Result<Vec<u8>> {
    let json = bundle.to_json_bytes()?;

    let signing_key = rsa::pkcs1v15::SigningKey::<Sha256>::new(key.clone());
    let signature = signing_key.sign(&json);

    let mut blob = json;
    blob.extend_from_slice(&signature.to_bytes());
    Ok(blob)
}

/// The big-endian modulus of an RSA key, left-padded to [`MODULUS_LEN`].
pub fn modulus_bytes(key: &RsaPrivateKey) -> Vec<u8> {
    use rsa::traits::PublicKeyParts;
    let raw = key.n().to_bytes_be();
    // A proper 2048-bit key has its top bit set, so this is normally already 256 bytes; pad
    // defensively in case the high byte happened to be zero.
    if raw.len() >= MODULUS_LEN {
        raw
    } else {
        let mut padded = vec![0u8; MODULUS_LEN - raw.len()];
        padded.extend_from_slice(&raw);
        padded
    }
}

/// Generate a signing key and everything derived from it, for a server whose TLS certificate
/// has the given DER `SubjectPublicKeyInfo`.
pub fn generate(spki_der: &[u8], created: i64) -> Result<Artifacts> {
    let mut rng = rand::thread_rng();
    let key = RsaPrivateKey::new(&mut rng, SIGNING_KEY_BITS)
        .context("failed to generate bundle signing key")?;

    let bundle = CertBundle::trusting(spki_der, created);
    let signed_bundle = sign_bundle(&bundle, &key)?;
    let modulus = modulus_bytes(&key);

    let signing_key_pem = key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .context("failed to encode signing key")?
        .to_string();

    Ok(Artifacts {
        signing_key_pem,
        modulus,
        signed_bundle,
    })
}

/// A world-server RSA signing key plus the modulus the client must be patched with.
pub struct WorldSigningArtifacts {
    /// The RSA private key, PKCS#1 PEM. The world server signs `SMSG_ENTER_ENCRYPTED_MODE` /
    /// `SMSG_CONNECT_TO` with this; keep it with the world server config.
    pub signing_key_pem: String,
    /// The 256-byte **little-endian** RSA modulus to patch into the client's connect-to slot. The
    /// world server reverses its signatures to match this byte order.
    pub modulus_le: Vec<u8>,
}

/// Generate the RSA keypair the modern world server uses to sign its encrypted-mode / connect-to
/// messages, returning the private key and the little-endian modulus for the patcher. This is a
/// *separate* key from the certificate-bundle signing key: it lands in a different slot in the
/// client and is verified by different code.
pub fn generate_world_signing_key() -> Result<WorldSigningArtifacts> {
    let mut rng = rand::thread_rng();
    let key = RsaPrivateKey::new(&mut rng, SIGNING_KEY_BITS)
        .context("failed to generate world signing key")?;

    // The client stores this modulus little-endian (it reverses signatures), so ship it reversed
    // from the big-endian encoding.
    let mut modulus_le = modulus_bytes(&key);
    modulus_le.reverse();

    let signing_key_pem = key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .context("failed to encode world signing key")?
        .to_string();

    Ok(WorldSigningArtifacts {
        signing_key_pem,
        modulus_le,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::signature::Verifier;
    use rsa::RsaPublicKey;

    fn sample_spki() -> Vec<u8> {
        // Any bytes work as a stand-in for a DER SPKI here; the hash is over raw bytes.
        b"a-sample-subject-public-key-info".to_vec()
    }

    #[test]
    fn spki_hash_is_64_uppercase_hex_chars() {
        let hash = spki_sha256_hex(&sample_spki());
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
    }

    #[test]
    fn bundle_serializes_with_blizzard_field_names() {
        let bundle = CertBundle::trusting(&sample_spki(), 1_600_000_000);
        let json: serde_json::Value = serde_json::from_slice(&bundle.to_json_bytes().unwrap()).unwrap();

        assert_eq!(json["Created"], 1_600_000_000);
        assert_eq!(json["Certificates"][0]["Uri"], "*.*");
        assert_eq!(json["Certificates"][0]["ShaHashPublicKeyInfo"].as_str().unwrap().len(), 64);
        assert!(json["PublicKeys"].is_array());
        assert!(json["SigningCertificates"].is_array());
    }

    #[test]
    fn signed_bundle_is_json_followed_by_a_verifiable_signature() {
        let mut rng = rand::thread_rng();
        // 1024 bits keeps the test fast; the scheme is identical at 2048.
        let key = RsaPrivateKey::new(&mut rng, 1024).unwrap();

        let bundle = CertBundle::trusting(&sample_spki(), 42);
        let json = bundle.to_json_bytes().unwrap();
        let blob = sign_bundle(&bundle, &key).unwrap();

        let sig_len = 1024 / 8;
        assert_eq!(blob.len(), json.len() + sig_len);
        assert_eq!(&blob[..json.len()], &json[..]);

        // The appended signature verifies against the public key over the JSON.
        let verifying = VerifyingKey::<Sha256>::new(RsaPublicKey::from(&key));
        let signature = rsa::pkcs1v15::Signature::try_from(&blob[json.len()..]).unwrap();
        verifying
            .verify(&json, &signature)
            .expect("appended signature must verify");
    }

    #[test]
    fn generated_modulus_is_256_bytes() {
        let artifacts = generate(&sample_spki(), 1).unwrap();
        assert_eq!(artifacts.modulus.len(), MODULUS_LEN);
        assert!(artifacts.signing_key_pem.contains("RSA PRIVATE KEY"));
        assert!(artifacts.signed_bundle.len() > MODULUS_LEN);
    }

    #[test]
    fn world_signing_key_modulus_is_256_bytes_and_little_endian() {
        let artifacts = generate_world_signing_key().unwrap();
        assert_eq!(artifacts.modulus_le.len(), MODULUS_LEN);
        assert!(artifacts.signing_key_pem.contains("RSA PRIVATE KEY"));

        // Little-endian: reversing recovers a big-endian modulus whose top byte is non-zero (a
        // proper 2048-bit key has its high bit set).
        let mut big_endian = artifacts.modulus_le.clone();
        big_endian.reverse();
        assert_ne!(big_endian[0], 0);
    }
}
