//! World Socket Manager - TCP listener and connection management
//!
//! Manages the TCP server that accepts incoming client connections
//! and spawns tasks to handle each connection.

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use crate::shared::database::Databases;
use crate::world::core::network::socket::WorldSocket;
use crate::world::core::session::SessionManager;
use crate::world::World;

/// Manages the world server TCP listener and connections
pub struct WorldSocketMgr {
    /// TCP listener
    listener: Option<TcpListener>,
    /// Session manager
    session_mgr: Arc<SessionManager>,
    /// Database connections
    databases: Arc<Databases>,
    /// World reference
    world: Arc<World>,
    /// Bind address
    bind_addr: SocketAddr,
    /// Running flag
    running: AtomicBool,
    /// Connection counter (Arc for sharing with spawned tasks)
    connection_count: Arc<AtomicU32>,
    /// Maximum connections (0 = unlimited)
    max_connections: u32,
    /// Active connection task handles for shutdown
    connection_tasks: Arc<std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl WorldSocketMgr {
    /// Create a new socket manager
    pub fn new(
        bind_addr: SocketAddr,
        session_mgr: Arc<SessionManager>,
        databases: Arc<Databases>,
        world: Arc<World>,
    ) -> Self {
        Self {
            listener: None,
            session_mgr,
            databases,
            world,
            bind_addr,
            running: AtomicBool::new(false),
            connection_count: Arc::new(AtomicU32::new(0)),
            max_connections: 0,
            connection_tasks: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Set maximum connections (0 = unlimited)
    pub fn set_max_connections(&mut self, max: u32) {
        self.max_connections = max;
    }

    /// Get current connection count
    pub fn connection_count(&self) -> u32 {
        self.connection_count.load(Ordering::Relaxed)
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start listening for connections
    pub async fn start(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("world server listening on {}", self.bind_addr);
        self.listener = Some(listener);
        self.running.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Stop the socket manager
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Abort all active connection tasks
    pub fn abort_all_connections(&self) {
        let mut tasks = self.connection_tasks.lock().unwrap();
        tracing::debug!("Aborting {} active connection tasks", tasks.len());
        for task in tasks.drain(..) {
            task.abort();
        }
    }

    /// Accept connections in a loop
    pub async fn run(&self) -> Result<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Socket manager not started"))?;

        info!("world server ready, accepting connections");

        while self.running.load(Ordering::Relaxed) {
            // Check connection limit
            if self.max_connections > 0 {
                let current = self.connection_count.load(Ordering::Relaxed);
                if current >= self.max_connections {
                    warn!(
                        "Maximum connections reached ({}), rejecting new connections",
                        self.max_connections
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(100));
                    continue;
                }
            }

            match listener.accept().await {
                Ok((stream, addr)) => {
                    // Disable Nagle's algorithm for low-latency game packets
                    if let Err(e) = stream.set_nodelay(true) {
                        error!("Failed to set TCP_NODELAY for {}: {}", addr, e);
                    }

                    info!("New connection from {}", addr);

                    // Increment connection counter
                    self.connection_count.fetch_add(1, Ordering::Relaxed);

                    let session_mgr = Arc::clone(&self.session_mgr);
                    let databases = Arc::clone(&self.databases);
                    let world = Arc::clone(&self.world);
                    let connection_count = Arc::clone(&self.connection_count);
                    let connection_tasks = Arc::clone(&self.connection_tasks);

                    // Spawn a task to handle this connection
                    let handle = tokio::spawn(async move {
                        let socket = WorldSocket::new(stream, addr, session_mgr, databases, world);

                        if let Err(e) = socket.run().await {
                            error!("Connection error for {}: {}", addr, e);
                        }

                        // Decrement connection counter when done
                        connection_count.fetch_sub(1, Ordering::Relaxed);

                        info!("Connection closed: {}", addr);
                    });

                    // Track the task handle
                    {
                        let mut tasks = connection_tasks.lock().unwrap();
                        tasks.push(handle);
                    }
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }

        info!("world server stopped accepting connections");
        Ok(())
    }
}
