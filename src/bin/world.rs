//! World Server - Game world server
//!
//! Features:
//! - Slim objects (identity + persistent data only)
//! - System-owned state
//! - Clean separation: Managers → Systems → Handlers
//! - Async-first design for 10k+ player scalability

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};
use wow_server::shared::config::{find_config_file, RootConfig};
use wow_server::shared::console::run_console_input;
use wow_server::shared::database::Databases;
use wow_server::world::config::initialize_config_mgr;
use wow_server::world::core::network::socket_mgr::WorldSocketMgr;
use wow_server::world::logging;
use wow_server::world::Config as WorldConfig;
use wow_server::world::World;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse command line arguments
    let args = parse_args();

    // 2. Load configuration
    let config_path = args
        .config_path
        .map(PathBuf::from)
        .unwrap_or_else(find_config_file);

    let root = match RootConfig::load(&config_path) {
        Ok(root) => root,
        Err(e) => {
            logging::init_basic_logging()?;
            error!(
                "Failed to load configuration from {}: {}",
                config_path.display(),
                e
            );
            eprintln!("Error: Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    let config = root.world;

    // 3. Initialize database connection pools
    let databases = match Databases::new(&config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize databases: {}", e);
            eprintln!("Error: Failed to connect to databases: {}", e);
            std::process::exit(1);
        }
    };

    databases.ping_all().await?;

    // 4. Create World instance and set up shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let databases_arc = Arc::new(databases);
    // Create world config from the loaded root config
    let mut world_config = WorldConfig::default();
    world_config.start_player_money = config.start_player_money;
    world_config.start_player_level = config.start_player_level;
    world_config.max_players = config.max_players;
    world_config.player_limit = config.player_limit;
    world_config.characters_per_realm = config.characters_per_realm;
    world_config.min_player_name = config.min_player_name;
    world_config.max_player_name = config.max_player_name;
    world_config.strict_player_names = config.strict_player_names;
    world_config.characters_creating_disabled = config.characters_creating_disabled;
    world_config.is_pvp_realm = config.is_pvp_realm;
    world_config.allow_two_side_accounts = config.allow_two_side_accounts;
    world_config.allow_cross_faction_whispers = config.allow_cross_faction_whispers;
    world_config.allow_cross_faction_chat = config.allow_cross_faction_chat;
    world_config.allow_cross_faction_channel = config.allow_cross_faction_channel;
    world_config.allow_cross_faction_group = config.allow_cross_faction_group;
    world_config.allow_cross_faction_guild = config.allow_cross_faction_guild;
    world_config.allow_cross_faction_trade = config.allow_cross_faction_trade;
    world_config.allow_cross_faction_auction = config.allow_cross_faction_auction;
    world_config.allow_cross_faction_mail = config.allow_cross_faction_mail;
    world_config.allow_cross_faction_add_friend = config.allow_cross_faction_add_friend;
    world_config.logout_timer = config.logout_timer;
    world_config.log_level = config.log_level;
    world_config.log_file_level = config.log_file_level;
    world_config.log_file = config.log_file.clone();
    world_config.logs_dir = config.logs_dir.clone();
    world_config.realm_heartbeat_interval = config.realm_heartbeat_interval;
    let world_config_for_start = world_config.clone();
    let world_config = Arc::new(world_config);
    let world = Arc::new(World::new(
        databases_arc.clone(),
        world_config,
        config.world_update_interval,
        config.data_dir.clone(),
    ));

    // 5. Initialize global config manager
    initialize_config_mgr(world_config_for_start.clone());

    // Set shutdown receiver
    world.set_shutdown_receiver(shutdown_rx).await;

    // 6. Set realm_id on world
    let realm_id = if config.realm_id <= 0 {
        1
    } else {
        config.realm_id
    };
    world.set_realm_id(realm_id);

    // 7. Create console command channel and set receiver
    let (console_tx, console_rx) = tokio::sync::mpsc::channel(100);
    world.set_console_receiver(console_rx).await;

    // Spawn console input task
    let shutdown_rx_console = shutdown_tx.subscribe();
    let console_task = tokio::spawn(async move {
        run_console_input(console_tx, shutdown_rx_console).await;
    });

    // 7. Start world (logging, initialize, heartbeat, update loop, signal handler)
    let world_for_start = world.clone();
    world_for_start.start(&world_config_for_start).await?;

    print_banner();

    info!("world server starting up...");
    info!("world server initialized successfully");
    info!("Bind IP: {}", config.bind_ip);
    info!("Port: {}", config.world_server_port);
    info!("Update interval: {}ms", config.world_update_interval);
    info!("Data directory: {}", config.data_dir.display());

    // 8. Run socket server (stays in binary)
    let bind_addr: SocketAddr = format!("{}:{}", config.bind_ip, config.world_server_port)
        .parse()
        .context("Invalid bind address")?;

    let session_mgr = world.session_mgr.clone();
    let mut socket_mgr =
        WorldSocketMgr::new(bind_addr, session_mgr, databases_arc.clone(), world.clone());
    socket_mgr.start().await?;

    // Spawn update loop
    let world_clone = world.clone();
    let world_update_task = tokio::spawn(async move {
        if let Err(e) = world_clone.run().await {
            error!("World update loop error: {}", e);
        }
    });

    // Run socket manager with graceful shutdown on Ctrl+C
    tokio::select! {
        result = socket_mgr.run() => {
            if let Err(e) = result {
                error!("Socket manager error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("world server shutting down...");
        }
    }

    // Stop accepting new connections
    socket_mgr.stop();

    // Stop world update loop
    world.stop();

    // Send shutdown signal to any tasks listening
    let _ = shutdown_tx.send(());

    // Close all sessions (this closes packet channels, causing socket tasks to exit)
    world.session_mgr.close_all_sessions();

    // Abort all active connection tasks immediately (in case some are stuck)
    socket_mgr.abort_all_connections();

    // Shutdown world systems (this will abort background tasks)
    world.shutdown().await?;

    // Wait for world update loop to finish
    if let Err(e) = world_update_task.await {
        error!("Error waiting for world update loop: {:?}", e);
    }

    // Cancel console input task (it should exit on shutdown signal)
    console_task.abort();
    let _ = console_task.await;

    // Give tasks a moment to finish
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    info!("world server shutdown complete");

    // Force process exit to ensure we don't hang on background tasks
    // All cleanup has been done, so it's safe to exit
    std::process::exit(0);
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
    let matches = clap::Command::new("worldserver")
        .version("0.1.0")
        .about("World of Warcraft world server")
        .arg(
            clap::Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Path to configuration file"),
        )
        .get_matches();

    Args {
        config_path: matches.get_one::<String>("config").map(|s| s.clone()),
    }
}
