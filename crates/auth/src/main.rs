use anyhow::Result;
use oxcore_auth::context::AuthServer;
use oxcore_auth::init::initialize_database;
use oxcore_auth::logging;
use oxcore_auth::metrics::Metrics;
use oxcore_auth::server::start_server;
use oxcore_auth::shared::config::{find_config_file, load_toml};
use oxcore_auth::shared::console::run_console_input;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    let config_path = args
        .config_path
        .map(PathBuf::from)
        .unwrap_or_else(find_config_file);

    let root = match load_toml::<RootConfig, _>(&config_path) {
        Ok(root) => root,
        Err(e) => {
            logging::init_basic()?;
            error!(
                "Failed to load configuration from {}: {}",
                config_path.display(),
                e
            );
            eprintln!("Error: Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    let config = root.auth;

    logging::init(&config)?;

    let database = match initialize_database(&config).await {
        Ok(db) => db,
        Err(e) => {
            logging::init_basic()?;
            error!("Failed to initialize database: {}", e);
            eprintln!("Error: Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

    let auth_server = Arc::new(AuthServer::new(
        config.clone(),
        database,
        Arc::new(Metrics::new()),
        shutdown_tx.clone(),
    ));

    let (console_tx, console_rx) = tokio::sync::mpsc::channel(100);
    auth_server.set_console_receiver(console_rx).await;

    let shutdown_rx_console = shutdown_tx.subscribe();
    tokio::spawn(async move {
        run_console_input(console_tx, shutdown_rx_console).await;
    });

    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::warn!("Failed to register SIGTERM handler: {}", e);
                    None
                }
            };

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("auth server shutting down...");
                    let _ = shutdown_tx_clone.send(());
                }
                _ = async {
                    if let Some(ref mut sigterm) = sigterm {
                        sigterm.recv().await;
                    }
                } => {
                    info!("auth server shutting down...");
                    let _ = shutdown_tx_clone.send(());
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to register Ctrl+C handler");
            info!("auth server shutting down...");
            let _ = shutdown_tx_clone.send(());
        }
    });

    print_banner();

    info!("auth server starting up...");
    info!("Configuration loaded from: {}", config_path.display());
    info!("auth server initialized successfully");
    info!("Bind IP: {}", config.bind_ip);
    info!("Port: {}", config.realm_server_port);
    info!("Patches directory: {}", config.patches_dir.display());

    if let Err(e) = start_server(auth_server, shutdown_rx).await {
        error!("Server error: {}", e);
        return Err(e);
    }

    info!("auth server shutdown complete");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    auth: oxcore_auth::config::Config,
}

fn print_banner() {
    println!();
    println!("▄████▄ ▄▄ ▄▄ ▄█████  ▄▄▄  ▄▄▄▄  ▄▄▄▄▄");
    println!("██  ██ ▀█▄█▀ ██     ██▀██ ██▄█▄ ██▄▄ ");
    println!("▀████▀ ██ ██ ▀█████ ▀███▀ ██ ██ ██▄▄▄");
    println!();
}

#[derive(Debug)]
struct Args {
    config_path: Option<String>,
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
        .get_matches();

    Args {
        config_path: matches.get_one::<String>("config").cloned(),
    }
}
