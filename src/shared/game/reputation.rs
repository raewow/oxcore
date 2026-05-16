//! Vanilla WoW Reputation System Types
//!
//! Reputation is stored as RELATIVE to base reputation from Faction.dbc.
//! Absolute reputation = base_reputation + standing
//! Packets always send absolute values.

/// Faction ID from Faction.dbc (e.g., 72 = Stormwind)
pub type FactionId = u32;

/// Reputation list ID (0-63), used as index in packets and client UI
pub type ReputationListID = u32;

/// Maximum number of reputation list slots sent to the client
pub const MAX_REPUTATION_LIST_SLOTS: usize = 64;

/// Maximum absolute reputation value (top of Exalted)
pub const REPUTATION_CAP: i32 = 42999;

/// Minimum absolute reputation value (bottom of Hated)
pub const REPUTATION_BOTTOM: i32 = -42000;

/// Number of reputation points in each rank tier (from Hated upward)
/// Index 0 = Hated→Hostile, 1 = Hostile→Unfriendly, etc.
pub const POINTS_IN_RANK: [i32; 8] = [
    36000, // Hated to Hostile
    3000,  // Hostile to Unfriendly
    3000,  // Unfriendly to Neutral
    3000,  // Neutral to Friendly
    6000,  // Friendly to Honored
    12000, // Honored to Revered
    21000, // Revered to Exalted
    1000,  // Exalted cap buffer
];

/// Reputation rank - determines relationship between player and faction
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ReputationRank {
    Hated = 0,
    Hostile = 1,
    Unfriendly = 2,
    Neutral = 3,
    Friendly = 4,
    Honored = 5,
    Revered = 6,
    Exalted = 7,
}

impl ReputationRank {
    /// Convert to u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Convert from i32 (for database/deserialization)
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(ReputationRank::Hated),
            1 => Some(ReputationRank::Hostile),
            2 => Some(ReputationRank::Unfriendly),
            3 => Some(ReputationRank::Neutral),
            4 => Some(ReputationRank::Friendly),
            5 => Some(ReputationRank::Honored),
            6 => Some(ReputationRank::Revered),
            7 => Some(ReputationRank::Exalted),
            _ => None,
        }
    }

    /// Convert absolute reputation value to rank
    /// Matches C++ ReputationToRank() function from MaNGOS
    pub fn from_value(value: i32) -> Self {
        let mut limit = REPUTATION_CAP + 1; // 43000

        // Walk from Exalted (index 7) down to Hated (index 0)
        for i in (0..8).rev() {
            limit -= POINTS_IN_RANK[i];
            if value >= limit {
                return match i {
                    0 => ReputationRank::Hated,
                    1 => ReputationRank::Hostile,
                    2 => ReputationRank::Unfriendly,
                    3 => ReputationRank::Neutral,
                    4 => ReputationRank::Friendly,
                    5 => ReputationRank::Honored,
                    6 => ReputationRank::Revered,
                    7 => ReputationRank::Exalted,
                    _ => ReputationRank::Hated,
                };
            }
        }

        ReputationRank::Hated
    }

    /// Get the minimum absolute reputation value for this rank
    pub fn to_value(self) -> i32 {
        match self {
            ReputationRank::Hated => REPUTATION_BOTTOM, // -42000
            ReputationRank::Hostile => -6000,
            ReputationRank::Unfriendly => -3000,
            ReputationRank::Neutral => 0,
            ReputationRank::Friendly => 3000,
            ReputationRank::Honored => 9000,
            ReputationRank::Revered => 21000,
            ReputationRank::Exalted => 42000,
        }
    }

    /// Get points required to reach this rank from Hated
    pub fn get_points_to_rank(self) -> i32 {
        let rank_idx = self as usize;
        if rank_idx == 0 {
            return REPUTATION_BOTTOM;
        }
        let mut points = REPUTATION_BOTTOM;
        for i in 0..rank_idx {
            points += POINTS_IN_RANK[i];
        }
        points
    }
}

/// Faction is visible in the reputation panel
pub const FACTION_FLAG_VISIBLE: u32 = 0x01;

/// Player has declared at-war with this faction
pub const FACTION_FLAG_AT_WAR: u32 = 0x02;

/// Faction is hidden from the reputation panel
pub const FACTION_FLAG_HIDDEN: u32 = 0x04;

/// Faction is forced invisible by game mechanics
pub const FACTION_FLAG_INVISIBLE_FORCED: u32 = 0x08;

/// Peace is forced with this faction (cannot declare war)
pub const FACTION_FLAG_PEACE_FORCED: u32 = 0x10;

/// Faction is inactive (collapsed in panel)
pub const FACTION_FLAG_INACTIVE: u32 = 0x20;

/// Faction is marked as rival
pub const FACTION_FLAG_RIVAL: u32 = 0x40;

/// Legacy FactionFlags struct (for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionFlags(pub u8);

impl FactionFlags {
    pub const NONE: u8 = 0x00;
    pub const VISIBLE: u8 = 0x01;
    pub const ATWAR: u8 = 0x02;
    pub const HIDDEN: u8 = 0x04;
    pub const INVISIBLE: u8 = 0x08;
    pub const OWN_TEAM: u8 = 0x10;
    pub const INACTIVE: u8 = 0x20;
    pub const HAS_REP: u8 = 0x40;
    pub const CAN_REP_WILL_FRIENDLY: u8 = 0x80;

    pub fn has_flag(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }
}

/// Legacy FactionState struct (for backward compatibility)
#[derive(Debug, Clone)]
pub struct FactionState {
    pub id: FactionId,
    pub reputation: i32,
    pub rank: ReputationRank,
    pub flags: FactionFlags,
    pub in_war_with: Vec<FactionId>,
    pub war_goal: FactionId,
    pub pending_equivalency: bool,
}

impl FactionState {
    pub fn new(id: FactionId) -> Self {
        Self {
            id,
            reputation: 0,
            rank: ReputationRank::Neutral,
            flags: FactionFlags(FactionFlags::NONE),
            in_war_with: Vec::new(),
            war_goal: 0,
            pending_equivalency: false,
        }
    }
}

/// Get vendor discount percentage for a reputation rank
/// Vanilla 1.12: Honored 5%, Revered 10%, Exalted 15%
pub fn vendor_discount_pct(rank: ReputationRank) -> f32 {
    match rank {
        ReputationRank::Honored => 0.05,
        ReputationRank::Revered => 0.10,
        ReputationRank::Exalted => 0.15,
        _ => 0.0,
    }
}

/// Calculate final vendor price after reputation discount
pub fn apply_vendor_discount(base_price: u32, rank: ReputationRank) -> u32 {
    let discount = vendor_discount_pct(rank);
    if discount <= 0.0 {
        return base_price;
    }
    let discounted = base_price as f32 * (1.0 - discount);
    (discounted as u32).max(1)
}

/// Calculate reputation with level-based reduction
/// Formula: reputation = base_reputation * (1.0 - (level_diff * 0.05))
/// Maximum reduction: 90% (at 18+ level difference)
pub fn apply_level_reduction(base_rep: i32, player_level: u8, creature_level: u8) -> i32 {
    if player_level == 0 || creature_level == 0 {
        return base_rep;
    }

    let level_diff = player_level as i32 - creature_level as i32;

    // No reduction if player is same level or lower
    if level_diff <= 0 {
        return base_rep;
    }

    // 5% reduction per level difference, capped at 90%
    let reduction_factor = (level_diff as f32 * 0.05).min(0.9);
    let multiplier = 1.0 - reduction_factor;

    (base_rep as f32 * multiplier) as i32
}
