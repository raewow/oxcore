//! `bnet gen-certs` — mint the TLS certificate and the client-patch artifacts together.
//!
//! Produces four files in the output directory:
//!
//! | File | Used by | Purpose |
//! |------|---------|---------|
//! | `bnet.cert.pem` | server | TLS certificate served on both ports |
//! | `bnet.key.pem` | server | TLS private key |
//! | `signature_modulus.bin` | patcher `--modulus` | replaces the client's bundle-verify modulus |
//! | `cert_bundle.bin` | patcher `--cert-bundle` | the signed bundle trusting `bnet.cert.pem` |
//!
//! The signing key is deliberately *not* written: it is used once, here, and never needed
//! again. Regenerating replaces every artifact as a set, so the server cert, the embedded
//! modulus and the embedded bundle always match.

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

    write(out_dir, "bnet.cert.pem", tls.cert.pem().as_bytes())?;
    write(out_dir, "bnet.key.pem", tls.key_pair.serialize_pem().as_bytes())?;
    write(out_dir, "signature_modulus.bin", &artifacts.modulus)?;
    write(out_dir, "cert_bundle.bin", &artifacts.signed_bundle)?;

    eprintln!("generated certificate and patch artifacts in {}", out_dir.display());
    eprintln!("  server: point cert_file/key_file at bnet.cert.pem / bnet.key.pem");
    eprintln!(
        "  patcher: oxcore-patcher -i Wow.exe \\\n           \
         --modulus {}/signature_modulus.bin \\\n           \
         --cert-bundle {}/cert_bundle.bin",
        out_dir.display(),
        out_dir.display()
    );
    Ok(())
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let path = dir.join(name);
    std::fs::write(&path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
