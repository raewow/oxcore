//! Accept loops for the two listeners.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

use crate::database::Database;
use crate::rpc::services::Services;

/// Serve the REST login router over TLS until `shutdown` fires.
pub async fn serve_rest(
    addr: SocketAddr,
    acceptor: TlsAcceptor,
    router: Router,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind REST login service on {addr}"))?;

    info!(%addr, "REST login service listening");

    loop {
        let (stream, peer) = tokio::select! {
            _ = shutdown.recv() => {
                info!("REST login service shutting down");
                return Ok(());
            }
            accepted = listener.accept() => match accepted {
                Ok(v) => v,
                Err(e) => {
                    // A failed accept is usually a transient per-connection problem (fd limits,
                    // a client that vanished mid-handshake); keep the listener alive.
                    warn!("REST accept failed: {e}");
                    continue;
                }
            },
        };

        let acceptor = acceptor.clone();
        let router = router.clone();

        tokio::spawn(async move {
            debug!(%peer, "REST TCP connection accepted");
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    // Overwhelmingly this means the client rejected our certificate, which is
                    // the single most common misconfiguration — log it at debug with the peer
                    // so it is findable without drowning the log.
                    warn!(%peer, "REST TLS handshake failed: {e}");
                    return;
                }
            };
            debug!(%peer, "REST TLS session established");

            // Deliberately not hyper: it lowercases response header names and the login browser
            // will not accept that. See `rest::wire`.
            if let Err(e) = crate::rest::wire::serve_connection(tls_stream, router).await {
                debug!(%peer, "REST connection error: {e}");
            }
        });
    }
}

/// Serve the BGS protobuf-RPC channel until `shutdown` fires. Each accepted connection gets its
/// own [`Services`] carrying the shared database handle and the web-auth URL handed to the client
/// during `Logon`.
pub async fn serve_bnet(
    addr: SocketAddr,
    acceptor: TlsAcceptor,
    db: Database,
    web_auth_url: String,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind BGS RPC channel on {addr}"))?;

    info!(%addr, "BGS RPC channel listening");

    loop {
        let (stream, peer) = tokio::select! {
            _ = shutdown.recv() => {
                info!("BGS RPC channel shutting down");
                return Ok(());
            }
            accepted = listener.accept() => match accepted {
                Ok(v) => v,
                Err(e) => {
                    warn!("BGS accept failed: {e}");
                    continue;
                }
            },
        };

        let acceptor = acceptor.clone();
        let services = Services::new(db.clone(), web_auth_url.clone());
        tokio::spawn(async move {
            // Temporary diagnostic: peek the raw ClientHello (without consuming it) so we can see
            // the TLS extensions the client offers — ALPN in particular.
            let mut hello = [0u8; 512];
            if let Ok(n) = stream.peek(&mut hello).await {
                debug!(%peer, n, hello = %hex::encode(&hello[..n.min(400)]), "raw client hello");
            }
            match acceptor.accept(stream).await {
                Ok(session) => {
                    debug!(%peer, "BGS RPC session established");
                    if let Err(e) = crate::rpc::session::run(services, session).await {
                        debug!(%peer, "BGS session ended: {e}");
                    }
                }
                Err(e) => debug!(%peer, "TLS handshake failed: {e}"),
            }
        });
    }
}
