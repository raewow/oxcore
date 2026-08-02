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
use oxcore_db::database::AccountRepository;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use super::connect_to::{instance_address, ConnectKeyStore};
use super::driver::{
    run_auth, run_instance_auth, run_world_connection, AccountSessionKeys, ConnectionRole,
    ModernAuthContext,
};
use super::packets::EnterEncryptedModeSigner;
use crate::World;

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
    /// Sent in `SMSG_FEATURE_SYSTEM_STATUS_GLUE_SCREEN` as `MaxCharactersOnThisRealm`; without it
    /// the client hides the character-select screen's Create button entirely.
    pub characters_per_realm: u32,
    /// Address the client is told to open its world (instance) socket to.
    ///
    /// Must be reachable *by the client*, not by the server: it goes out in `SMSG_CONNECT_TO` and
    /// the client dials it directly. A loopback address here works only for a client on the same
    /// machine.
    pub instance_address: std::net::SocketAddr,
}

/// Serve modern world connections until `shutdown` fires. `signer` signs the encrypted-mode
/// message and must pair with the `connect_to_modulus` patched into the client.
pub async fn serve_modern(
    addr: SocketAddr,
    accounts: AccountRepository,
    signer: Arc<dyn EnterEncryptedModeSigner>,
    config: Arc<ModernServerConfig>,
    world: Arc<World>,
    keys: Arc<ConnectKeyStore>,
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
        let world = world.clone();
        let keys = keys.clone();
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
                characters_per_realm: config.characters_per_realm,
            };

            match run_auth(&mut stream, &ctx).await {
                Ok(conn) => {
                    debug!(%peer, account = %conn.account, "modern client authenticated");
                    let role = ConnectionRole::Realm {
                        keys,
                        signer,
                        instance_address: config.instance_address,
                    };
                    if let Err(e) = run_world_connection(stream, conn, world, role).await {
                        debug!(%peer, "modern connection ended: {e}");
                    }
                }
                Err(e) => debug!(%peer, "modern auth failed: {e}"),
            }
        });
    }
}

/// Serve the **instance** (world) connections a modern client opens after `SMSG_CONNECT_TO`.
///
/// Deliberately a second listener rather than a mode on the first: the client dials a host and port
/// of its own accord, and the two sockets are live at the same time. There is no auth here beyond
/// the single-use connect key — the realm connection already authenticated this account moments
/// ago, which is why the key must be unguessable and is consumed on use.
pub async fn serve_modern_instance(
    addr: SocketAddr,
    signer: Arc<dyn EnterEncryptedModeSigner>,
    world: Arc<World>,
    keys: Arc<ConnectKeyStore>,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind modern instance listener on {addr}"))?;
    info!(%addr, "modern (1.14.x) instance listener listening");

    loop {
        let (stream, peer) = tokio::select! {
            _ = shutdown.recv() => {
                info!("modern instance listener shutting down");
                return Ok(());
            }
            accepted = listener.accept() => match accepted {
                Ok(v) => v,
                Err(e) => {
                    warn!("modern instance accept failed: {e}");
                    continue;
                }
            },
        };

        let signer = signer.clone();
        let world = world.clone();
        let keys = keys.clone();
        tokio::spawn(async move {
            let mut stream = stream;
            match run_instance_auth(&mut stream, &keys, signer.as_ref()).await {
                Ok((conn, pending)) => {
                    debug!(%peer, account = %conn.account, "instance connection established");
                    let role = ConnectionRole::Instance {
                        player_guid: pending.player_guid,
                    };
                    if let Err(e) = run_world_connection(stream, conn, world, role).await {
                        debug!(%peer, "instance connection ended: {e}");
                    }
                }
                Err(e) => debug!(%peer, "instance auth failed: {e}"),
            }
        });
    }
}

/// Build the address to advertise in `SMSG_CONNECT_TO` from the configured external host and port.
pub fn advertised_instance_address(external_ip: std::net::Ipv4Addr, port: u16) -> SocketAddr {
    instance_address(external_ip, port)
}
