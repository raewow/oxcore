//! Certificate bundle generation and signing — the **verified** Blizzard format.
//!
//! Rewritten to match the format `wowemulation-dev/wow-patcher` documents and verifies against
//! real 1.14.x / 2.5.3 clients (`scripts/gen-cert-bundle.py`). The earlier version got five things
//! wrong (direct-cert pinning, missing `RootCAPublicKeys`/`SigningCertificates.RawData`, no
//! `"Blizzard Certificate Bundle"` sign context, no `NGIS` magic, big-endian modulus); all are
//! corrected here.
//!
//! ## Trust architecture (two keys, CA-based)
//!
//! * **TLS CA** — a self-signed root. The bnet server presents a **leaf** cert signed by it; the
//!   client trusts the leaf because the CA's SPKI SHA-256 is listed in the bundle's
//!   `RootCAPublicKeys`. The CA is *not* patched into the client; it rides inside the bundle JSON
//!   as `SigningCertificates[0].RawData`.
//! * **Bundle-signing key** (RSA-2048) — signs the bundle JSON. Its **little-endian** modulus is
//!   patched into the client, which verifies the bundle's signature against it.
//!
//! ## Bundle wire format
//!
//! ```text
//! [ compact JSON (UTF-8) ][ "NGIS" (4 bytes) ][ 256-byte signature, byte-reversed ]
//! ```
//!
//! where the signature is RSA-2048 PKCS#1 v1.5 over `SHA-256(JSON ‖ "Blizzard Certificate
//! Bundle")`, reversed to little-endian (Blizzard's BigNumber representation).
//!
//! ## Still unverified: 1.15.x
//!
//! This is the **RSA / embedded-or-nydus** format used through ~2.5.3 and 1.14.x. The live 1.15.x
//! client dropped the RSA modulus entirely and verifies a *downloaded* bundle with **Ed25519** —
//! a format not documented or implemented anywhere public yet. That path is not handled here.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use anyhow::{Context, Result};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose, SanType};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::RsaPrivateKey;
use serde::Serialize;
use sha2::Digest;

/// Bits in the bundle signing key. The client's modulus slot is 256 bytes, i.e. 2048 bits.
pub const SIGNING_KEY_BITS: usize = 2048;
/// Length of the RSA modulus and signature, in bytes.
pub const MODULUS_LEN: usize = SIGNING_KEY_BITS / 8;

/// The context string appended to the JSON before hashing/signing.
const SIGN_CONTEXT: &[u8] = b"Blizzard Certificate Bundle";
/// Magic marking the start of the signature after the JSON.
const NGIS_MAGIC: &[u8] = b"NGIS";
/// Hostname pattern the bundle pins. `*.*` matches every host the client may dial, matching
/// Arctium's and wow-patcher's approach.
const PINNED_URI: &str = "*.*";

/// One `Certificates`/`PublicKeys` entry: a host pattern and the trusted CA's SPKI hash.
#[derive(Debug, Clone, Serialize)]
struct BundleEntry {
    #[serde(rename = "Uri")]
    uri: String,
    #[serde(rename = "ShaHashPublicKeyInfo")]
    sha_hash_public_key_info: String,
}

/// The `SigningCertificates` entry carrying the CA certificate itself.
#[derive(Debug, Clone, Serialize)]
struct SigningCertificate {
    #[serde(rename = "RawData")]
    raw_data: String,
}

/// The bundle JSON, in Blizzard's field order (the signature covers these exact bytes).
#[derive(Debug, Clone, Serialize)]
struct CertBundle {
    #[serde(rename = "Created")]
    created: i64,
    #[serde(rename = "Certificates")]
    certificates: Vec<BundleEntry>,
    #[serde(rename = "PublicKeys")]
    public_keys: Vec<BundleEntry>,
    #[serde(rename = "SigningCertificates")]
    signing_certificates: Vec<SigningCertificate>,
    #[serde(rename = "RootCAPublicKeys")]
    root_ca_public_keys: Vec<String>,
}

/// Uppercase-hex SHA-256 of a DER `SubjectPublicKeyInfo`.
pub fn spki_sha256_hex(spki_der: &[u8]) -> String {
    hex::encode_upper(sha2::Sha256::digest(spki_der))
}

/// Everything a server + patcher need, all minted together so nothing can drift.
pub struct Artifacts {
    /// The self-signed CA certificate, PEM.
    pub ca_pem: String,
    /// The TLS chain the bnet server presents (leaf ‖ CA), PEM.
    pub leaf_chain_pem: String,
    /// The leaf certificate's private key, PEM.
    pub leaf_key_pem: String,
    /// The bundle-signing key, PKCS#1 PEM. Secret; not needed at runtime.
    pub signing_key_pem: String,
    /// The 256-byte **little-endian** RSA modulus to patch into the client.
    pub modulus_le: Vec<u8>,
    /// The signed bundle blob (`JSON ‖ NGIS ‖ reversed-signature`) to embed or serve.
    pub signed_bundle: Vec<u8>,
}

/// Build the bundle JSON bytes trusting a CA identified by its PEM and SPKI hash.
fn build_bundle_json(ca_pem: &str, ca_spki_hash: &str, created: i64) -> Result<Vec<u8>> {
    // RawData stores the CA PEM as one line.
    let raw_data: String = ca_pem.chars().filter(|&c| c != '\n' && c != '\r').collect();
    let entry = BundleEntry {
        uri: PINNED_URI.to_string(),
        sha_hash_public_key_info: ca_spki_hash.to_string(),
    };
    let bundle = CertBundle {
        created,
        certificates: vec![entry.clone()],
        public_keys: vec![entry],
        signing_certificates: vec![SigningCertificate { raw_data }],
        root_ca_public_keys: vec![ca_spki_hash.to_string()],
    };
    // Compact separators (no spaces), matching `json.dumps(separators=(",", ":"))`.
    serde_json::to_vec(&bundle).context("failed to serialize certificate bundle")
}

/// Sign the JSON and assemble the blob: `JSON ‖ "NGIS" ‖ reverse(sig)`, where
/// `sig = RSA-2048 PKCS#1 v1.5 over SHA-256(JSON ‖ "Blizzard Certificate Bundle")`.
pub fn sign_bundle(json_bytes: &[u8], key: &RsaPrivateKey) -> Result<Vec<u8>> {
    let signing_key = rsa::pkcs1v15::SigningKey::<Sha256>::new(key.clone());

    let mut message = json_bytes.to_vec();
    message.extend_from_slice(SIGN_CONTEXT);
    let signature = signing_key.sign(&message);
    let mut sig_bytes = signature.to_bytes().to_vec(); // big-endian
    anyhow::ensure!(
        sig_bytes.len() == MODULUS_LEN,
        "signature length {} != {MODULUS_LEN}",
        sig_bytes.len()
    );
    sig_bytes.reverse(); // little-endian, matching the client's BigNumber

    let mut blob = json_bytes.to_vec();
    blob.extend_from_slice(NGIS_MAGIC);
    blob.extend_from_slice(&sig_bytes);
    Ok(blob)
}

/// The big-endian modulus of an RSA key, left-padded to [`MODULUS_LEN`].
pub fn modulus_bytes(key: &RsaPrivateKey) -> Vec<u8> {
    use rsa::traits::PublicKeyParts;
    let raw = key.n().to_bytes_be();
    if raw.len() >= MODULUS_LEN {
        raw
    } else {
        let mut padded = vec![0u8; MODULUS_LEN - raw.len()];
        padded.extend_from_slice(&raw);
        padded
    }
}

/// Generate the full trust set for a server whose leaf certificate covers `leaf_dns_names`.
pub fn generate(leaf_dns_names: &[String], created: i64) -> Result<Artifacts> {
    // --- TLS CA (self-signed root) ---
    let ca_key = KeyPair::generate().context("failed to generate CA key")?;
    let mut ca_params =
        CertificateParams::new(Vec::<String>::new()).context("failed to build CA params")?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "oxcore Battle.net Root CA");
    ca_params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let ca_cert = ca_params
        .self_signed(&ca_key)
        .context("failed to self-sign CA")?;
    let ca_pem = ca_cert.pem();
    let ca_spki_hash = spki_sha256_hex(ca_key.public_key_der().as_ref());

    // --- Leaf (server) cert signed by the CA ---
    let leaf_key = KeyPair::generate().context("failed to generate leaf key")?;
    let mut leaf_params =
        CertificateParams::new(leaf_dns_names.to_vec()).context("failed to build leaf params")?;
    // Always cover loopback so a localhost test works regardless of the --host list.
    leaf_params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    leaf_params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    if let Some(first) = leaf_dns_names.first() {
        leaf_params
            .distinguished_name
            .push(DnType::CommonName, first);
    }
    let leaf_cert = leaf_params
        .signed_by(&leaf_key, &ca_cert, &ca_key)
        .context("failed to sign leaf with CA")?;
    let leaf_chain_pem = format!("{}{}", leaf_cert.pem(), ca_pem);
    let leaf_key_pem = leaf_key.serialize_pem();

    // --- Bundle-signing key + signed bundle + LE modulus ---
    let mut rng = rand::thread_rng();
    let signing_key =
        RsaPrivateKey::new(&mut rng, SIGNING_KEY_BITS).context("failed to generate signing key")?;
    let json_bytes = build_bundle_json(&ca_pem, &ca_spki_hash, created)?;
    let signed_bundle = sign_bundle(&json_bytes, &signing_key)?;

    let mut modulus_le = modulus_bytes(&signing_key);
    modulus_le.reverse();

    let signing_key_pem = signing_key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .context("failed to encode signing key")?
        .to_string();

    Ok(Artifacts {
        ca_pem,
        leaf_chain_pem,
        leaf_key_pem,
        signing_key_pem,
        modulus_le,
        signed_bundle,
    })
}

/// A world-server RSA signing key plus the modulus the client must be patched with.
pub struct WorldSigningArtifacts {
    pub signing_key_pem: String,
    /// 256-byte little-endian modulus for the patcher.
    pub modulus_le: Vec<u8>,
}

/// Generate the RSA keypair the modern world server uses to sign its encrypted-mode / connect-to
/// messages (separate from the bundle-signing key).
pub fn generate_world_signing_key() -> Result<WorldSigningArtifacts> {
    let mut rng = rand::thread_rng();
    let key = RsaPrivateKey::new(&mut rng, SIGNING_KEY_BITS)
        .context("failed to generate world signing key")?;
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

    fn sample_hosts() -> Vec<String> {
        vec!["oxcore.localhost".to_string()]
    }

    #[test]
    fn spki_hash_is_64_uppercase_hex_chars() {
        let hash = spki_sha256_hex(b"a-sample-subject-public-key-info");
        assert_eq!(hash.len(), 64);
        assert!(hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
    }

    #[test]
    fn bundle_json_has_the_verified_schema() {
        let json = build_bundle_json(
            "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n",
            "DEADBEEF",
            42,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();

        assert_eq!(value["Created"], 42);
        assert_eq!(value["Certificates"][0]["Uri"], "*.*");
        assert_eq!(value["Certificates"][0]["ShaHashPublicKeyInfo"], "DEADBEEF");
        assert_eq!(value["RootCAPublicKeys"][0], "DEADBEEF");
        // RawData is the CA PEM with newlines stripped.
        let raw = value["SigningCertificates"][0]["RawData"].as_str().unwrap();
        assert!(!raw.contains('\n'));
        assert!(raw.starts_with("-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn bundle_json_is_compact_no_spaces() {
        let json = build_bundle_json("X", "H", 1).unwrap();
        let text = String::from_utf8(json).unwrap();
        assert!(!text.contains(", "));
        assert!(!text.contains(": "));
    }

    #[test]
    fn signed_bundle_has_ngis_magic_and_a_verifiable_reversed_signature() {
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let json = build_bundle_json("CAPEM", "HASH", 7).unwrap();
        let blob = sign_bundle(&json, &key).unwrap();

        // Layout: JSON, then NGIS, then 256-byte signature.
        assert_eq!(blob.len(), json.len() + 4 + MODULUS_LEN);
        assert_eq!(&blob[json.len()..json.len() + 4], NGIS_MAGIC);

        // Un-reverse the signature and verify it over SHA-256(JSON ‖ context).
        let mut sig_be = blob[json.len() + 4..].to_vec();
        sig_be.reverse();
        let mut message = json.clone();
        message.extend_from_slice(SIGN_CONTEXT);

        let verifying = VerifyingKey::<Sha256>::new(RsaPublicKey::from(&key));
        let signature = rsa::pkcs1v15::Signature::try_from(sig_be.as_slice()).unwrap();
        verifying
            .verify(&message, &signature)
            .expect("reversed signature must verify once un-reversed, over JSON ‖ context");
    }

    #[test]
    fn generate_produces_a_ca_backed_chain_and_le_modulus() {
        let artifacts = generate(&sample_hosts(), 1).unwrap();

        assert!(artifacts.ca_pem.contains("BEGIN CERTIFICATE"));
        // The chain is leaf then CA (two certs).
        assert_eq!(
            artifacts
                .leaf_chain_pem
                .matches("BEGIN CERTIFICATE")
                .count(),
            2
        );
        assert!(artifacts.leaf_key_pem.contains("PRIVATE KEY"));
        assert!(artifacts.signing_key_pem.contains("RSA PRIVATE KEY"));

        assert_eq!(artifacts.modulus_le.len(), MODULUS_LEN);
        // Little-endian: reversing recovers a BE modulus with a non-zero top byte.
        let mut be = artifacts.modulus_le.clone();
        be.reverse();
        assert_ne!(be[0], 0);

        // The bundle carries the CA's SPKI hash in RootCAPublicKeys.
        let json_end = artifacts
            .signed_bundle
            .windows(4)
            .position(|w| w == NGIS_MAGIC)
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&artifacts.signed_bundle[..json_end]).unwrap();
        assert_eq!(value["RootCAPublicKeys"][0].as_str().unwrap().len(), 64);
    }
}
