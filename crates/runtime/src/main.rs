//! Unified oxcore runtime.
//!
//! Runs the auth and world servers in a single process behind a shared ratatui TUI
//! (tabs: Both / Auth / World / Performance). With `--headless` (or when stdout is not a
//! TTY) it falls back to plain stderr/file logging for systemd/CI/piped use.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use oxcore_shared::config::{find_config_file, load_toml};
use oxcore_tui::{LogSettings, LogStore, ServerPane};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};

#[derive(Debug, Deserialize)]
struct RootConfig {
    auth: Option<oxcore_auth::config::Config>,
    world: Option<oxcore_world::config::Config>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RunMode {
    Both,
    Auth,
    World,
}

struct Args {
    config_path: Option<String>,
    headless: bool,
    only: RunMode,
}

fn parse_args() -> Args {
    let matches = clap::Command::new("oxcore")
        .version("0.1.0")
        .about("Unified oxcore runtime (auth + world) with a shared TUI")
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
        .arg(
            clap::Arg::new("only")
                .long("only")
                .value_name("SERVER")
                .value_parser(["auth", "world", "both"])
                .help("Run only one server (auth|world) instead of both"),
        )
        .get_matches();

    let only = match matches.get_one::<String>("only").map(|s| s.as_str()) {
        Some("auth") => RunMode::Auth,
        Some("world") => RunMode::World,
        _ => RunMode::Both,
    };

    Args {
        config_path: matches.get_one::<String>("config").cloned(),
        headless: matches.get_flag("headless"),
        only,
    }
}

/// Build TUI/headless log settings from the world config (preferred) or auth config.
fn log_settings(root: &RootConfig) -> LogSettings {
    if let Some(w) = &root.world {
        return LogSettings {
            console_level: w.log_level,
            file_level: w.log_file_level,
            log_file: resolve_log_file(&w.logs_dir, &w.log_file),
            wipe: w.log_wipe_on_start,
        };
    }
    if let Some(a) = &root.auth {
        return LogSettings {
            console_level: a.log_level,
            file_level: a.log_file_level,
            log_file: resolve_log_file(&a.logs_dir, &a.log_file),
            wipe: false,
        };
    }
    LogSettings {
        console_level: 2,
        file_level: 0,
        log_file: None,
        wipe: false,
    }
}

fn resolve_log_file(logs_dir: &std::path::Path, log_file: &str) -> Option<PathBuf> {
    if log_file.is_empty() {
        None
    } else if logs_dir.as_os_str().is_empty() {
        Some(PathBuf::from(log_file))
    } else {
        Some(logs_dir.join(log_file))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    let config_path = args
        .config_path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(find_config_file);

    let root: RootConfig = load_toml(&config_path)
        .with_context(|| format!("Failed to load configuration from {}", config_path.display()))?;

    // Decide output mode.
    let use_tui = !args.headless && std::io::stdout().is_terminal();
    let settings = log_settings(&root);

    let store = LogStore::new(5000);
    if use_tui {
        oxcore_tui::install_tui_subscriber(store.clone(), &settings)?;
    } else {
        oxcore_tui::install_headless_subscriber(&settings)?;
    }

    info!("oxcore runtime starting ({:?} mode)", args.only);

    // One shared shutdown broadcast for everything.
    let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);

    let mut panes: Vec<ServerPane> = Vec::new();

    // --- Auth ---
    if matches!(args.only, RunMode::Both | RunMode::Auth) {
        let config = root
            .auth
            .clone()
            .context("[auth] config section missing")?;
        let metrics = Arc::new(oxcore_auth::metrics::Metrics::new());
        let (tx, rx) = mpsc::channel(100);
        let server = oxcore_auth::serve(config, metrics.clone(), shutdown_tx.clone(), rx).await?;
        let commands = server.command_registry.read().await.command_names();
        panes.push(ServerPane {
            name: "Auth".to_string(),
            source: oxcore_tui::LogSource::Auth,
            metrics: Box::new(oxcore_auth::AuthMetrics::new(metrics)),
            cmd_tx: tx,
            commands,
        });
    }

    // --- World ---
    if matches!(args.only, RunMode::Both | RunMode::World) {
        let config = root
            .world
            .clone()
            .context("[world] config section missing")?;
        let (tx, rx) = mpsc::channel(100);
        let world = oxcore_world::serve(config, shutdown_tx.subscribe(), rx).await?;
        let commands = world.command_registry.read().await.command_names();
        panes.push(ServerPane {
            name: "World".to_string(),
            source: oxcore_tui::LogSource::World,
            metrics: Box::new(oxcore_world::WorldMetrics::new(world)),
            cmd_tx: tx,
            commands,
        });
    }

    if use_tui {
        oxcore_tui::run_tui(panes, store, shutdown_tx.clone()).await?;
        // TUI exited (user quit); give servers a moment to tear down.
        tokio::time::sleep(Duration::from_millis(500)).await;
    } else {
        // Headless: single ctrl-c handler maps to the shared shutdown.
        info!("running headless; press Ctrl+C to stop");
        let _ = tokio::signal::ctrl_c().await;
        info!("shutdown signal received");
        let _ = shutdown_tx.send(());
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!("oxcore runtime shutdown complete");
    // Force exit so lingering background tasks (mlua, sqlx pools) don't hang the process.
    std::process::exit(0);
}
