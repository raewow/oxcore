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
use oxcore_dbc::structures::SpellEntry;

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
    TriggerStun,
    Sleep,
    /// Rogue-only: Kidney Shot has its own DR in vanilla
    KidneyShot,
    /// Fear effects (Fear, Psychic Scream, Intimidating Shout, etc.)
    Fear,
    /// Root effects (Frost Nova, Entangling Roots, etc.)
    Root,
    TriggerRoot,
    /// Silence effects (Silence, Counterspell silence, etc.)
    Silence,
    /// Disorient (Polymorph, Sap, Gouge, etc.)
    Disorient,
    Polymorph,
    /// Mind Control
    MindControl,
    /// Freeze (Frost Nova pet, etc.)
    Freeze,
    /// Banish
    Banish,
    DeathCoil,
    WarlockFear,
    Disarm,
    Knockout,
    LimitOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DRType {
    None,
    Player,
    All,
}

pub fn dr_type(group: DRGroup) -> DRType {
    match group {
        DRGroup::Stun | DRGroup::TriggerStun | DRGroup::KidneyShot => DRType::All,
        DRGroup::Sleep | DRGroup::Root | DRGroup::TriggerRoot | DRGroup::Fear
        | DRGroup::MindControl | DRGroup::Polymorph | DRGroup::Silence | DRGroup::Disarm
        | DRGroup::DeathCoil | DRGroup::Freeze | DRGroup::Banish | DRGroup::WarlockFear
        | DRGroup::Knockout => DRType::Player,
        _ => DRType::None,
    }
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
        if matches!(dr_type(group), DRType::None) {
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

/// Classify a spell using the reference `SpellEntry::GetDiminishingReturnsGroup` order.
pub fn get_dr_group_for_spell(spell: &SpellEntry, triggered_by_aura: bool) -> DRGroup {
    const ROGUE: u32 = 8;
    const HUNTER: u32 = 9;
    const WARLOCK: u32 = 5;
    const WARRIOR: u32 = 4;
    const SHAMAN: u32 = 11;
    let flags = spell.spell_family_flags;
    match spell.spell_family_name {
        ROGUE if flags & 0x0020_0000 != 0 => return DRGroup::KidneyShot,
        ROGUE if flags & 0x0100_0000 != 0 => return DRGroup::None,
        HUNTER if flags & 0x0000_0008 != 0 => return DRGroup::Freeze,
        WARLOCK if flags & 0x8000_0000 != 0 && spell.mechanic == 5 => return DRGroup::WarlockFear,
        WARLOCK if spell.id == 6358 => return DRGroup::WarlockFear,
        WARLOCK if flags & 0x8000_0000 != 0 => return DRGroup::LimitOnly,
        WARRIOR if flags & 0x0000_0002 != 0 => return DRGroup::LimitOnly,
        SHAMAN if flags & 0x8000_0000 != 0 => return DRGroup::Root,
        3 if spell.spell_visual == 4325 => return DRGroup::None,
        0 if matches!(spell.id, 12355 | 18093) => return DRGroup::TriggerStun,
        _ => {}
    }
    if matches!(spell.id, 7922 | 20253 | 20614 | 20615) { return DRGroup::Stun; }
    let mechanics = spell.effect_mechanic.iter().fold(1u32 << spell.mechanic.saturating_sub(1), |mask, &m| mask | (1u32 << m.saturating_sub(1)));
    let has = |mechanic: u32| mechanics & (1 << mechanic.saturating_sub(1)) != 0;
    if has(12) { return if triggered_by_aura { DRGroup::TriggerStun } else { DRGroup::Stun }; }
    if has(10) { return DRGroup::Sleep; }
    if has(17) { return DRGroup::Polymorph; }
    if has(7) { return if triggered_by_aura { DRGroup::TriggerRoot } else { DRGroup::Root }; }
    if has(5) { return DRGroup::Fear; }
    if has(1) { return DRGroup::MindControl; }
    if has(9) { return DRGroup::Silence; }
    if has(2) { return DRGroup::Disarm; }
    if has(3) { return DRGroup::Freeze; }
    if has(14) || has(15) { return DRGroup::Knockout; }
    if has(18) { return DRGroup::Banish; }
    if has(24) { return DRGroup::DeathCoil; }
    DRGroup::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_applications_reduce_duration_then_make_target_immune() {
        let mut state = DiminishingState::default();

        assert_eq!(state.apply_dr(DRGroup::Stun, 1_000), 1.0);
        assert_eq!(state.apply_dr(DRGroup::Stun, 2_000), 0.5);
        assert_eq!(state.apply_dr(DRGroup::Stun, 3_000), 0.25);
        assert_eq!(state.apply_dr(DRGroup::Stun, 4_000), 0.0);
        assert!(state.is_immune(DRGroup::Stun, 4_000));
    }

    #[test]
    fn expired_diminishing_state_restores_full_duration() {
        let mut state = DiminishingState::default();

        state.apply_dr(DRGroup::Fear, 1_000);
        state.apply_dr(DRGroup::Fear, 2_000);

        assert_eq!(state.apply_dr(DRGroup::Fear, 17_000), 1.0);
        assert!(!state.is_immune(DRGroup::Fear, 17_000));
    }

    #[test]
    fn no_diminishing_group_never_records_state() {
        let mut state = DiminishingState::default();

        assert_eq!(state.apply_dr(DRGroup::None, 1_000), 1.0);
        assert!(state.groups.is_empty());
    }
}
