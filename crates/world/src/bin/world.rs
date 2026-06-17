//! World Server - standalone binary.
//!
//! Runs only the world server, behind the shared oxcore TUI (single-pane view). Use
//! `--headless` (or run without a TTY) for plain stderr/file logging.

use anyhow::{Context, Result};
use oxcore_shared::config::{find_config_file, load_toml};
use oxcore_tui::{LoadUpdate, LogControl, LogSettings, LogSource, LogStore, Progress, ServerPane};
use serde::Deserialize;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Deserialize)]
struct RootConfig {
    world: oxcore_world::config::Config,
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

    let root: RootConfig = load_toml(&config_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_path.display()
        )
    })?;
    let config = root.world;

    let use_tui = !args.headless && std::io::stdout().is_terminal();
    let settings = LogSettings {
        console_level: config.log_level,
        file_level: config.log_file_level,
        log_file: resolve_log_file(&config.logs_dir, &config.log_file),
        wipe: config.log_wipe_on_start,
    };

    let store = LogStore::new(5000);
    let mut log_control: Option<LogControl> = None;
    if use_tui {
        log_control = Some(oxcore_tui::install_tui_subscriber(
            store.clone(),
            &settings,
        )?);
    } else {
        oxcore_tui::install_headless_subscriber(&settings)?;
    }

    info!("world server starting up...");

    let (shutdown_tx, _rx) = tokio::sync::broadcast::channel(1);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(100);

    if use_tui {
        let progress = Progress::new();
        let (load_tx, load_rx) = tokio::sync::mpsc::channel(16);
        let progress_task = progress.clone();
        let shutdown_task = shutdown_tx.clone();
        tokio::spawn(async move {
            match oxcore_world::serve(config, shutdown_task.subscribe(), cmd_rx, progress_task)
                .await
            {
                Ok(world) => {
                    let commands = world.command_registry.read().await.command_names();
                    let pane = ServerPane {
                        name: "World".to_string(),
                        source: LogSource::World,
                        metrics: Box::new(oxcore_world::WorldMetrics::new(world)),
                        cmd_tx,
                        commands,
                    };
                    let _ = load_tx.send(LoadUpdate::Ready(vec![pane])).await;
                }
                Err(e) => {
                    let _ = load_tx
                        .send(LoadUpdate::Failed(format!("world: {}", e)))
                        .await;
                }
            }
        });
        oxcore_tui::run_tui_loading(
            store,
            log_control.context("TUI log control was not initialized")?,
            progress,
            shutdown_tx.clone(),
            load_rx,
        )
        .await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    } else {
        oxcore_world::serve(config, shutdown_tx.subscribe(), cmd_rx, Progress::new()).await?;
        info!("running headless; press Ctrl+C to stop");
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(());
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    info!("world server shutdown complete");
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
    let matches = clap::Command::new("world")
        .version("0.1.0")
        .about("World of Warcraft world server")
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
