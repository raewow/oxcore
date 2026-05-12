//! Reputation state for world player system
//!
//! This module defines the per-player reputation state that is embedded
//! directly in the Player struct. The ReputationSystem operates on this state.

use crate::shared::game::reputation::{
    ReputationListID, ReputationRank, FACTION_FLAG_AT_WAR, FACTION_FLAG_HIDDEN,
    FACTION_FLAG_INACTIVE, FACTION_FLAG_INVISIBLE_FORCED, FACTION_FLAG_PEACE_FORCED,
    FACTION_FLAG_RIVAL, FACTION_FLAG_VISIBLE, MAX_REPUTATION_LIST_SLOTS, POINTS_IN_RANK,
    REPUTATION_BOTTOM, REPUTATION_CAP,
};
use std::collections::HashMap;

/// Per-player reputation state (embedded in Player struct)
#[derive(Debug, Clone, Default)]
pub struct ReputationState {
    /// Faction standings indexed by ReputationListID (0-63)
    /// Not every slot is populated; only factions with reputation_list_id >= 0 in Faction.dbc
    pub factions: HashMap<ReputationListID, FactionStanding>,

    /// Forced reactions override normal reputation rank for specific factions.
    /// Used by special events, quests, or spells that temporarily change
    /// how a faction treats the player.
    pub forced_reactions: HashMap<u32, ReputationRank>,

    /// When true, the system needs to send SMSG_SET_FACTION_STANDING
    /// for all dirty faction entries on the next update tick.
    pub need_send: bool,
}

impl ReputationState {
    /// Create a new empty reputation state
    pub fn new() -> Self {
        Self {
            factions: HashMap::new(),
            forced_reactions: HashMap::new(),
            need_send: false,
        }
    }

    /// Get a faction standing by reputation list ID
    pub fn get_standing(&self, rep_list_id: ReputationListID) -> Option<&FactionStanding> {
        self.factions.get(&rep_list_id)
    }

    /// Get a mutable faction standing by reputation list ID
    pub fn get_standing_mut(
        &mut self,
        rep_list_id: ReputationListID,
    ) -> Option<&mut FactionStanding> {
        self.factions.get_mut(&rep_list_id)
    }

    /// Get a faction standing by faction ID (iterates through all factions)
    pub fn get_standing_by_faction_id(&self, faction_id: u32) -> Option<&FactionStanding> {
        self.factions.values().find(|s| s.faction_id == faction_id)
    }

    /// Get a mutable faction standing by faction ID
    pub fn get_standing_mut_by_faction_id(
        &mut self,
        faction_id: u32,
    ) -> Option<&mut FactionStanding> {
        self.factions
            .values_mut()
            .find(|s| s.faction_id == faction_id)
    }

    /// Check if a faction exists
    pub fn has_faction(&self, rep_list_id: ReputationListID) -> bool {
        self.factions.contains_key(&rep_list_id)
    }

    /// Insert a new faction standing
    pub fn insert_standing(&mut self, standing: FactionStanding) {
        self.factions.insert(standing.reputation_list_id, standing);
    }

    /// Get all factions that need to be sent to the client
    pub fn get_dirty_factions(&mut self) -> Vec<&mut FactionStanding> {
        self.factions.values_mut().filter(|s| s.need_send).collect()
    }

    /// Clear all dirty flags
    pub fn clear_dirty_flags(&mut self) {
        for standing in self.factions.values_mut() {
            standing.need_send = false;
        }
        self.need_send = false;
    }

    /// Get all factions that need to be saved to the database
    pub fn get_factions_to_save(&self) -> Vec<&FactionStanding> {
        self.factions.values().filter(|s| s.need_save).collect()
    }

    /// Mark all factions as saved
    pub fn mark_all_saved(&mut self) {
        for standing in self.factions.values_mut() {
            standing.need_save = false;
        }
    }
}

/// Standing for a single faction
///
/// CRITICAL: `standing` is stored RELATIVE to the base reputation derived
/// from Faction.dbc for the player's race/class.
///
/// Example: A Human Warrior starts with base_rep = 0 for Stormwind.
/// If the player earns 500 reputation, standing = 500.
/// Absolute value = base_rep + standing = 0 + 500 = 500.
///
/// Packets ALWAYS send the absolute value (base + standing), never just standing.
#[derive(Debug, Clone)]
pub struct FactionStanding {
    /// Faction ID from Faction.dbc (e.g., 72 = Stormwind)
    pub faction_id: u32,

    /// Reputation list ID (0-63, maps to client UI slot)
    pub reputation_list_id: ReputationListID,

    /// Faction flags controlling visibility and war state
    /// See FACTION_FLAG_* constants
    pub flags: u32,

    /// Current standing RELATIVE to base reputation
    /// Absolute reputation = base_reputation + standing
    pub standing: i32,

    /// When true, this faction needs an SMSG_SET_FACTION_STANDING update
    pub need_send: bool,

    /// When true, this faction's data has changed and needs a DB write
    pub need_save: bool,
}

impl FactionStanding {
    /// Create a new faction standing
    pub fn new(
        faction_id: u32,
        reputation_list_id: ReputationListID,
        standing: i32,
        flags: u32,
    ) -> Self {
        Self {
            faction_id,
            reputation_list_id,
            flags,
            standing,
            need_send: true,
            need_save: true,
        }
    }

    /// Check if at war with this faction
    pub fn is_at_war(&self) -> bool {
        (self.flags & FACTION_FLAG_AT_WAR) != 0
    }

    /// Check if faction is visible in the reputation panel
    pub fn is_visible(&self) -> bool {
        (self.flags & FACTION_FLAG_VISIBLE) != 0
    }

    /// Check if faction is hidden (never shown in panel)
    pub fn is_hidden(&self) -> bool {
        (self.flags & FACTION_FLAG_HIDDEN) != 0
    }

    /// Check if faction is forced invisible by game mechanics
    pub fn is_invisible_forced(&self) -> bool {
        (self.flags & FACTION_FLAG_INVISIBLE_FORCED) != 0
    }

    /// Check if faction has peace forced (cannot declare at-war)
    pub fn is_peace_forced(&self) -> bool {
        (self.flags & FACTION_FLAG_PEACE_FORCED) != 0
    }

    /// Check if faction is inactive (collapsed in panel)
    pub fn is_inactive(&self) -> bool {
        (self.flags & FACTION_FLAG_INACTIVE) != 0
    }

    /// Check if faction is marked as rival
    pub fn is_rival(&self) -> bool {
        (self.flags & FACTION_FLAG_RIVAL) != 0
    }

    /// Get reputation rank, requires base reputation for absolute calculation
    pub fn get_rank(&self, base_reputation: i32) -> ReputationRank {
        let absolute = base_reputation + self.standing;
        ReputationRank::from_value(absolute)
    }

    /// Get absolute reputation value (base + standing)
    pub fn get_absolute_reputation(&self, base_reputation: i32) -> i32 {
        base_reputation + self.standing
    }

    /// Set the at-war flag
    pub fn set_at_war(&mut self, at_war: bool) {
        if at_war {
            self.flags |= FACTION_FLAG_AT_WAR;
        } else {
            self.flags &= !FACTION_FLAG_AT_WAR;
        }
        self.need_send = true;
        self.need_save = true;
    }

    /// Set the inactive flag
    pub fn set_inactive(&mut self, inactive: bool) {
        if inactive {
            self.flags |= FACTION_FLAG_INACTIVE;
        } else {
            self.flags &= !FACTION_FLAG_INACTIVE;
        }
        self.need_send = true;
        self.need_save = true;
    }

    /// Set the visible flag
    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            // Cannot set visible if forced invisible or hidden
            if !self.is_invisible_forced() && !self.is_hidden() {
                self.flags |= FACTION_FLAG_VISIBLE;
                self.need_send = true;
                self.need_save = true;
            }
        } else {
            self.flags &= !FACTION_FLAG_VISIBLE;
            self.need_send = true;
            self.need_save = true;
        }
    }
}

/// FactionEntry - loaded from Faction.dbc
/// Each entry defines a faction with its base reputation values and flags.
#[derive(Debug, Clone)]
pub struct FactionEntry {
    /// Unique faction ID (e.g., 72 = Stormwind, 76 = Orgrimmar)
    pub id: u32,

    /// Reputation list ID (0-63). If < 0, faction is not tracked in the panel.
    pub reputation_list_id: i32,

    /// Base reputation race masks (4 slots).
    /// Each slot is a bitmask of races that use this base value.
    /// Bit 0 = Human, Bit 1 = Orc, Bit 2 = Dwarf, etc.
    pub base_rep_race_mask: [u32; 4],

    /// Base reputation class masks (4 slots).
    /// Each slot is a bitmask of classes that use this base value.
    /// Bit 0 = Warrior, Bit 1 = Paladin, etc.
    pub base_rep_class_mask: [u32; 4],

    /// Base reputation values (4 slots, one per mask pair).
    /// The first slot whose race AND class mask matches is used.
    pub base_rep_value: [i32; 4],

    /// Initial reputation flags (4 slots, one per mask pair).
    /// Determines initial VISIBLE, AT_WAR, etc. state.
    pub reputation_flags: [u32; 4],

    /// Parent faction ("team") ID. 0 if none.
    /// Used for spillover and faction hierarchy.
    pub team: u32,

    /// Legacy 88-value array for backward compatibility (11 classes * 8 races).
    pub base_rep_value_legacy: [i32; 88],
}

impl FactionEntry {
    /// Create a new FactionEntry with default values
    pub fn new(id: u32) -> Self {
        Self {
            id,
            reputation_list_id: -1,
            base_rep_race_mask: [0; 4],
            base_rep_class_mask: [0; 4],
            base_rep_value: [0; 4],
            reputation_flags: [0; 4],
            team: 0,
            base_rep_value_legacy: [0; 88],
        }
    }

    /// Get index that fits the given race and class masks.
    /// Matches C++ GetIndexFitTo().
    ///
    /// Tests all 4 mask slots. A mask of 0 matches everything (wildcard).
    /// Returns the first matching index, or -1 if no match.
    pub fn get_index_fit_to(&self, race_mask: u32, class_mask: u32) -> i32 {
        for i in 0..4 {
            if (self.base_rep_race_mask[i] == 0 || (self.base_rep_race_mask[i] & race_mask) != 0)
                && (self.base_rep_class_mask[i] == 0
                    || (self.base_rep_class_mask[i] & class_mask) != 0)
            {
                return i as i32;
            }
        }
        -1
    }

    /// Get base reputation for a specific race/class combination.
    ///
    /// Tries the mask-based system first (4 slots).
    /// Falls back to the legacy 88-value array (race-1)*11 + (class-1).
    /// Returns 0 (neutral) if no match.
    pub fn get_base_reputation(&self, race: u8, class: u8) -> i32 {
        let race_mask = if race >= 1 && race <= 8 {
            1u32 << (race - 1)
        } else {
            return 0;
        };

        let class_mask = if class >= 1 && class <= 11 {
            1u32 << (class - 1)
        } else {
            return 0;
        };

        // Try mask-based system first
        let idx = self.get_index_fit_to(race_mask, class_mask);
        if idx >= 0 {
            return self.base_rep_value[idx as usize];
        }

        // Fall back to legacy 88-value array
        let index = ((race - 1) as usize * 11) + (class - 1) as usize;
        if index < self.base_rep_value_legacy.len() {
            self.base_rep_value_legacy[index]
        } else {
            0 // Default neutral
        }
    }

    /// Get initial reputation flags for a specific race/class combination.
    /// Determines which factions start visible, at-war, etc.
    pub fn get_reputation_flags(&self, race: u8, class: u8) -> u32 {
        let race_mask = if race >= 1 && race <= 8 {
            1u32 << (race - 1)
        } else {
            return 0;
        };

        let class_mask = if class >= 1 && class <= 11 {
            1u32 << (class - 1)
        } else {
            return 0;
        };

        let idx = self.get_index_fit_to(race_mask, class_mask);
        if idx >= 0 {
            self.reputation_flags[idx as usize]
        } else {
            0
        }
    }

    /// Check if this faction has a valid reputation list ID
    pub fn is_trackable(&self) -> bool {
        self.reputation_list_id >= 0
    }

    /// Create a FactionEntry from a DBC entry
    pub fn from_dbc(dbc_entry: &crate::world::dbc::structures::FactionDbcEntry) -> Self {
        Self {
            id: dbc_entry.id,
            reputation_list_id: dbc_entry.reputation_list_id,
            base_rep_race_mask: dbc_entry.base_rep_race_mask,
            base_rep_class_mask: dbc_entry.base_rep_class_mask,
            base_rep_value: dbc_entry.base_rep_value,
            reputation_flags: dbc_entry.reputation_flags,
            team: dbc_entry.team,
            base_rep_value_legacy: dbc_entry.base_rep_value_legacy,
        }
    }
}

/// Reputation spillover template entry
///
/// Up to 4 target factions can receive spillover from one source.
/// Each target has an independent rate and rank cap.
#[derive(Debug, Clone)]
pub struct ReputationSpilloverTemplate {
    /// Source faction ID
    pub faction: u32,

    /// Target faction IDs (0 = unused slot)
    pub spillover_factions: [u32; 4],

    /// Rate multiplier per target (0.0 - 1.0 typically)
    pub spillover_rates: [f32; 4],

    /// Maximum rank the spillover can push the target to.
    /// Once the player reaches this rank, no more spillover is applied.
    pub spillover_rank_caps: [u8; 4],
}

impl ReputationSpilloverTemplate {
    /// Create a new spillover template for a faction
    pub fn new(faction: u32) -> Self {
        Self {
            faction,
            spillover_factions: [0; 4],
            spillover_rates: [0.0; 4],
            spillover_rank_caps: [0; 4],
        }
    }
}

/// Convert a u8 rank cap value to ReputationRank enum
pub fn rank_cap_to_reputation_rank(cap: u8) -> ReputationRank {
    match cap {
        0 => ReputationRank::Hated,
        1 => ReputationRank::Hostile,
        2 => ReputationRank::Unfriendly,
        3 => ReputationRank::Neutral,
        4 => ReputationRank::Friendly,
        5 => ReputationRank::Honored,
        6 => ReputationRank::Revered,
        7 => ReputationRank::Exalted,
        _ => ReputationRank::Exalted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_rank_from_value() {
        // Test boundaries
        assert_eq!(ReputationRank::from_value(-42000), ReputationRank::Hated);
        assert_eq!(ReputationRank::from_value(-6000), ReputationRank::Hostile);
        assert_eq!(
            ReputationRank::from_value(-3000),
            ReputationRank::Unfriendly
        );
        assert_eq!(ReputationRank::from_value(0), ReputationRank::Neutral);
        assert_eq!(ReputationRank::from_value(3000), ReputationRank::Friendly);
        assert_eq!(ReputationRank::from_value(9000), ReputationRank::Honored);
        assert_eq!(ReputationRank::from_value(21000), ReputationRank::Revered);
        assert_eq!(ReputationRank::from_value(42000), ReputationRank::Exalted);
        assert_eq!(ReputationRank::from_value(42999), ReputationRank::Exalted);
    }

    #[test]
    fn test_faction_standing_flags() {
        let mut standing = FactionStanding::new(72, 0, 0, 0);

        // Test at-war flag
        assert!(!standing.is_at_war());
        standing.set_at_war(true);
        assert!(standing.is_at_war());
        standing.set_at_war(false);
        assert!(!standing.is_at_war());

        // Test visible flag
        assert!(!standing.is_visible());
        standing.set_visible(true);
        assert!(standing.is_visible());

        // Test inactive flag
        assert!(!standing.is_inactive());
        standing.set_inactive(true);
        assert!(standing.is_inactive());
    }

    #[test]
    fn test_faction_entry_base_reputation() {
        let mut entry = FactionEntry::new(72);
        entry.reputation_list_id = 0;
        entry.base_rep_race_mask[0] = 0x01; // Human
        entry.base_rep_class_mask[0] = 0; // Any class
        entry.base_rep_value[0] = 0; // Neutral start

        // Human should have base rep of 0
        assert_eq!(entry.get_base_reputation(1, 1), 0);

        // Orc (race 2) should not match, fallback to 0
        assert_eq!(entry.get_base_reputation(2, 1), 0);
    }

    #[test]
    fn test_get_rank() {
        let standing = FactionStanding::new(72, 0, 500, 0);
        // With base rep 0, standing 500 = absolute 500 = Neutral
        assert_eq!(standing.get_rank(0), ReputationRank::Neutral);

        // With base rep 0, standing 3000 = absolute 3000 = Friendly
        let standing2 = FactionStanding::new(72, 0, 3000, 0);
        assert_eq!(standing2.get_rank(0), ReputationRank::Friendly);
    }
}
