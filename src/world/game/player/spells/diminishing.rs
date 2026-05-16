//! Diminishing Returns (DR)
//!
//! In vanilla WoW, crowd control effects have diminishing returns when applied
//! repeatedly to the same target. Each successive application within a 15-second
//! window has reduced duration:
//!
//! - Level 0 (first): 100% duration
//! - Level 1 (second): 50% duration
//! - Level 2 (third): 25% duration
//! - Level 3+ (fourth+): immune (0% duration)
//!
//! DR groups are shared among related CC effects (e.g., all stuns share a group).
//! The DR level resets 15 seconds after the last application.

use std::collections::HashMap;

/// Duration (in milliseconds) before DR resets for a target.
pub const DR_RESET_TIME_MS: u64 = 15_000;

/// Maximum DR level before target becomes immune.
pub const DR_MAX_LEVEL: u8 = 3;

/// Diminishing return groups. Spells in the same group share DR on a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DRGroup {
    None = 0,
    /// All stuns (Cheap Shot, Hammer of Justice, Bash, Kidney Shot, etc.)
    Stun,
    /// Rogue-only: Kidney Shot has its own DR in vanilla
    KidneyShot,
    /// Fear effects (Fear, Psychic Scream, Intimidating Shout, etc.)
    Fear,
    /// Root effects (Frost Nova, Entangling Roots, etc.)
    Root,
    /// Silence effects (Silence, Counterspell silence, etc.)
    Silence,
    /// Disorient (Polymorph, Sap, Gouge, etc.)
    Disorient,
    /// Mind Control
    MindControl,
    /// Freeze (Frost Nova pet, etc.)
    Freeze,
    /// Banish
    Banish,
}

/// Per-target DR tracking for one group.
#[derive(Debug, Clone)]
pub struct DRState {
    /// Current DR level (0=first, 1=50%, 2=25%, 3+=immune)
    pub level: u8,
    /// Game time (ms) when the DR was last incremented — resets after 15s
    pub last_applied_ms: u64,
}

/// Per-player (target) diminishing returns state.
/// Tracks DR for all groups that have been applied to this player.
#[derive(Debug, Clone, Default)]
pub struct DiminishingState {
    /// DR state per group
    pub groups: HashMap<DRGroup, DRState>,
}

impl DiminishingState {
    /// Get the DR duration modifier for a group (1.0 = full, 0.5 = half, 0.25 = quarter, 0.0 = immune).
    /// Also increments the DR level and resets the timer.
    pub fn apply_dr(&mut self, group: DRGroup, now_ms: u64) -> f32 {
        if group == DRGroup::None {
            return 1.0;
        }

        // Clean up expired DR
        if let Some(state) = self.groups.get(&group) {
            if now_ms >= state.last_applied_ms + DR_RESET_TIME_MS {
                self.groups.remove(&group);
            }
        }

        let modifier = match self.groups.get(&group).map(|s| s.level).unwrap_or(0) {
            0 => 1.0,  // First application: full duration
            1 => 0.5,  // Second: half
            2 => 0.25, // Third: quarter
            _ => 0.0,  // Fourth+: immune
        };

        // Increment DR level
        let state = self.groups.entry(group).or_insert(DRState {
            level: 0,
            last_applied_ms: now_ms,
        });
        state.level = (state.level + 1).min(DR_MAX_LEVEL + 1);
        state.last_applied_ms = now_ms;

        modifier
    }

    /// Check if target is immune to a DR group (without incrementing).
    pub fn is_immune(&self, group: DRGroup, now_ms: u64) -> bool {
        if group == DRGroup::None {
            return false;
        }

        if let Some(state) = self.groups.get(&group) {
            if now_ms < state.last_applied_ms + DR_RESET_TIME_MS {
                return state.level >= DR_MAX_LEVEL;
            }
        }
        false
    }

    /// Clear expired DR states (housekeeping, called periodically).
    pub fn clear_expired(&mut self, now_ms: u64) {
        self.groups
            .retain(|_, state| now_ms < state.last_applied_ms + DR_RESET_TIME_MS);
    }
}

/// Determine the DR group for a spell based on its mechanic and aura type.
///
/// This is a simplified mapping based on MaNGOS GetDiminishingReturnsGroupForSpell().
/// In vanilla, DR grouping is based on spell mechanic and specific spell IDs.
pub fn get_dr_group_for_spell(mechanic: u32, aura_type: u32) -> DRGroup {
    // Check by mechanic first (covers most cases)
    match mechanic {
        // MECHANIC_STUN = 12
        12 => DRGroup::Stun,
        // MECHANIC_FEAR = 5
        5 => DRGroup::Fear,
        // MECHANIC_ROOT = 7
        7 => DRGroup::Root,
        // MECHANIC_SILENCE = 9
        9 => DRGroup::Silence,
        // MECHANIC_POLYMORPH = 17
        17 => DRGroup::Disorient,
        // MECHANIC_SAP = 14
        14 => DRGroup::Disorient,
        // MECHANIC_SLEEP = 10
        10 => DRGroup::Disorient,
        // MECHANIC_CHARM = 1
        1 => DRGroup::MindControl,
        // MECHANIC_FREEZE = 3
        3 => DRGroup::Freeze,
        // MECHANIC_BANISH = 18
        18 => DRGroup::Banish,
        _ => {
            // Fallback: check aura type for CC that might not have a mechanic set
            use crate::world::game::player::auras::effects::*;
            match aura_type {
                AURA_MOD_STUN => DRGroup::Stun,
                AURA_MOD_FEAR => DRGroup::Fear,
                AURA_MOD_ROOT => DRGroup::Root,
                _ => DRGroup::None,
            }
        }
    }
}
