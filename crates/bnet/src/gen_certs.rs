//! `bnet gen-certs` — mint the TLS certificate and the client-patch artifacts together.
//!
//! Produces six files in the output directory:
//!
//! | File | Used by | Purpose |
//! |------|---------|---------|
//! | `bnet.cert.pem` | server | TLS certificate served on both ports |
//! | `bnet.key.pem` | server | TLS private key |
//! | `signature_modulus.bin` | patcher `--modulus` | replaces the client's bundle-verify modulus |
//! | `cert_bundle.bin` | patcher `--cert-bundle` | the signed bundle trusting `bnet.cert.pem` |
//! | `connect_to_modulus.bin` | patcher `--connect-to-modulus` | replaces the client's world-signature modulus |
//! | `world.signing.key.pem` | world server | RSA key that signs `SMSG_ENTER_ENCRYPTED_MODE` / `SMSG_CONNECT_TO` |
//!
//! The bundle signing key is deliberately *not* written: it is used once, here, and never needed
//! again. The **world** signing key, by contrast, *is* written — the world server needs it at
//! runtime to sign encrypted-mode/connect-to messages, paired with `connect_to_modulus.bin` in the
//! client. Regenerating replaces every artifact as a set, so the server certs, the embedded moduli
//! and the embedded bundle always match.

use std::path::Path;

use anyhow::{Context, Result};

/// Generate the TLS certificate and patch artifacts for `hostnames`, writing into `out_dir`.
pub fn run(out_dir: &Path, hostnames: &[String]) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create output directory {}", out_dir.display()))?;

    // The TLS certificate the server presents. Its public key is what the client pins.
    let tls = rcgen::generate_simple_self_signed(hostnames.to_vec())
        .context("failed to generate TLS certificate")?;

    let spki_der = tls.key_pair.public_key_der();
    let created = chrono::Utc::now().timestamp();
    let artifacts = crate::certs::generate(&spki_der, created)?;

    // The modern world server's signing key (separate from the bundle key) and the matching
    // little-endian modulus the client is patched with.
    let world = crate::certs::generate_world_signing_key()?;

    write(out_dir, "bnet.cert.pem", tls.cert.pem().as_bytes())?;
    write(out_dir, "bnet.key.pem", tls.key_pair.serialize_pem().as_bytes())?;
    write(out_dir, "signature_modulus.bin", &artifacts.modulus)?;
    write(out_dir, "cert_bundle.bin", &artifacts.signed_bundle)?;
    write(out_dir, "connect_to_modulus.bin", &world.modulus_le)?;
    write(out_dir, "world.signing.key.pem", world.signing_key_pem.as_bytes())?;

    eprintln!("generated certificate and patch artifacts in {}", out_dir.display());
    eprintln!("  bnet server: point cert_file/key_file at bnet.cert.pem / bnet.key.pem");
    eprintln!("  world server: point its modern signing key at world.signing.key.pem");
    eprintln!(
        "  patcher: oxcore-patcher -i Wow.exe \\\n           \
         --modulus {out}/signature_modulus.bin \\\n           \
         --cert-bundle {out}/cert_bundle.bin \\\n           \
         --connect-to-modulus {out}/connect_to_modulus.bin",
        out = out_dir.display(),
    );
    Ok(())
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
