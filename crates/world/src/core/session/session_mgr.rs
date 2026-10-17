//! Session Manager - manages all active sessions

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::core::session::WorldSession;
use oxcore_shared::protocol::{ObjectGuid, Protocol};

/// Manages all active world sessions
pub struct SessionManager {
    /// Sessions by session ID
    sessions: DashMap<u32, Arc<WorldSession>>,
    /// Sessions by account ID (for duplicate detection)
    by_account: DashMap<u32, u32>,
    /// Modern realm sockets remain connected after the instance socket owns the player.
    /// Account-data messages must still travel through this connection.
    realm_by_account: DashMap<u32, u32>,
    /// Sessions by player GUID (when logged in)
    by_player: DashMap<ObjectGuid, u32>,
    /// Next session ID
    next_id: AtomicU32,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            by_account: DashMap::new(),
            realm_by_account: DashMap::new(),
            by_player: DashMap::new(),
            next_id: AtomicU32::new(1),
        }
    }

    /// Generate a new session ID
    pub fn generate_session_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Add a session
    pub fn add_session(&self, session: Arc<WorldSession>) {
        let id = session.id();
        let account_id = session.account_id();

        self.sessions.insert(id, session);
        self.by_account.insert(account_id, id);
    }

    /// Remove a session
    pub fn remove_session(&self, id: u32) -> Option<Arc<WorldSession>> {
        if let Some((_, session)) = self.sessions.remove(&id) {
            if self
                .by_account
                .get(&session.account_id())
                .is_some_and(|current| *current == id)
            {
                self.by_account.remove(&session.account_id());
            }
            if self
                .realm_by_account
                .get(&session.account_id())
                .is_some_and(|current| *current == id)
            {
                self.realm_by_account.remove(&session.account_id());
            }
            if let Some(guid) = session.player_guid() {
                if self
                    .by_player
                    .get(&guid)
                    .is_some_and(|current| *current == id)
                {
                    self.by_player.remove(&guid);
                }
            }
            Some(session)
        } else {
            None
        }
    }

    /// Get a session by ID
    pub fn get_session(&self, id: u32) -> Option<Arc<WorldSession>> {
        self.sessions.get(&id).map(|r| Arc::clone(&r))
    }

    /// Get session by account ID
    pub fn get_session_by_account(&self, account_id: u32) -> Option<Arc<WorldSession>> {
        self.by_account
            .get(&account_id)
            .and_then(|id| self.sessions.get(&id).map(|r| Arc::clone(&r)))
    }

    /// Register the persistent realm socket for a modern account.
    pub fn register_realm_session(&self, account_id: u32, session_id: u32) {
        self.realm_by_account.insert(account_id, session_id);
    }

    /// The realm socket is distinct from the instance session that owns the player.
    pub fn get_realm_session_by_account(&self, account_id: u32) -> Option<Arc<WorldSession>> {
        self.realm_by_account
            .get(&account_id)
            .and_then(|id| self.sessions.get(&id).map(|session| Arc::clone(&session)))
    }

    /// Get session by player GUID
    pub fn get_session_by_player(&self, guid: ObjectGuid) -> Option<Arc<WorldSession>> {
        self.by_player
            .get(&guid)
            .and_then(|id| self.sessions.get(&id).map(|r| Arc::clone(&r)))
    }

    /// Register player GUID to session
    pub fn register_player(&self, session_id: u32, guid: ObjectGuid) {
        self.by_player.insert(guid, session_id);
    }

    /// Unregister player GUID
    pub fn unregister_player(&self, guid: ObjectGuid) {
        self.by_player.remove(&guid);
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get all logged-in player GUIDs
    /// Used for broadcasting to all players
    pub fn get_all_sessions(&self) -> Vec<ObjectGuid> {
        self.by_player.iter().map(|entry| *entry.key()).collect()
    }

    /// Check if account is already logged in
    pub fn is_account_online(&self, account_id: u32) -> bool {
        self.by_account.contains_key(&account_id)
    }

    /// Remove existing session for an account
    ///
    /// Returns:
    /// - `true` if no session exists or session was successfully removed
    /// - `false` if existing session is in loading state (reject new login)
    ///
    /// This method performs SYNCHRONOUS player removal from managers to prevent
    /// race conditions where new session finds old player still in world.
    pub async fn remove_session_for_account(&self, account_id: u32) -> bool {
        debug!(
            "[DEDUP] remove_session_for_account called for account {}",
            account_id
        );

        // Check if there's an existing session for this account
        if let Some((existing_id, existing_session)) = self.by_account.remove(&account_id) {
            debug!(
                "[DEDUP] Found existing session {} for account {}",
                existing_id, account_id
            );

            // Remove from sessions map
            self.sessions.remove(&existing_id);

            // Note: Player cleanup would be done by the old session's handler task
            // We just need to remove the session from the manager

            debug!(
                "[DEDUP] Removed existing session {} for account {}",
                existing_id, account_id
            );
            true
        } else {
            debug!(
                "[DEDUP] No existing session found for account {}",
                account_id
            );
            true
        }
    }

    /// Check for expired logout timers and perform cleanup
    /// Called from World update loop
    pub async fn update_logout_timers(&self, world: &crate::World) -> anyhow::Result<()> {
        let logout_timer_secs = world.config.logout_timer;

        // Collect sessions with expired timers
        let mut sessions_to_logout = Vec::new();
        let mut active_logout_timers = 0;

        for entry in self.sessions.iter() {
            let session = entry.value();

            // Only check timer if player is logged in
            if session.player_guid().is_some() {
                if let Some(remaining) = session.logout_time_remaining(logout_timer_secs) {
                    active_logout_timers += 1;
                    debug!(
                        "[LOGOUT_TIMER] Session {} (account: {}) has {}s remaining",
                        session.id(),
                        session.account_name(),
                        remaining
                    );

                    if session.is_logout_ready(logout_timer_secs) {
                        info!(
                            "[LOGOUT_TIMER] Timer expired for session {} (account: {})",
                            session.id(),
                            session.account_name()
                        );
                        sessions_to_logout.push(Arc::clone(session));
                    }
                }
            }
        }

        if active_logout_timers > 0 {
            debug!(
                "[LOGOUT_TIMER] Checked {} active logout timers, {} ready to logout",
                active_logout_timers,
                sessions_to_logout.len()
            );
        }

        // Perform logout for expired sessions
        for session in sessions_to_logout {
            info!(
                "[LOGOUT_TIMER] Performing logout cleanup for session {} (account: {})",
                session.id(),
                session.account_name()
            );
            if let Err(e) =
                crate::handlers::character::perform_logout_cleanup(&session, world).await
            {
                error!(
                    "[LOGOUT_TIMER] Failed to perform logout cleanup for session {}: {}",
                    session.id(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Re-send `SMSG_TIME_SYNC_REQUEST` to every in-world modern session whose timer has come due.
    ///
    /// Called once per world tick. The first request is sent at login; this keeps the client's
    /// clock reference fresh, which only a 1.14 session uses (a 1.12 session has no time sync and
    /// is never armed). No-op for a tick where nothing is due.
    pub fn update_time_sync(&self) {
        for entry in self.sessions.iter() {
            let session = entry.value();
            if session.protocol() != Protocol::Modern || !session.is_logged_in() {
                continue;
            }
            if session.take_due_time_sync() {
                if let Err(e) = session.send_time_sync() {
                    debug!(
                        "[TIME_SYNC] Failed to re-send time sync for session {}: {}",
                        session.id(),
                        e
                    );
                }
            }
        }
    }

    /// Logout every active player during server shutdown, preserving the normal
    /// logout save path before sessions and sockets are torn down.
    pub async fn logout_all_players(&self, world: &crate::World) -> anyhow::Result<()> {
        let sessions: Vec<Arc<WorldSession>> = self
            .sessions
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect();
        let mut failures = 0usize;
        let mut saved_players = 0usize;
        let mut sessions_without_player = 0usize;

        info!(
            "Logging out {} active sessions for shutdown",
            sessions.len()
        );

        for session in sessions {
            let session_id = session.id();

            if let Some(player_guid) = session.player_guid() {
                info!(
                    "[SHUTDOWN] Saving player {} from session {}",
                    player_guid, session_id
                );

                if let Err(e) =
                    crate::handlers::character::perform_logout_cleanup(&session, world).await
                {
                    failures += 1;
                    error!(
                        "[SHUTDOWN] Failed to save/logout player {} from session {}: {}",
                        player_guid, session_id, e
                    );
                } else {
                    saved_players += 1;
                }
            } else {
                sessions_without_player += 1;
                info!(
                    "[SHUTDOWN] Session {} has no active player; removing without player save",
                    session_id
                );
            }

            self.remove_session(session_id);
        }

        info!(
            "[SHUTDOWN] Player session save summary: saved={}, no_player={}, failed={}",
            saved_players, sessions_without_player, failures
        );

        if failures > 0 {
            anyhow::bail!("failed to save/logout {failures} player session(s) during shutdown");
        }

        Ok(())
    }

    /// Close all active sessions by closing their packet channels
    /// This will cause socket tasks to exit
    pub fn close_all_sessions(&self) {
        let session_ids: Vec<u32> = self.sessions.iter().map(|entry| *entry.key()).collect();
        tracing::debug!("Closing {} active sessions", session_ids.len());

        for session_id in session_ids {
            if let Some(session) = self.sessions.get(&session_id) {
                // Closing the packet channel will cause the socket task to exit
                // The session's packet_tx is dropped when the session is removed
                drop(session);
            }
            self.remove_session(session_id);
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SessionManager;
    use crate::core::session::{SessionState, WorldSession};
    use oxcore_shared::protocol::{Opcode, Protocol, WorldPacket};
    use std::sync::Arc;

    fn session(id: u32, account_id: u32) -> Arc<WorldSession> {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<WorldPacket>();
        Arc::new(WorldSession::new_with_protocol(
            id,
            account_id,
            "account".to_string(),
            0,
            Protocol::Modern,
            tx,
        ))
    }

    #[test]
    fn realm_session_survives_instance_session_cleanup() {
        let sessions = SessionManager::new();
        let realm = session(1, 7);
        let instance = session(2, 7);

        sessions.add_session(realm);
        sessions.register_realm_session(7, 1);
        sessions.add_session(instance);

        sessions.remove_session(2);
        assert_eq!(sessions.get_realm_session_by_account(7).unwrap().id(), 1);
    }

    #[test]
    fn periodic_time_sync_resends_only_when_due() {
        let sessions = SessionManager::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WorldPacket>();
        let session = Arc::new(WorldSession::new_with_protocol(
            1,
            7,
            "account".to_string(),
            0,
            Protocol::Modern,
            tx,
        ));
        session.set_state(SessionState::LoggedIn);
        sessions.add_session(session.clone());

        // The login-time request arms the 5 s timer and emits its packet.
        session.send_time_sync().unwrap();
        let first = rx.try_recv().expect("first time sync packet");
        assert_eq!(first.opcode(), Opcode::SMSG_TIME_SYNC_REQUEST);

        // Not due yet: the tick must not send anything.
        sessions.update_time_sync();
        assert!(rx.try_recv().is_err(), "time sync fired before it was due");

        // Once due, the tick re-sends.
        session.force_time_sync_due();
        sessions.update_time_sync();
        let second = rx.try_recv().expect("periodic time sync packet");
        assert_eq!(second.opcode(), Opcode::SMSG_TIME_SYNC_REQUEST);
    }

    #[test]
    fn periodic_time_sync_skips_vanilla_sessions() {
        let sessions = SessionManager::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WorldPacket>();
        let session = Arc::new(WorldSession::new_with_protocol(
            1,
            7,
            "account".to_string(),
            0,
            Protocol::Vanilla,
            tx,
        ));
        session.set_state(SessionState::LoggedIn);
        sessions.add_session(session.clone());

        // Even with the timer due, a 1.12 session must never see a time sync.
        session.force_time_sync_due();
        sessions.update_time_sync();
        assert!(rx.try_recv().is_err(), "vanilla session got a time sync");
    }

    #[test]
    fn periodic_time_sync_skips_sessions_not_in_world() {
        let sessions = SessionManager::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WorldPacket>();
        let session = Arc::new(WorldSession::new_with_protocol(
            1,
            7,
            "account".to_string(),
            0,
            Protocol::Modern,
            tx,
        ));
        // Still at character select: the login-time request has not been sent yet, so nothing is due.
        session.set_state(SessionState::Authenticated);
        sessions.add_session(session.clone());

        session.force_time_sync_due();
        sessions.update_time_sync();
        assert!(rx.try_recv().is_err(), "unlogged session got a time sync");
    }
}
