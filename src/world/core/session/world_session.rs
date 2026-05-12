//! World Session - represents a player connection

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Position, WorldPacket};
use crate::world::core::session::SessionState;

/// World session - represents an authenticated player connection
pub struct WorldSession {
    /// Session ID
    id: u32,
    /// Account ID
    account_id: u32,
    /// Account name
    account_name: String,
    /// GM security level
    security: u8,
    /// Current state (uses interior mutability for shared access)
    state: RwLock<SessionState>,
    /// Channel to send packets to the socket (unbounded for no packet drops)
    packet_tx: mpsc::UnboundedSender<WorldPacket>,
    /// Player GUID (when logged in, uses interior mutability)
    player_guid: RwLock<Option<ObjectGuid>>,
    /// Logout timer (for countdown)
    logout_timer: RwLock<Option<std::time::Instant>>,
    /// Guard against concurrent login attempts
    login_in_progress: AtomicBool,
    /// Pending area trigger teleport (dest_map, dest_instance_id, dest_pos)
    pending_teleport: Arc<RwLock<Option<(u32, u32, Position)>>>,
}

impl WorldSession {
    /// Create a new session
    pub fn new(
        id: u32,
        account_id: u32,
        account_name: String,
        security: u8,
        packet_tx: mpsc::UnboundedSender<WorldPacket>,
    ) -> Self {
        Self {
            id,
            account_id,
            account_name,
            security,
            state: RwLock::new(SessionState::Authenticated),
            packet_tx,
            player_guid: RwLock::new(None),
            logout_timer: RwLock::new(None),
            login_in_progress: AtomicBool::new(false),
            pending_teleport: Arc::new(RwLock::new(None)),
        }
    }

    /// Get session ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get account ID
    pub fn account_id(&self) -> u32 {
        self.account_id
    }

    /// Get account name
    pub fn account_name(&self) -> &str {
        &self.account_name
    }

    /// Get security level
    pub fn security(&self) -> u8 {
        self.security
    }

    /// Get current state
    pub fn state(&self) -> SessionState {
        *self.state.read()
    }

    /// Set state
    pub fn set_state(&self, state: SessionState) {
        *self.state.write() = state;
    }

    /// Get player GUID
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        *self.player_guid.read()
    }

    /// Set player GUID
    pub fn set_player_guid(&self, guid: Option<ObjectGuid>) {
        *self.player_guid.write() = guid;
    }

    /// Get packet sender channel (for PlayerBroadcaster)
    pub fn packet_tx(&self) -> mpsc::UnboundedSender<WorldPacket> {
        self.packet_tx.clone()
    }

    /// Send a packet to the client (internal use only)
    /// External code should use send_msg for type-safe messaging
    /// Note: This is now synchronous (unbounded channel send never blocks)
    pub(crate) fn send_packet(&self, packet: WorldPacket) -> anyhow::Result<()> {
        tracing::trace!(
            "[PKT-OUT] opcode={:?} len={}",
            packet.opcode(),
            packet.size()
        );
        self.packet_tx
            .send(packet)
            .map_err(|_| anyhow::anyhow!("Failed to send packet (channel closed)"))
    }

    /// Send a message struct to the client
    /// Note: This is now synchronous (unbounded channel send never blocks)
    pub fn send_msg(&self, msg: impl ToWorldPacket) -> anyhow::Result<()> {
        let packet_data: WorldPacket = msg.to_world_packet();
        let mut packet = WorldPacket::new(packet_data.opcode());
        packet.write_bytes(packet_data.contents());
        self.send_packet(packet)
    }

    /// Atomically try to mark login as in progress (prevents concurrent logins)
    pub fn try_start_login(&self) -> bool {
        self.login_in_progress.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    /// Clear the login-in-progress flag
    pub fn clear_login_in_progress(&self) {
        self.login_in_progress.store(false, Ordering::SeqCst);
    }

    /// Check if logged in
    pub fn is_logged_in(&self) -> bool {
        *self.state.read() == SessionState::LoggedIn
    }

    /// Check if player is currently loading
    pub fn is_player_loading(&self) -> bool {
        self.player_guid.read().is_none() && *self.state.read() == SessionState::LoggedIn
    }

    /// Start logout timer
    pub fn start_logout_timer(&self) {
        let now = std::time::Instant::now();
        *self.logout_timer.write() = Some(now);
        tracing::debug!(
            "[LOGOUT_TIMER] Timer started for session {} at {:?}",
            self.id,
            now
        );
    }

    /// Cancel logout timer
    pub fn cancel_logout_timer(&self) {
        let had_timer = self.logout_timer.read().is_some();
        *self.logout_timer.write() = None;
        if had_timer {
            tracing::debug!(
                "[LOGOUT_TIMER] Timer cancelled for session {}",
                self.id
            );
        }
    }

    /// Check if logout timer has expired
    pub fn is_logout_ready(&self, logout_timer_secs: u32) -> bool {
        if let Some(timer_start) = *self.logout_timer.read() {
            timer_start.elapsed().as_secs() >= logout_timer_secs as u64
        } else {
            false
        }
    }

    /// Get remaining logout time in seconds (returns None if no timer active)
    pub fn logout_time_remaining(&self, logout_timer_secs: u32) -> Option<u32> {
        if let Some(timer_start) = *self.logout_timer.read() {
            let elapsed = timer_start.elapsed().as_secs();
            if elapsed < logout_timer_secs as u64 {
                Some((logout_timer_secs as u64 - elapsed) as u32)
            } else {
                Some(0)
            }
        } else {
            None
        }
    }

    /// Set pending teleport destination (used by area trigger handler)
    pub fn set_pending_teleport(&self, teleport: Option<(u32, u32, Position)>) {
        *self.pending_teleport.write() = teleport;
    }

    /// Get pending teleport destination (used by worldport ACK handler)
    pub fn get_pending_teleport(&self) -> Option<(u32, u32, Position)> {
        *self.pending_teleport.read()
    }

    /// Clear pending teleport destination
    pub fn clear_pending_teleport(&self) {
        *self.pending_teleport.write() = None;
    }
}
