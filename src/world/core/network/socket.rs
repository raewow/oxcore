//! WorldSocket - per-connection handler
//!
//! Handles a single client TCP connection, managing:
//! - Initial authentication handshake
//! - Packet encryption/decryption
//! - Forwarding packets to/from the WorldSession

use anyhow::Result;
use bytes::{Buf, BytesMut};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::shared::database::Databases;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::core::network::crypt::AuthCrypt;
use crate::world::core::network::protocol::{
    build_server_header, client_payload_size, parse_client_header, server_packet_size,
    CLIENT_HEADER_SIZE, MAX_PACKET_SIZE, SERVER_HEADER_SIZE,
};
use crate::world::core::session::{SessionManager, WorldSession};
use crate::world::handlers::auth as auth_handler;
use crate::world::World;

/// Connection state for WorldSocket
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Initial state, sending auth challenge
    Challenge,
    /// Waiting for CMSG_AUTH_SESSION
    WaitingAuth,
    /// Authenticated, forwarding packets to session
    Authenticated,
    /// Error state, will disconnect
    Error,
}

/// Per-connection handler
pub struct WorldSocket {
    /// TCP stream
    stream: TcpStream,
    /// Remote address
    remote_addr: SocketAddr,
    /// Read buffer
    read_buffer: BytesMut,
    /// Encryption handler
    crypt: AuthCrypt,
    /// Connection state
    state: ConnectionState,
    /// Server seed (for auth challenge)
    server_seed: u32,
    /// Session manager reference
    session_mgr: Arc<SessionManager>,
    /// Database connections
    databases: Arc<Databases>,
    /// World reference
    world: Arc<World>,
    /// Account ID (set after authentication)
    account_id: Option<u32>,
    /// Account name (set after authentication)
    account_name: Option<String>,
    /// Security level (GM level)
    security: u8,
    /// Session ID (set after authentication)
    session_id: Option<u32>,
    /// Channel for sending packets to session
    packet_tx: Option<mpsc::UnboundedSender<WorldPacket>>,
    /// Channel for receiving packets from session
    packet_rx: Option<mpsc::UnboundedReceiver<WorldPacket>>,
}

impl WorldSocket {
    /// Create a new socket handler
    pub fn new(
        stream: TcpStream,
        remote_addr: SocketAddr,
        session_mgr: Arc<SessionManager>,
        databases: Arc<Databases>,
        world: Arc<World>,
    ) -> Self {
        Self {
            stream,
            remote_addr,
            read_buffer: BytesMut::with_capacity(4096),
            crypt: AuthCrypt::new(),
            state: ConnectionState::Challenge,
            server_seed: rand::random(),
            session_mgr,
            databases,
            world,
            account_id: None,
            account_name: None,
            security: 0,
            session_id: None,
            packet_tx: None,
            packet_rx: None,
        }
    }

    /// Get the remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Get server seed
    pub fn server_seed(&self) -> u32 {
        self.server_seed
    }

    /// Get connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Initialize encryption with session key
    pub fn init_crypt(&mut self, session_key: &[u8]) {
        debug!(
            "[CRYPT] Initializing encryption with session key ({} bytes)",
            session_key.len()
        );
        self.crypt.set_key_from_slice(session_key);
        self.crypt.init();
        self.state = ConnectionState::Authenticated;
        debug!("[CRYPT] Encryption initialized, state set to Authenticated");
    }

    /// Run the socket handler - main event loop
    pub async fn run(mut self) -> Result<()> {
        debug!("New connection from {}", self.remote_addr);

        // Send auth challenge
        self.send_auth_challenge().await?;
        self.state = ConnectionState::WaitingAuth;

        // Authentication loop
        loop {
            let mut read_buf = vec![0u8; 4096];
            match self.stream.read(&mut read_buf).await {
                Ok(0) => {
                    info!("Connection closed by client: {}", self.remote_addr);
                    return Ok(());
                }
                Ok(n) => {
                    self.read_buffer.extend_from_slice(&read_buf[..n]);

                    // Process packets (unencrypted during auth phase)
                    while let Some(packet) = self.read_packet_unencrypted()? {
                        if packet.opcode() == Opcode::CMSG_AUTH_SESSION {
                            self.handle_auth_session(packet).await?;
                            if self.state == ConnectionState::Authenticated {
                                break;
                            }
                        } else {
                            warn!("Unexpected packet {:?} during auth", packet.opcode());
                            self.state = ConnectionState::Error;
                            return Err(anyhow::anyhow!("Unexpected packet during auth"));
                        }
                    }

                    if self.state == ConnectionState::Authenticated {
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading from {}: {}", self.remote_addr, e);
                    return Err(e.into());
                }
            }
        }

        // Main packet loop (after authentication)
        self.run_authenticated().await
    }

    /// Run the authenticated packet loop
    async fn run_authenticated(&mut self) -> Result<()> {
        use crate::world::handlers::dispatch_packet;

        debug!(
            "[POST-AUTH] Entered authenticated packet loop for {}",
            self.remote_addr
        );
        let mut read_buf = vec![0u8; 4096];
        let mut last_select_time = std::time::Instant::now();

        loop {
            let select_wait = last_select_time.elapsed();
            tokio::select! {
                // Read from client
                result = self.stream.read(&mut read_buf) => {
                    match result {
                        Ok(0) => {
                            self.handle_disconnect("normal");
                            return Ok(());
                        }
                        Ok(n) => {
                            if select_wait.as_millis() > 50 {
                                warn!("[PERF] Socket was busy for {}ms before reading {} bytes",
                                    select_wait.as_millis(), n);
                            }
                            trace!("[POST-AUTH] Received {} bytes from {}, buffer now {} bytes",
                                n, self.remote_addr, self.read_buffer.len() + n);
                            self.read_buffer.extend_from_slice(&read_buf[..n]);

                            // Process encrypted packets
                            while let Some(mut packet) = self.read_packet_encrypted()? {
                                let dispatch_start = std::time::Instant::now();
                                let packet_opcode = packet.opcode();
                                trace!("[PACKET] {:?} (opcode=0x{:04X}, size={}) from {}", packet_opcode, packet_opcode.as_u32(), packet.size(), self.remote_addr);

                                // Get session for this connection
                                if let Some(session_id) = self.session_id {
                                    if let Some(session) = self.session_mgr.get_session(session_id) {
                                        debug!("Dispatching {:?} to handler (state: {:?})", packet_opcode, session.state());
                                        // Route packet based on session state
                                        if let Err(e) = self.route_packet(session, packet).await {
                                            error!("Error routing packet: {}", e);
                                        }
                                    } else {
                                        warn!("No session found for session_id {}", session_id);
                                    }
                                } else {
                                    warn!("No session_id set for connection, cannot dispatch packet {:?}", packet_opcode);
                                }

                                let elapsed = dispatch_start.elapsed();
                                if elapsed.as_millis() > 5 {
                                    warn!("[PERF] Packet {:?} took {}ms to dispatch", packet_opcode, elapsed.as_millis());
                                }
                            }
                        }
                        Err(e) => {
                            self.handle_disconnect("read error");
                            return Err(e.into());
                        }
                    }
                    last_select_time = std::time::Instant::now();
                }

                // Send packets from session to client (for async responses)
                packet = async {
                    match &mut self.packet_rx {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    if let Some(packet) = packet {
                        // Collect any additional queued packets to send in batch
                        let mut pending = Vec::new();
                        if let Some(rx) = &mut self.packet_rx {
                            while let Ok(p) = rx.try_recv() {
                                pending.push(p);
                            }
                        }
                        self.send_packet(&packet).await?;
                        for p in &pending {
                            self.send_packet(p).await?;
                        }
                    } else {
                        self.handle_disconnect("channel closed");
                        return Ok(());
                    }
                    last_select_time = std::time::Instant::now();
                }
            }
        }
    }

    /// Route packet based on session state
    async fn route_packet(
        &self,
        session: Arc<WorldSession>,
        mut packet: WorldPacket,
    ) -> Result<()> {
        use crate::world::core::session::SessionState;
        use crate::world::handlers::dispatch_packet;

        let opcode_for_log = packet.opcode();
        match session.state() {
            SessionState::LoggedIn => {
                // Player in-world, route to handler task
                if let Some(player_guid) = session.player_guid() {
                    self.route_to_player_handler(player_guid, packet).await?;
                } else {
                    warn!("LoggedIn state but no player GUID");
                }
            }
            SessionState::Authenticated => {
                let opcode = packet.opcode();

                if opcode == Opcode::CMSG_PLAYER_LOGIN {
                    // Spawn login as a separate task to keep socket responsive (~157ms)
                    if !session.try_start_login() {
                        warn!("Login already in progress for session {}", session.id());
                        return Ok(());
                    }

                    let guid_raw = packet
                        .read_u64()
                        .ok_or_else(|| anyhow::anyhow!("Failed to read GUID for login"))?;
                    let session_clone = Arc::clone(&session);
                    let databases = Arc::clone(&self.databases);
                    let world = self.world.clone();

                    tokio::spawn(async move {
                        use crate::world::handlers::character;
                        if let Err(e) = character::handle_player_login_with_guid(
                            &session_clone,
                            guid_raw,
                            &databases,
                            &world,
                        )
                        .await
                        {
                            error!("[LOGIN] Spawned login task failed: {}", e);
                            session_clone.clear_login_in_progress();
                        }
                    });
                } else {
                    // Other authenticated packets processed inline (fast operations)
                    dispatch_packet(&session, &mut packet, &self.databases, &self.world).await?;
                }
            }
            _ => {
                debug!(
                    "Packet {:?} received in unexpected state {:?}",
                    packet.opcode(),
                    session.state()
                );
            }
        }
        Ok(())
    }

    /// Route packet to player's handler task
    async fn route_to_player_handler(
        &self,
        player_guid: ObjectGuid,
        packet: WorldPacket,
    ) -> Result<()> {
        let opcode = packet.opcode();

        // All packets (including movement) go to the handler task for immediate processing
        if let Some(handler) = self.world.player_handlers.get(&player_guid) {
            handler.send(packet).await?;
        } else {
            warn!(
                "No handler found for player {}, packet {:?} dropped",
                player_guid, opcode
            );
        }
        Ok(())
    }

    /// Perform cleanup when a client disconnects
    async fn handle_disconnect(&self, reason: &str) {
        info!(
            "[DISCONNECT] Client disconnected ({}): {}",
            reason, self.remote_addr
        );

        // Perform logout cleanup if player is logged in
        if let Some(session_id) = self.session_id {
            if let Some(session) = self.session_mgr.get_session(session_id) {
                if let Some(player_guid) = session.player_guid() {
                    // Log broadcaster state before cleanup
                    if let Some(broadcaster) =
                        self.world.managers.player_mgr.get_broadcaster(player_guid)
                    {
                        info!(
                            "[DISCONNECT] Player {} had {} listeners before disconnect",
                            player_guid,
                            broadcaster.listener_count()
                        );
                        let listeners_lock = broadcaster.listeners().read();
                        let listeners = listeners_lock.keys().cloned().collect::<Vec<_>>();
                        info!(
                            "[DISCONNECT] Player {} listeners: {:?}",
                            player_guid, listeners
                        );
                    }

                    info!(
                        "[DISCONNECT] Performing logout cleanup for player {}",
                        player_guid
                    );

                    // Remove player handler and movement buffer before other cleanup
                    self.world.remove_player_handler(player_guid);
                    self.world.remove_movement_buffer(player_guid);

                    // Perform instant logout on disconnect
                    if let Err(e) = crate::world::handlers::character::perform_logout_cleanup(
                        &session,
                        &self.world,
                    )
                    .await
                    {
                        error!(
                            "[DISCONNECT] Failed to cleanup player {}: {}",
                            player_guid, e
                        );
                    }
                }

                // Remove session
                self.session_mgr.remove_session(session_id);
            }
        }
    }

    /// Send auth challenge (SMSG_AUTH_CHALLENGE)
    async fn send_auth_challenge(&mut self) -> Result<()> {
        use crate::shared::messages::login::SmsgAuthChallenge;

        let challenge = SmsgAuthChallenge {
            seed: self.server_seed,
        };
        self.send_msg(challenge).await
    }

    /// Handle CMSG_AUTH_SESSION
    async fn handle_auth_session(&mut self, mut packet: WorldPacket) -> Result<()> {
        // Call auth handler with all security checks
        let auth_result = auth_handler::handle_auth_session(
            self.remote_addr,
            packet,
            self.server_seed,
            &self.databases,
            &self.session_mgr,
        )
        .await?;

        match auth_result {
            auth_handler::AuthResult::Success {
                account_id,
                account_name,
                security,
                session_key,
            } => {
                // Initialize encryption
                self.init_crypt(&session_key);

                // Store account info
                self.account_id = Some(account_id);
                self.account_name = Some(account_name.clone());
                self.security = security;

                // Generate session ID
                let session_id = self.session_mgr.generate_session_id();
                self.session_id = Some(session_id);

                // Create packet channels (unbounded to guarantee no packet drops)
                let (session_to_socket_tx, session_to_socket_rx) = mpsc::unbounded_channel();
                self.packet_rx = Some(session_to_socket_rx);

                // Create WorldSession
                let session = WorldSession::new(
                    session_id,
                    account_id,
                    account_name.clone(),
                    security,
                    session_to_socket_tx,
                );

                // Register in SessionManager
                self.session_mgr.add_session(Arc::new(session));

                // Send success response
                self.send_auth_response_success().await?;

                debug!(
                    "Authentication successful for account '{}' (ID: {})",
                    account_name, account_id
                );
                Ok(())
            }
            auth_handler::AuthResult::Error { error_code } => {
                // Send error response, don't transition to authenticated
                self.send_auth_response(error_code).await?;
                Err(anyhow::anyhow!(
                    "Authentication failed with code: 0x{:02x}",
                    error_code
                ))
            }
        }
    }

    /// Send SMSG_AUTH_RESPONSE (success)
    /// Note: This is sent ENCRYPTED after crypt initialization (checked by send_packet)
    async fn send_auth_response_success(&mut self) -> Result<()> {
        use crate::shared::messages::login::{AuthErrorCode, SmsgAuthResponse};

        debug!(
            "[AUTH] Sending SMSG_AUTH_RESPONSE (success, encrypted={}) to {}",
            self.crypt.is_initialized(),
            self.remote_addr
        );
        let response = SmsgAuthResponse {
            error_code: AuthErrorCode::Ok as u8,
        };
        let packet_data = response.to_world_packet();
        let mut packet = WorldPacket::new(packet_data.opcode());
        packet.write_bytes(packet_data.contents());

        debug!(
            "[AUTH] SMSG_AUTH_RESPONSE packet size: {} bytes, opcode: {:?}",
            packet.size(),
            packet.opcode()
        );
        self.send_packet(&packet).await?;
        debug!("[AUTH] SMSG_AUTH_RESPONSE sent successfully");
        Ok(())
    }

    /// Send SMSG_AUTH_RESPONSE (error case)
    async fn send_auth_response(&mut self, error_code: u8) -> Result<()> {
        use crate::shared::messages::login::SmsgAuthResponse;

        let response = SmsgAuthResponse { error_code };
        let packet_data = response.to_world_packet();
        let mut packet = WorldPacket::new(packet_data.opcode());
        packet.write_bytes(packet_data.contents());

        // Error responses are always sent unencrypted (before crypt init)
        self.send_packet_unencrypted(&packet).await
    }

    /// Read a packet from the buffer (unencrypted)
    fn read_packet_unencrypted(&mut self) -> Result<Option<WorldPacket>> {
        if self.read_buffer.len() < CLIENT_HEADER_SIZE {
            return Ok(None);
        }

        // Parse header (unencrypted)
        let header = &self.read_buffer[..CLIENT_HEADER_SIZE];
        let (size, opcode) = match parse_client_header(header) {
            Some(result) => result,
            None => return Ok(None),
        };

        // Calculate total packet size
        let payload_size = client_payload_size(size);
        let total_size = CLIENT_HEADER_SIZE + payload_size;

        if self.read_buffer.len() < total_size {
            return Ok(None);
        }

        // Validate size
        if total_size > MAX_PACKET_SIZE {
            return Err(anyhow::anyhow!("Packet too large: {} bytes", total_size));
        }

        // Extract packet data
        self.read_buffer.advance(CLIENT_HEADER_SIZE);
        let data = self.read_buffer.split_to(payload_size);

        Ok(Some(WorldPacket::from_data(opcode, data)))
    }

    /// Read a packet from the buffer (encrypted)
    fn read_packet_encrypted(&mut self) -> Result<Option<WorldPacket>> {
        if self.read_buffer.len() < CLIENT_HEADER_SIZE {
            return Ok(None);
        }

        // Decrypt header in place
        let mut header = [0u8; CLIENT_HEADER_SIZE];
        header.copy_from_slice(&self.read_buffer[..CLIENT_HEADER_SIZE]);
        self.crypt.decrypt_recv(&mut header);

        // Parse decrypted header
        let (size, opcode) = match parse_client_header(&header) {
            Some(result) => result,
            None => return Ok(None),
        };

        // Calculate total packet size
        let payload_size = client_payload_size(size);
        let total_size = CLIENT_HEADER_SIZE + payload_size;

        if self.read_buffer.len() < total_size {
            return Ok(None);
        }

        // Validate size
        if total_size > MAX_PACKET_SIZE {
            return Err(anyhow::anyhow!("Packet too large: {} bytes", total_size));
        }

        // Remove header from buffer (already decrypted via copy)
        self.read_buffer.advance(CLIENT_HEADER_SIZE);
        let data = self.read_buffer.split_to(payload_size);

        Ok(Some(WorldPacket::from_data(opcode, data)))
    }

    /// Send a packet (encrypted if crypt is initialized)
    async fn send_packet(&mut self, packet: &WorldPacket) -> Result<()> {
        let size = server_packet_size(packet.size());
        let mut header = build_server_header(size, packet.opcode());

        // Encrypt header only if crypt is initialized
        if self.crypt.is_initialized() {
            self.crypt.encrypt_send(&mut header);
        }

        // Combine header + data into single write to avoid extra TCP segments
        let mut buf = Vec::with_capacity(header.len() + packet.data().len());
        buf.extend_from_slice(&header);
        buf.extend_from_slice(packet.data());
        self.stream.write_all(&buf).await?;

        Ok(())
    }

    /// Send a packet (unencrypted, for auth phase)
    async fn send_packet_unencrypted(&mut self, packet: &WorldPacket) -> Result<()> {
        let size = server_packet_size(packet.size());
        let header = build_server_header(size, packet.opcode());

        // Combine header + data into single write to avoid extra TCP segments
        let mut buf = Vec::with_capacity(header.len() + packet.data().len());
        buf.extend_from_slice(&header);
        buf.extend_from_slice(packet.data());
        self.stream.write_all(&buf).await?;

        Ok(())
    }

    /// Send a message struct (using ToWorldPacket trait)
    async fn send_msg(&mut self, msg: impl ToWorldPacket) -> Result<()> {
        let packet_data = msg.to_world_packet();
        let mut packet = WorldPacket::new(packet_data.opcode());
        packet.write_bytes(packet_data.contents());
        self.send_packet(&packet).await
    }
}
