use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{broadcast, Mutex, RwLock};

use crate::auth::config::Config;
use crate::auth::database::Database;
use crate::auth::metrics::Metrics;
use crate::shared::console::{CommandRegistry, ConsoleCommand};

pub struct AuthServer {
    pub database: Database,
    pub config: Config,
    pub metrics: Arc<Metrics>,
    pub console_rx: Arc<Mutex<tokio::sync::mpsc::Receiver<ConsoleCommand>>>,
    pub command_registry: Arc<RwLock<CommandRegistry<AuthServer>>>,
    shutdown_tx: broadcast::Sender<()>,
    running: Arc<AtomicBool>,
}

impl AuthServer {
    pub fn new(
        config: Config,
        database: Database,
        metrics: Arc<Metrics>,
        shutdown_tx: broadcast::Sender<()>,
    ) -> Self {
        use crate::auth::console::commands::register_all_commands;

        let mut command_registry = CommandRegistry::new();
        register_all_commands(&mut command_registry);

        Self {
            database,
            config,
            metrics,
            console_rx: Arc::new(Mutex::new(tokio::sync::mpsc::channel(1).1)),
            command_registry: Arc::new(RwLock::new(command_registry)),
            shutdown_tx,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub async fn set_console_receiver(&self, rx: tokio::sync::mpsc::Receiver<ConsoleCommand>) {
        *self.console_rx.lock().await = rx;
    }

    pub async fn get_command_registry(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, CommandRegistry<AuthServer>> {
        self.command_registry.read().await
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = self.shutdown_tx.send(());
    }

    pub async fn process_console_commands(&self) {
        if let Err(e) = self
            .command_registry
            .read()
            .await
            .process_commands(&self.console_rx, self)
            .await
        {
            tracing::error!("Console command processing error: {}", e);
        }
    }
}
