use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::auth::auth::AuthSocket;
use crate::auth::config::Config;
use crate::auth::database::Database;
use crate::auth::metrics::Metrics;
use crate::auth::patch::PatchCache;
use crate::auth::realm::{AllowedBuilds, RealmList};

pub async fn start_server(
    config: &Config,
    database: Database,
    shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let addr = format!("{}:{}", config.bind_ip, config.realm_server_port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("auth server listening on {}", addr);

    let metrics = Arc::new(Metrics::new());

    let metrics_log = metrics.clone();
    let mut metrics_shutdown = shutdown.resubscribe();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let snapshot = metrics_log.snapshot();
                    info!("{}", snapshot.format());
                }
                _ = metrics_shutdown.recv() => {
                    break;
                }
            }
        }
    });

    let realm_repo = Arc::new(database.realms.clone());
    let db_pool = database.pool().clone();
    let realm_list = Arc::new(RealmList::new(
        realm_repo.clone(),
        config.realms_state_update_delay as u64,
        config.realm_offline_threshold,
    ));

    if let Err(e) = realm_list.load_realms().await {
        error!("Failed to load initial realm list: {}", e);
        return Err(e);
    }

    let allowed_builds = Arc::new(
        AllowedBuilds::load_from_db(&database.realms)
            .await
            .context("Failed to load allowed client builds")?,
    );

    let builds_count = allowed_builds.get_all_builds().await.len();
    if builds_count == 0 {
        error!("No valid client builds specified in database");
        return Err(anyhow::anyhow!("No allowed client builds found"));
    }

    info!("Loaded {} allowed client builds", builds_count);

    let realm_list_task = realm_list.clone();
    tokio::spawn(async move {
        if let Err(e) = realm_list_task.start_update_task().await {
            error!("Realm list update task error: {}", e);
        }
    });

    let patch_cache = Arc::new(
        PatchCache::new(&config.patches_dir)
            .await
            .context("Failed to initialize patch cache")?,
    );
    info!(
        "Patch cache initialized with {} patches",
        patch_cache.get_patch_files().await.len()
    );

    let db_pool_ping = db_pool.clone();
    let max_ping_time = config.max_ping_time;
    let mut ping_shutdown = shutdown.resubscribe();
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(max_ping_time as u64 * 60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = sqlx::query("SELECT 1").execute(&db_pool_ping).await {
                        error!("Database ping failed: {}", e);
                    }
                }
                _ = ping_shutdown.recv() => {
                    break;
                }
            }
        }
    });

    let mut connection_tasks = Vec::new();
    let mut shutdown_rx = shutdown;

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, remote_addr)) => {
                        metrics.increment_connections();
                        info!("Accepted connection from {} (total: {}, active: {})",
                              remote_addr,
                              metrics.snapshot().connections_total,
                              metrics.snapshot().connections_active);

                        let db = database.clone();
                        let realms = realm_list.clone();
                        let patches = patch_cache.clone();
                        let builds = allowed_builds.clone();
                        let cfg = config.clone();
                        let metrics_clone = metrics.clone();
                        let mut conn_shutdown = shutdown_rx.resubscribe();

                        let task = tokio::spawn(async move {
                            let socket = AuthSocket::new(
                                stream,
                                remote_addr,
                                db,
                                realms,
                                patches,
                                builds,
                                cfg,
                                metrics_clone.clone()
                            );

                            tokio::select! {
                                result = socket.handle() => {
                                    if let Err(e) = result {
                                        error!("Error handling connection from {}: {}", remote_addr, e);
                                    }
                                }
                                _ = conn_shutdown.recv() => {
                                    warn!("Connection from {} cancelled due to shutdown", remote_addr);
                                }
                            }

                            metrics_clone.decrement_connections();
                        });

                        connection_tasks.push(task);
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }

            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, closing server...");
                break;
            }
        }
    }

    info!(
        "Waiting for {} active connections to close...",
        metrics.snapshot().connections_active
    );

    let grace_period = tokio::time::Duration::from_secs(10);
    let start = std::time::Instant::now();

    while metrics.snapshot().connections_active > 0 && start.elapsed() < grace_period {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let remaining = metrics.snapshot().connections_active;
    if remaining > 0 {
        warn!("Forcefully closing {} remaining connections", remaining);
        for task in connection_tasks {
            task.abort();
        }
    } else {
        info!("All connections closed gracefully");
    }

    let final_metrics = metrics.snapshot();
    info!("Final metrics:\n{}", final_metrics.format());

    Ok(())
}
