//! Library entry point for running the bnet server.
//!
//! Mirrors [`oxcore_auth::serve`]: builds shared state, spawns the accept loops as background
//! tasks driven by the shutdown broadcast, and returns immediately. Installs no tracing
//! subscriber and no ctrl-c handler — the caller owns those.

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::broadcast;
use tracing::error;

use crate::config::Config;
use crate::database::Database;
use crate::rest::{self, RestState};
use crate::server::{serve_bnet, serve_rest};
use crate::tls::build_acceptor;

pub struct BnetServer {
    pub config: Config,
}

pub async fn serve(config: Config, shutdown_tx: broadcast::Sender<()>) -> Result<Arc<BnetServer>> {
    let acceptor = build_acceptor(&config.cert_file, &config.key_file)?;
    let db = Database::connect(&config.login_database_url).await?;

    let rest_addr = format!("{}:{}", config.bind_ip, config.login_port)
        .parse()
        .with_context(|| {
            format!(
                "invalid REST bind address: {}:{}",
                config.bind_ip, config.login_port
            )
        })?;
    let bnet_addr = format!("{}:{}", config.bind_ip, config.bnet_port)
        .parse()
        .with_context(|| {
            format!(
                "invalid BGS bind address: {}:{}",
                config.bind_ip, config.bnet_port
            )
        })?;

    let web_auth_url = config.login_base_url();
    let world_port = config.world_port;
    let router = rest::router(Arc::new(RestState::new(config.clone(), db.clone())));

    let rest_acceptor = acceptor.clone();
    let rest_shutdown = shutdown_tx.subscribe();
    tokio::spawn(async move {
        if let Err(e) = serve_rest(rest_addr, rest_acceptor, router, rest_shutdown).await {
            error!("REST login service error: {e}");
        }
    });

    let bnet_shutdown = shutdown_tx.subscribe();
    tokio::spawn(async move {
        if let Err(e) = serve_bnet(
            bnet_addr,
            acceptor,
            db,
            web_auth_url,
            world_port,
            bnet_shutdown,
        )
        .await
        {
            error!("BGS RPC channel error: {e}");
        }
    });

    Ok(Arc::new(BnetServer { config }))
}
