//! Accept loop for modern (1.14.x) world connections.
//!
//! Binds a TCP port and, per connection, runs the auth handshake ([`super::driver::run_auth`]) and
//! then the encrypted packet loop ([`super::driver::run_connection`]). It sits **alongside** the
//! vanilla world socket — a modern client connects here, a 1.12 client to the legacy listener.
//!
//! This is the thin accept-side glue; the substantive logic is in [`super::driver`]. Wiring it into
//! the world bootstrap (sourcing the realm build/OS, loading `world.signing.key.pem`, and choosing
//! the port) is a configuration step handled by the caller.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use oxcore_shared::database::AccountRepository;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use super::driver::{run_auth, run_connection, AccountSessionKeys, ModernAuthContext};
use super::packets::EnterEncryptedModeSigner;

/// Per-server settings for the modern listener that don't vary by connection.
#[derive(Debug, Clone)]
pub struct ModernServerConfig {
    /// The client build the realm serves (selects the auth seed).
    pub build: u32,
    /// The client OS string (selects the auth seed): `Wn64`, `Mc64`, `MacA`.
    pub os: String,
    /// The realm's virtual-realm address sent in `SMSG_AUTH_RESPONSE`.
    pub virtual_realm_address: u32,
    /// Active/account expansion levels for `SMSG_AUTH_RESPONSE` (0 for Classic Era).
    pub active_expansion: u8,
    pub account_expansion: u8,
}

/// Serve modern world connections until `shutdown` fires. `signer` signs the encrypted-mode
/// message and must pair with the `connect_to_modulus` patched into the client.
pub async fn serve_modern(
    addr: SocketAddr,
    accounts: AccountRepository,
    signer: Arc<dyn EnterEncryptedModeSigner>,
    config: Arc<ModernServerConfig>,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind modern world listener on {addr}"))?;
    info!(%addr, "modern (1.14.x) world listener listening");

    loop {
        let (stream, peer) = tokio::select! {
            _ = shutdown.recv() => {
                info!("modern world listener shutting down");
                return Ok(());
            }
            accepted = listener.accept() => match accepted {
                Ok(v) => v,
                Err(e) => {
                    warn!("modern accept failed: {e}");
                    continue;
                }
            },
        };

        let accounts = accounts.clone();
        let signer = signer.clone();
        let config = config.clone();
        tokio::spawn(async move {
            let mut stream = stream;
            let provider = AccountSessionKeys {
                accounts: &accounts,
            };
            let ctx = ModernAuthContext {
                signer: signer.as_ref(),
                sessions: &provider,
                build: config.build,
                os: &config.os,
                virtual_realm_address: config.virtual_realm_address,
                active_expansion: config.active_expansion,
                account_expansion: config.account_expansion,
            };

            match run_auth(&mut stream, &ctx).await {
                Ok(conn) => {
                    debug!(%peer, account = %conn.account, "modern client authenticated");
                    if let Err(e) = run_connection(stream, conn).await {
                        debug!(%peer, "modern connection ended: {e}");
                    }
                }
                Err(e) => debug!(%peer, "modern auth failed: {e}"),
            }
        });
    }
}
