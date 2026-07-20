//! Bnet Server - standalone binary.
//!
//! Serves the Battle.net login flow for 1.14.x clients. Run it alongside the `auth` binary,
//! which keeps serving 1.12 clients on the legacy realmd protocol.

use anyhow::{Context, Result};
use oxcore_bnet::shared::config::{find_config_file, load_toml};
use oxcore_tui::LogSettings;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Deserialize)]
struct RootConfig {
    bnet: oxcore_bnet::config::Config,
}

struct Args {
    config_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    let config_path = args
        .config_path
        .map(PathBuf::from)
        .unwrap_or_else(find_config_file);

    let root: RootConfig = load_toml(&config_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_path.display()
        )
    })?;
    let config = root.bnet;

    oxcore_tui::install_headless_subscriber(&LogSettings {
        console_level: config.log_level,
        file_level: config.log_file_level,
        log_file: Some(resolve_log_file(&config.logs_dir, &config.log_file)),
        wipe: false,
    })?;

    info!("bnet server starting up...");

    let (shutdown_tx, _rx) = tokio::sync::broadcast::channel(1);
    let server = oxcore_bnet::serve(config, shutdown_tx.clone()).await?;

    info!(
        portal = %server.config.portal_address(),
        "bnet server ready — patched clients should use this as their portal"
    );

    tokio::signal::ctrl_c().await?;
    info!("shutdown signal received");
    let _ = shutdown_tx.send(());

    Ok(())
}

fn parse_args() -> Args {
    let mut config_path = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-c" | "--config" => config_path = args.next(),
            _ => {}
        }
    }

    Args { config_path }
}

fn resolve_log_file(logs_dir: &Path, log_file: &str) -> PathBuf {
    if logs_dir.as_os_str().is_empty() {
        PathBuf::from(log_file)
    } else {
        logs_dir.join(log_file)
    }
}
