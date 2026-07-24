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

    // Generate the CA + leaf TLS cert, the bundle-signing key, and the signed bundle as one set.
    // The server presents the leaf chain; the client trusts it via the CA hash in the bundle.
    let created = chrono::Utc::now().timestamp();
    let artifacts = crate::certs::generate(hostnames, created)?;

    // ONE RSA key signs the bundle AND (later) the modern world messages. The 1.14.x client
    // verifies the cert bundle against the ConnectTo-modulus location (per wow-patcher's RSA
    // patch), so both `signature_modulus.bin` and `connect_to_modulus.bin` carry this same
    // bundle-signing modulus, and `world.signing.key.pem` is this same key.
    write(
        out_dir,
        "bnet.cert.pem",
        artifacts.leaf_chain_pem.as_bytes(),
    )?;
    write(out_dir, "bnet.key.pem", artifacts.leaf_key_pem.as_bytes())?;
    write(out_dir, "ca.pem", artifacts.ca_pem.as_bytes())?;
    write(out_dir, "signature_modulus.bin", &artifacts.modulus_le)?;
    write(out_dir, "cert_bundle.bin", &artifacts.signed_bundle)?;
    write(out_dir, "connect_to_modulus.bin", &artifacts.modulus_le)?;
    write(
        out_dir,
        "world.signing.key.pem",
        artifacts.signing_key_pem.as_bytes(),
    )?;

    eprintln!(
        "generated certificate and patch artifacts in {}",
        out_dir.display()
    );
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
