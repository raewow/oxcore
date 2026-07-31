//! World Session - represents a player connection

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::core::session::SessionState;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Position, Protocol, WorldPacket};

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
    /// Client protocol used to select message bodies and wire opcodes.
    protocol: Protocol,
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
    /// Guard against concurrent auction-house list requests
    auction_list_request_in_progress: AtomicBool,
    /// Pending area trigger teleport (dest_map, dest_instance_id, dest_pos)
    pending_teleport: Arc<RwLock<Option<(u32, u32, Position)>>>,
    /// Pending same-map teleport destination, completed by MSG_MOVE_TELEPORT_ACK.
    pending_near_teleport: Arc<RwLock<Option<Position>>>,
    /// GUID the client currently believes it is controlling.
    /// `None`/empty means the player itself. Set via CMSG_SET_ACTIVE_MOVER.
    client_mover_guid: RwLock<Option<ObjectGuid>>,
    /// Server time (ms) before which movement packets are rejected.
    move_reject_time: AtomicU32,
    /// The logout flow has sent SMSG_FORCE_MOVE_ROOT and awaits its client ACK.
    pending_root_ack: AtomicBool,
    /// Sequence index for the next `SMSG_TIME_SYNC_REQUEST`. Modern-only; a 1.12 client never sees a time sync.
    time_sync_next_index: AtomicU32,
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
        Self::new_with_protocol(
            id,
            account_id,
            account_name,
            security,
            Protocol::Vanilla,
            packet_tx,
        )
    }

    /// Create a session for a specific client protocol.
    pub fn new_with_protocol(
        id: u32,
        account_id: u32,
        account_name: String,
        security: u8,
        protocol: Protocol,
        packet_tx: mpsc::UnboundedSender<WorldPacket>,
    ) -> Self {
        Self {
            id,
            account_id,
            account_name,
            security,
            protocol,
            state: RwLock::new(SessionState::Authenticated),
            packet_tx,
            player_guid: RwLock::new(None),
            logout_timer: RwLock::new(None),
            login_in_progress: AtomicBool::new(false),
            time_sync_next_index: AtomicU32::new(0),
            auction_list_request_in_progress: AtomicBool::new(false),
            pending_teleport: Arc::new(RwLock::new(None)),
            pending_near_teleport: Arc::new(RwLock::new(None)),
            client_mover_guid: RwLock::new(None),
            move_reject_time: AtomicU32::new(0),
            pending_root_ack: AtomicBool::new(false),
        }
    }

    /// GUID the client currently controls. Returns the player's own GUID when no
    /// alternate mover has been set.
    pub fn client_mover_guid(&self) -> Option<ObjectGuid> {
        match *self.client_mover_guid.read() {
            Some(guid) => Some(guid),
            None => self.player_guid(),
        }
    }

    /// Set the active mover GUID. Pass `None` to clear.
    pub fn set_client_mover_guid(&self, guid: Option<ObjectGuid>) {
        *self.client_mover_guid.write() = guid;
    }

    /// Resolve a mover GUID the client claims to control. Without pet/possession support the only
    /// valid mover is the player itself.
    pub fn get_mover_from_guid(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        let player = self.player_guid()?;
        if guid.counter() == player.counter() {
            return Some(player);
        }
        // The client may still address the player by the active-mover GUID.
        if let Some(mover) = *self.client_mover_guid.read() {
            if guid.counter() == mover.counter() {
                return Some(player);
            }
        }
        None
    }

    /// Current move-reject timestamp (ms).
    pub fn move_reject_time(&self) -> u32 {
        self.move_reject_time.load(Ordering::Relaxed)
    }

    /// Reject movement packets with a client timestamp at or before `time_ms`.
    pub fn set_move_reject_time(&self, time_ms: u32) {
        self.move_reject_time.store(time_ms, Ordering::Relaxed);
    }

    /// Reject incoming movement packets for the next `ms` milliseconds.
    pub fn reject_movement_packets_for(&self, ms: u32) {
        let timeout = crate::core::common::get_ms_time().wrapping_add(ms);
        if self.move_reject_time() < timeout {
            self.set_move_reject_time(timeout);
        }
    }

    /// Record that the logout root packet awaits its movement acknowledgement.
    pub fn set_pending_root_ack(&self, pending: bool) {
        self.pending_root_ack.store(pending, Ordering::Relaxed);
    }

    /// Consume a pending logout root acknowledgement.
    pub fn take_pending_root_ack(&self) -> bool {
        self.pending_root_ack.swap(false, Ordering::Relaxed)
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

    /// Client protocol selected during authentication.
    pub fn protocol(&self) -> Protocol {
        self.protocol
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

    /// Send a hand-built packet to the client.
    ///
    /// **Refused for a modern session.** A `WorldPacket` assembled by a handler is vanilla bytes —
    /// nothing in the type records which protocol produced it — and putting those on a 1.14
    /// connection is silent corruption the client can only report as a disconnect. Only the opcode
    /// having a modern wire value is not evidence the *body* is right.
    ///
    /// This is the session-level counterpart of `PlayerBroadcaster::accepts_prebuilt`, which has
    /// guarded the broadcast path for a while; this path had no guard at all, which is Stage 0 item 1
    /// of `docs/modern-opcode-plan.md`.
    ///
    /// Use [`Self::send_msg`] with a real `to_modern`, or — when the body genuinely is identical for
    /// both clients — [`Self::send_packet_protocol_agnostic`].
    pub(crate) fn send_packet(&self, packet: WorldPacket) -> anyhow::Result<()> {
        if self.protocol == Protocol::Modern {
            tracing::warn!(
                "Refusing hand-built {:?} for modern player {:?}: the body is vanilla-only. \
                 Send the message type instead, or use send_packet_protocol_agnostic if the body \
                 is verified identical for both protocols.",
                packet.opcode(),
                self.player_guid()
            );
            return Ok(());
        }
        self.deliver(packet)
    }

    /// Send a hand-built packet whose body is the same for both protocols.
    ///
    /// For the empty and count-prefixed replies the 1.14 client demands during its bootstrap sweep:
    /// a bare `i32 0` is a bare `i32 0` either way. Every call site here has been checked to send a
    /// body that is genuinely identical for both protocols — check yours before adding one, because
    /// this bypasses the guard in
    /// [`Self::send_packet`] and a wrong body desynchronises every packet after it.
    pub(crate) fn send_packet_protocol_agnostic(&self, packet: WorldPacket) -> anyhow::Result<()> {
        self.deliver(packet)
    }

    /// Hand an already-encoded packet to the socket. No protocol check: the caller has established
    /// that this body belongs on this connection.
    fn deliver(&self, packet: WorldPacket) -> anyhow::Result<()> {
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
        let packet = match self.protocol {
            Protocol::Vanilla => msg.to_vanilla(),
            Protocol::Modern => match msg.to_modern() {
                Some(packet) => packet,
                None => {
                    // Name it: an unnamed drop is unactionable when a client renders a
                    // half-populated world. Costs a vanilla encode, but only on the drop path.
                    tracing::debug!(
                        "No modern encoding for {:?}; dropping",
                        msg.to_vanilla().opcode()
                    );
                    return Ok(());
                }
            },
        };
        self.deliver(packet)
    }

    /// Atomically try to mark login as in progress (prevents concurrent logins)
    pub fn try_start_login(&self) -> bool {
        self.login_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Take the next time-sync sequence index, so a response can be matched to its request.
    ///
    /// Increments after each send. The client echoes whatever it was given, so a
    /// monotonic counter still matches responses uniquely.
    pub fn next_time_sync_index(&self) -> u32 {
        self.time_sync_next_index.fetch_add(1, Ordering::Relaxed)
    }

    /// Clear the login-in-progress flag
    pub fn clear_login_in_progress(&self) {
        self.login_in_progress.store(false, Ordering::SeqCst);
    }

    /// Check whether an auction-house list request is already in flight.
    pub fn received_ah_list_request(&self) -> bool {
        self.auction_list_request_in_progress.load(Ordering::SeqCst)
    }

    /// Update the auction-house list in-flight gate.
    pub fn set_received_ah_list_request(&self, value: bool) {
        self.auction_list_request_in_progress
            .store(value, Ordering::SeqCst);
    }

    /// Clear the auction-house list gate.
    pub fn clear_received_ah_list_request(&self) {
        self.set_received_ah_list_request(false);
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
            tracing::debug!("[LOGOUT_TIMER] Timer cancelled for session {}", self.id);
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

    /// Set the destination for a same-map teleport awaiting MSG_MOVE_TELEPORT_ACK.
    pub fn set_pending_near_teleport(&self, position: Option<Position>) {
        *self.pending_near_teleport.write() = position;
    }

    /// Get the destination for a same-map teleport awaiting MSG_MOVE_TELEPORT_ACK.
    pub fn get_pending_near_teleport(&self) -> Option<Position> {
        *self.pending_near_teleport.read()
    }

    /// Clear the pending same-map teleport destination.
    pub fn clear_pending_near_teleport(&self) {
        *self.pending_near_teleport.write() = None;
    }
}
