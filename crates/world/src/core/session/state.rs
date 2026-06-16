//! Session state machine

/// Session states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Just connected, not authenticated
    Connected,
    /// Authenticated, at character select
    Authenticated,
    /// In world, playing
    LoggedIn,
    /// Logging out (timer active)
    LoggingOut,
}

impl SessionState {
    /// Can handle character-selection packets?
    pub fn can_handle_char_select(&self) -> bool {
        matches!(self, SessionState::Authenticated)
    }

    /// Can handle in-world packets?
    pub fn can_handle_world(&self) -> bool {
        matches!(self, SessionState::LoggedIn)
    }
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Connected
    }
}
