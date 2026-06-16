//! Auth Server - standalone binary.
//!
//! Runs only the auth server, behind the shared oxcore TUI (single-pane view). Use
//! `--headless` (or run without a TTY) for plain stderr/file logging.

use anyhow::{Context, Result};
use oxcore_auth::shared::config::{find_config_file, load_toml};
use oxcore_tui::{LogSettings, LogStore, LogSource, ServerPane};
use serde::Deserialize;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Deserialize)]
struct RootConfig {
    auth: oxcore_auth::config::Config,
}

struct Args {
    config_path: Option<String>,
    headless: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    let config_path = args
        .config_path
        .map(PathBuf::from)
        .unwrap_or_else(find_config_file);

    let root: RootConfig = load_toml(&config_path)
        .with_context(|| format!("Failed to load configuration from {}", config_path.display()))?;
    let config = root.auth;

    let use_tui = !args.headless && std::io::stdout().is_terminal();
    let settings = LogSettings {
        console_level: config.log_level,
        file_level: config.log_file_level,
        log_file: resolve_log_file(&config.logs_dir, &config.log_file),
        wipe: false,
    };

    let store = LogStore::new(5000);
    if use_tui {
        oxcore_tui::install_tui_subscriber(store.clone(), &settings)?;
    } else {
        oxcore_tui::install_headless_subscriber(&settings)?;
    }

    info!("auth server starting up...");

    let (shutdown_tx, _rx) = tokio::sync::broadcast::channel(1);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(100);
    let metrics = Arc::new(oxcore_auth::metrics::Metrics::new());

    let server =
        oxcore_auth::serve(config, metrics.clone(), shutdown_tx.clone(), cmd_rx).await?;
    let commands = server.command_registry.read().await.command_names();

    let pane = ServerPane {
        name: "Auth".to_string(),
        source: LogSource::Auth,
        metrics: Box::new(oxcore_auth::AuthMetrics::new(metrics)),
        cmd_tx,
        commands,
    };

    if use_tui {
        oxcore_tui::run_tui(vec![pane], store, shutdown_tx.clone()).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    } else {
        info!("running headless; press Ctrl+C to stop");
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(());
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!("auth server shutdown complete");
    std::process::exit(0);
}

fn resolve_log_file(logs_dir: &Path, log_file: &str) -> Option<PathBuf> {
    if log_file.is_empty() {
        None
    } else if logs_dir.as_os_str().is_empty() {
        Some(PathBuf::from(log_file))
    } else {
        Some(logs_dir.join(log_file))
    }
}

fn parse_args() -> Args {
    let matches = clap::Command::new("auth")
        .version("0.1.0")
        .about("World of Warcraft authentication server")
        .arg(
            clap::Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to configuration file"),
        )
        .arg(
            clap::Arg::new("headless")
                .long("headless")
                .action(clap::ArgAction::SetTrue)
                .help("Disable the TUI and log to stderr/file"),
        )
        .get_matches();

    Args {
        config_path: matches.get_one::<String>("config").cloned(),
        headless: matches.get_flag("headless"),
    }
}
