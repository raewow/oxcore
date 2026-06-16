//! Library entry point for running the auth server.
//!
//! [`serve`] wraps database init + `AuthServer` construction + `start_server`, returns a
//! live `Arc<AuthServer>` for metrics/commands, and runs the accept loop as a background
//! task driven by the shared shutdown broadcast. It installs no tracing subscriber and no
//! ctrl-c handler — the caller owns those.

use std::sync::Arc;

use anyhow::Result;
use oxcore_shared::console::ConsoleCommand;
use tokio::sync::{broadcast, mpsc};
use tracing::error;

use crate::config::Config;
use crate::context::AuthServer;
use crate::init::initialize_database;
use crate::metrics::Metrics;
use crate::server::start_server;

/// Set up and start the auth server. Returns the live `Arc<AuthServer>` immediately; the
/// accept loop runs as a background task that stops when `shutdown_tx` fires.
pub async fn serve(
    config: Config,
    metrics: Arc<Metrics>,
    shutdown_tx: broadcast::Sender<()>,
    console_rx: mpsc::Receiver<ConsoleCommand>,
) -> Result<Arc<AuthServer>> {
    let database = initialize_database(&config).await?;

    let auth_server = Arc::new(AuthServer::new(
        config,
        database,
        metrics,
        shutdown_tx.clone(),
    ));
    auth_server.set_console_receiver(console_rx).await;

    let server = auth_server.clone();
    let shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        if let Err(e) = start_server(server, shutdown_rx).await {
            error!("auth server error: {}", e);
        }
    });

    Ok(auth_server)
}

/// [`oxcore_tui::MetricsSource`] adapter for the auth server.
pub struct AuthMetrics {
    metrics: Arc<Metrics>,
}

impl AuthMetrics {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
}

impl oxcore_tui::MetricsSource for AuthMetrics {
    fn snapshot(&self) -> oxcore_tui::MetricsSnapshot {
        let s = self.metrics.snapshot();
        oxcore_tui::MetricsSnapshot {
            connections: s.connections_active,
            players_online: 0,
            tps: 0.0,
            tick_ms: 0.0,
            gauges: vec![
                ("total conns".to_string(), s.connections_total.to_string()),
                ("auth ok".to_string(), s.authentications_success.to_string()),
                ("auth fail".to_string(), s.authentications_failed.to_string()),
                ("realm reqs".to_string(), s.realm_list_requests.to_string()),
                ("patches".to_string(), s.patch_transfers.to_string()),
                ("ip bans".to_string(), s.ip_bans.to_string()),
            ],
        }
    }
}
