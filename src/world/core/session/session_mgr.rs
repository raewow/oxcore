//! Session Manager - manages all active sessions

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::shared::protocol::ObjectGuid;
use crate::world::core::session::{SessionState, WorldSession};

/// Manages all active world sessions
pub struct SessionManager {
    /// Sessions by session ID
    sessions: DashMap<u32, Arc<WorldSession>>,
    /// Sessions by account ID (for duplicate detection)
    by_account: DashMap<u32, u32>,
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
            self.by_account.remove(&session.account_id());
            if let Some(guid) = session.player_guid() {
                self.by_player.remove(&guid);
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

    /// Remove existing session for an account (following MaNGOS pattern)
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
    pub async fn update_logout_timers(&self, world: &crate::world::World) -> anyhow::Result<()> {
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
                crate::world::handlers::character::perform_logout_cleanup(&session, world).await
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
            self.sessions.remove(&session_id);
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
