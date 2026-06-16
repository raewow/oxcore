//! Experience system types
//!
//! In-memory types for the experience system.

/// Per-player experience state (cached for quick access)
#[derive(Debug, Clone, Default)]
pub struct ExperienceState {
    /// Current XP (0 to next_level_xp - 1)
    pub xp: u32,
    /// XP required for next level
    pub next_level_xp: u32,
    /// Total XP earned (lifetime)
    pub total_xp: u32,
    /// Rested XP bonus remaining
    pub rest_bonus: f32,
    /// Needs XP update sent to client
    pub dirty: bool,
}

impl ExperienceState {
    /// Create experience state for a given level
    pub fn new(level: u8, xp: u32, next_level_xp: u32) -> Self {
        Self {
            xp,
            next_level_xp,
            total_xp: 0,
            rest_bonus: 0.0,
            dirty: false,
        }
    }
}
