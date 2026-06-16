use std::collections::HashMap;

/// Maximum talent rank any single talent can reach
pub const MAX_TALENT_RANK: u8 = 5;

/// Number of talent tabs (trees) per class
pub const MAX_TALENT_TABS: usize = 3;

/// Points required per row tier in a talent tree
/// Row 0 = 0 points, Row 1 = 5, Row 2 = 10, ..., Row 6 = 30
pub const POINTS_PER_ROW: u32 = 5;

/// Number of rows in a talent tree (0-6 inclusive)
pub const MAX_TALENT_ROWS: usize = 7;

/// Number of columns in a talent tree (0-3 inclusive)
pub const MAX_TALENT_COLUMNS: usize = 4;

/// Per-player talent state, embedded in the Player struct.
///
/// Tracks all talent allocations, free points, and reset cost data.
/// Corresponds to C++ fields: m_talents, m_freeTalentPoints,
/// m_usedTalentCount, m_resetTalentsMultiplier, m_resetTalentsTime.
#[derive(Debug, Clone)]
pub struct TalentState {
    /// Map of talent_id -> current rank (1-5).
    /// Only contains talents the player has invested in.
    /// Talent IDs come from Talent.dbc, NOT spell IDs.
    pub talents: HashMap<u32, u8>,

    /// Unspent talent points available for allocation.
    /// Displayed in client via PLAYER_CHARACTER_POINTS1 update field.
    pub free_talent_points: u32,

    /// Total talent points spent across all trees.
    /// Sum of all rank values in the talents map.
    /// Invariant: free_talent_points + used_talent_count == calculate_total_talent_points(level)
    pub used_talent_count: u32,

    /// Reset cost multiplier, incremented on each paid reset.
    /// Decays by 1 per month since last reset.
    /// Used to compute escalating respec gold cost.
    pub reset_cost_multiplier: u32,

    /// Unix timestamp of last talent reset.
    /// Used for decay calculation of reset_cost_multiplier.
    pub last_reset_time: u64,
}

impl Default for TalentState {
    fn default() -> Self {
        Self {
            talents: HashMap::new(),
            free_talent_points: 0,
            used_talent_count: 0,
            reset_cost_multiplier: 0,
            last_reset_time: 0,
        }
    }
}

impl TalentState {
    /// Count total points spent in a specific talent tab (tree).
    ///
    /// # Arguments
    /// * `tab_id` - The talent tab index (0, 1, or 2)
    /// * `talent_tab_map` - Mapping of talent_id -> tab_id from DBC
    pub fn points_in_tab(&self, tab_id: u32, talent_tab_map: &HashMap<u32, u32>) -> u32 {
        self.talents
            .iter()
            .filter(|(talent_id, _)| talent_tab_map.get(talent_id).copied() == Some(tab_id))
            .map(|(_, &rank)| rank as u32)
            .sum()
    }

    /// Check if a specific talent is at max rank.
    pub fn is_talent_maxed(&self, talent_id: u32, max_rank: u8) -> bool {
        self.talents.get(&talent_id).copied().unwrap_or(0) >= max_rank
    }

    /// Get current rank of a talent (0 if not learned).
    pub fn talent_rank(&self, talent_id: u32) -> u8 {
        self.talents.get(&talent_id).copied().unwrap_or(0)
    }
}
