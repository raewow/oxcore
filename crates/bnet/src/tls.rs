//! TLS setup shared by the REST login service and the BGS RPC channel.
//!
//! Modern clients refuse plaintext on either port. The certificate presented here must be the
//! one named in the certificate bundle patched into the client executable — see
//! `crates/patcher` and [`crate::certs`].

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Build a TLS acceptor from a PEM certificate chain and private key.
pub fn build_acceptor(cert_file: &Path, key_file: &Path) -> Result<TlsAcceptor> {
    install_crypto_provider();

    let certs = load_certs(cert_file)?;
    let key = load_key(key_file)?;

    // Force TLS 1.2. The modern BGS client (like real Battle.net) expects TLS 1.2 on this channel;
    // if the server negotiates TLS 1.3 the client completes the handshake and then silently drops
    // the connection without sending a single BGS frame. HermesProxy pins Tls12 for the same reason.
    let builder = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
        .with_no_client_auth();

    // The client's ClientHello sends `status_request` (OCSP). Real Battle.net staples an OCSP
    // response; staple ours if `ocsp.der` sits next to the cert, in case the client requires it.
    let ocsp = cert_file
        .parent()
        .map(|dir| dir.join("ocsp.der"))
        .filter(|p| p.exists())
        .map(std::fs::read)
        .transpose()
        .context("failed to read OCSP staple")?;

    let mut config = match ocsp {
        Some(ocsp) => {
            tracing::info!("stapling OCSP response ({} bytes)", ocsp.len());
            builder.with_single_cert_with_ocsp(certs, key, ocsp)
        }
        None => builder.with_single_cert(certs, key),
    }
    .context("certificate and private key do not match")?;

    // The client offers the `session_ticket` extension; schannel-based clients (like WoW) expect
    // the server to issue TLS 1.2 session tickets, which rustls omits by default. Enable them.
    if let Ok(ticketer) = rustls::crypto::ring::Ticketer::new() {
        config.ticketer = ticketer;
        tracing::info!("TLS 1.2 session tickets enabled");
    }

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Select the ring crypto provider once per process.
///
/// rustls panics rather than erroring if no provider is installed and it cannot infer one from
/// crate features, which is easy to trip over as soon as anything else in the dependency graph
/// pulls in a second backend. Choosing explicitly makes that impossible.
fn install_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // Fails only if a provider is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open certificate file: {}", path.display()))?;
    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(file))
        .collect::<Result<_, _>>()
        .with_context(|| format!("failed to parse certificates in {}", path.display()))?;

    if certs.is_empty() {
        bail!("no certificates found in {}", path.display());
    }

    Ok(certs)
}

fn load_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open private key file: {}", path.display()))?;
    rustls_pemfile::private_key(&mut BufReader::new(file))
        .with_context(|| format!("failed to parse private key in {}", path.display()))?
        .with_context(|| format!("no private key found in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Generates a throwaway self-signed cert/key pair into `dir`, returning their paths.
    fn write_self_signed(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let cert_path = dir.join("test.cert.pem");
        let key_path = dir.join("test.key.pem");

        File::create(&cert_path)
            .unwrap()
            .write_all(cert.cert.pem().as_bytes())
            .unwrap();
        File::create(&key_path)
            .unwrap()
            .write_all(cert.key_pair.serialize_pem().as_bytes())
            .unwrap();

        (cert_path, key_path)
    }

    #[test]
    fn builds_acceptor_from_pem_pair() {
        let dir = std::env::temp_dir().join("oxcore-bnet-tls-ok");
        std::fs::create_dir_all(&dir).unwrap();
        let (cert, key) = write_self_signed(&dir);

        assert!(build_acceptor(&cert, &key).is_ok());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_certificate_is_an_error() {
        let result = build_acceptor(Path::new("/nonexistent.pem"), Path::new("/nonexistent.key"));
        let err = match result {
            Ok(_) => panic!("expected an error for a missing certificate"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("certificate file"));
    }
}
