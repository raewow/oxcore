//! Library entry point for running the world server.
//!
//! [`serve`] performs all setup (databases, world init, socket listener, update loop),
//! returns a live `Arc<World>` for metrics/commands, and tears everything down when the
//! shared shutdown broadcast fires. Unlike the old bin `main`, it does NOT install a
//! tracing subscriber, register its own ctrl-c handler, or call `std::process::exit` —
//! the caller (unified runtime or standalone bin) owns those concerns.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use oxcore_shared::console::ConsoleCommand;
use oxcore_shared::database::{DatabaseUrls, Databases};
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};

use crate::config::{initialize_config_mgr, Config};
use crate::core::network::socket_mgr::WorldSocketMgr;
use crate::World;

/// Build the gameplay `Config` consumed by `World`/the config manager from the loaded
/// config. Mirrors the original bin field-copy so behaviour is unchanged.
fn build_world_config(config: &Config) -> Config {
    let mut world_config = Config::default();
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
    world_config.quest_low_level_hide_diff = config.quest_low_level_hide_diff;
    world_config.realm_id = config.realm_id;
    world_config.realm_name = config.realm_name.clone();
    world_config
}

/// Set up and start the world server. Returns the live `Arc<World>` immediately; the
/// accept loop, update loop, and shutdown watcher run as background tasks.
pub async fn serve(
    config: Config,
    shutdown_rx: broadcast::Receiver<()>,
    console_rx: mpsc::Receiver<ConsoleCommand>,
    progress: oxcore_tui::Progress,
) -> Result<Arc<World>> {
    progress.set_label("connecting databases");

    // 1. Databases
    let database_urls = DatabaseUrls {
        world: config.world_database_url.clone(),
        character: config.character_database_url.clone(),
        auth: config.login_database_url.clone(),
        logs: config.logs_database_url.clone(),
    };
    let databases = Databases::new(&database_urls)
        .await
        .context("Failed to connect to databases")?;
    databases.ping_all().await?;
    let databases = Arc::new(databases);

    // 2. World instance
    let world_config_for_start = build_world_config(&config);
    let world = Arc::new(World::new(
        databases.clone(),
        Arc::new(world_config_for_start.clone()),
        config.world_update_interval,
        config.data_dir.clone(),
    ));

    initialize_config_mgr(world_config_for_start.clone());
    world.set_progress(progress.clone());
    world.set_shutdown_receiver(shutdown_rx.resubscribe()).await;

    let realm_id = if config.realm_id <= 0 {
        1
    } else {
        config.realm_id
    };
    world.set_realm_id(realm_id);

    world.set_console_receiver(console_rx).await;

    // 3. Start world (init, realmlist online, heartbeat)
    world.start(&world_config_for_start).await?;

    info!("world server initialized successfully");
    info!("Bind IP: {}", config.bind_ip);
    info!("Port: {}", config.world_server_port);
    info!("Update interval: {}ms", config.world_update_interval);
    info!("Data directory: {}", config.data_dir.display());

    // 4. Socket listener
    let bind_addr: SocketAddr = format!("{}:{}", config.bind_ip, config.world_server_port)
        .parse()
        .context("Invalid bind address")?;
    let mut socket_mgr = WorldSocketMgr::new(
        bind_addr,
        world.session_mgr.clone(),
        databases.clone(),
        world.clone(),
    );
    socket_mgr.start().await?;
    let socket_mgr = Arc::new(socket_mgr);

    // 5. Update loop
    let world_run = world.clone();
    let update_task = tokio::spawn(async move {
        if let Err(e) = world_run.run().await {
            error!("World update loop error: {}", e);
        }
    });

    // 6. Accept loop
    let sm_accept = socket_mgr.clone();
    let accept_task = tokio::spawn(async move {
        if let Err(e) = sm_accept.run().await {
            error!("World socket manager error: {}", e);
        }
    });

    // 7. Shutdown watcher: orderly teardown when the shared signal fires.
    let world_sd = world.clone();
    let sm_sd = socket_mgr.clone();
    let mut shutdown_rx = shutdown_rx;
    tokio::spawn(async move {
        let _ = shutdown_rx.recv().await;
        info!("world server shutting down...");
        sm_sd.stop();
        world_sd.stop();
        world_sd.session_mgr.close_all_sessions();
        sm_sd.abort_all_connections();
        if let Err(e) = world_sd.shutdown().await {
            error!("World shutdown error: {}", e);
        }
        accept_task.abort();
        let _ = update_task.await;
        info!("world server shutdown complete");
    });

    Ok(world)
}

/// [`oxcore_tui::MetricsSource`] adapter for the world server.
pub struct WorldMetrics {
    world: Arc<World>,
}

impl WorldMetrics {
    pub fn new(world: Arc<World>) -> Self {
        Self { world }
    }
}

impl oxcore_tui::MetricsSource for WorldMetrics {
    fn snapshot(&self) -> oxcore_tui::MetricsSnapshot {
        let players = self.world.managers.player_mgr.player_count() as u64;
        let sessions = self.world.session_mgr.session_count() as u64;
        let stats = self.world.tick_stats.lock().clone();

        let mut gauges = vec![
            ("sessions".to_string(), sessions.to_string()),
            ("players".to_string(), players.to_string()),
            ("tps".to_string(), format!("{:.1}", stats.tps)),
            ("tick".to_string(), format!("{:.2} ms", stats.last_tick_ms)),
        ];
        for (name, ms) in &stats.phases {
            gauges.push((format!("· {}", name), format!("{:.2} ms", ms)));
        }

        oxcore_tui::MetricsSnapshot {
            connections: sessions,
            players_online: players,
            tps: stats.tps,
            tick_ms: stats.last_tick_ms,
            gauges,
        }
    }
}
