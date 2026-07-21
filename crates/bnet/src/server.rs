//! Accept loops for the two listeners.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::Router;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

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
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    // Overwhelmingly this means the client rejected our certificate, which is
                    // the single most common misconfiguration — log it at debug with the peer
                    // so it is findable without drowning the log.
                    debug!(%peer, "TLS handshake failed: {e}");
                    return;
                }
            };

            let service = TowerToHyperService::new(router);
            if let Err(e) = Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls_stream), service)
                .await
            {
                debug!(%peer, "REST connection error: {e}");
            }
        });
    }
}

/// Serve the BGS protobuf-RPC channel until `shutdown` fires.
///
/// M3 replaces the placeholder body with the framing codec and service dispatch; for now this
/// binds the port and closes connections so the listener address can be verified end to end.
pub async fn serve_bnet(
    addr: SocketAddr,
    acceptor: TlsAcceptor,
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
        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(session) => {
                    debug!(%peer, "BGS RPC session established");
                    if let Err(e) = crate::rpc::session::run(session).await {
                        debug!(%peer, "BGS session ended: {e}");
                    }
                }
                Err(e) => debug!(%peer, "TLS handshake failed: {e}"),
            }
        });
    }
}
