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

use oxcore_dbc::structures::SpellEntry;
use std::collections::HashMap;

/// Duration (in milliseconds) before DR resets for a target.
pub const DR_RESET_TIME_MS: u64 = 15_000;

/// Maximum DR level before target becomes immune.
pub const DR_MAX_LEVEL: u8 = 3;

/// Diminishing return groups. Spells in the same group share DR on a target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DRGroup {
    #[default]
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

/// Duration multiplier for a diminishing level (`Spells::GetDiminishingRate`).
/// Level 0 is the first application and is never reduced; level 3 is full immunity.
pub fn diminishing_rate(level: u8) -> f32 {
    match level {
        0 => 1.0,
        1 => 0.5,
        2 => 0.25,
        _ => 0.0,
    }
}

/// The diminishing-returns decision taken once per unit hit, then reused by every aura
/// that hit applies.
///
/// The snapshot is sampled before effect processing rather than when each aura is added,
/// because one spell may apply several auras that must all share a single level and a
/// single counter increment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiminishSnapshot {
    /// The group this spell diminishes on, or `None` when it has no DR.
    pub group: DRGroup,
    /// The target's level in that group *before* this hit incremented it.
    pub level: u8,
    /// Whether `Unit::ApplyDiminishingToDuration` would actually shorten this hit's
    /// durations. False for a group with no DR, for a friendly (non-reflected) caster,
    /// and for a `DRTYPE_PLAYER` group outside a player-versus-player pair.
    pub diminishes_duration: bool,
}

impl DiminishSnapshot {
    /// Apply the snapshot to an aura duration (`Unit::ApplyDiminishingToDuration`).
    /// Permanent auras (`None`) are never diminished.
    pub fn apply_to_duration(&self, duration_ms: Option<u32>) -> Option<u32> {
        let duration = duration_ms?;
        if !self.diminishes_duration {
            return Some(duration);
        }
        Some((duration as f32 * diminishing_rate(self.level)) as u32)
    }

    /// Whether this hit is fully diminished, i.e. its auras would land with zero duration.
    pub fn is_fully_diminished(&self) -> bool {
        self.diminishes_duration && diminishing_rate(self.level) <= 0.0
    }
}

pub fn dr_type(group: DRGroup) -> DRType {
    match group {
        DRGroup::Stun | DRGroup::TriggerStun | DRGroup::KidneyShot => DRType::All,
        DRGroup::Sleep
        | DRGroup::Root
        | DRGroup::TriggerRoot
        | DRGroup::Fear
        | DRGroup::MindControl
        | DRGroup::Polymorph
        | DRGroup::Silence
        | DRGroup::Disarm
        | DRGroup::DeathCoil
        | DRGroup::Freeze
        | DRGroup::Banish
        | DRGroup::WarlockFear
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
    pub active_auras: u16,
}

/// Per-player (target) diminishing returns state.
/// Tracks DR for all groups that have been applied to this player.
#[derive(Debug, Clone, Default)]
pub struct DiminishingState {
    /// DR state per group
    pub groups: HashMap<DRGroup, DRState>,
}

impl DiminishingState {
    /// Current diminishing level for a group.
    ///
    /// Level 0 means "first application, full duration". Performs the lazy reset:
    /// once no aura of the group is active and 15 seconds have passed since the
    /// last one dropped, the counter falls back to level 0.
    pub fn get_diminishing(&mut self, group: DRGroup, now_ms: u64) -> u8 {
        let Some(state) = self.groups.get_mut(&group) else {
            return 0;
        };
        if state.level == 0 {
            return 0;
        }
        if state.active_auras == 0 && now_ms >= state.last_applied_ms + DR_RESET_TIME_MS {
            state.level = 0;
            return 0;
        }
        state.level
    }

    /// Raise the diminishing level for a group by one, capped at immunity
    /// (`Unit::IncrDiminishing`). A group seen for the first time goes straight to level 1,
    /// because the hit doing the incrementing has already consumed level 0.
    pub fn incr_diminishing(&mut self, group: DRGroup, now_ms: u64) {
        let state = self.groups.entry(group).or_insert(DRState {
            level: 0,
            last_applied_ms: now_ms,
            active_auras: 0,
        });
        state.level = (state.level + 1).min(DR_MAX_LEVEL);
        state.last_applied_ms = now_ms;
    }

    /// Track an aura of this group being applied (`Unit::ApplyDiminishingAura(group, true)`).
    /// The reset timer only starts once the last aura of the group has dropped.
    pub fn add_aura(&mut self, group: DRGroup, now_ms: u64) {
        let state = self.groups.entry(group).or_insert(DRState {
            level: 0,
            last_applied_ms: now_ms,
            active_auras: 0,
        });
        state.active_auras += 1;
    }

    pub fn remove_aura(&mut self, group: DRGroup, now_ms: u64) {
        if let Some(state) = self.groups.get_mut(&group) {
            state.active_auras = state.active_auras.saturating_sub(1);
            if state.active_auras == 0 {
                state.last_applied_ms = now_ms;
            }
        }
    }

    /// Check if target is immune to a DR group (without incrementing).
    pub fn is_immune(&self, group: DRGroup, now_ms: u64) -> bool {
        if group == DRGroup::None {
            return false;
        }

        if let Some(state) = self.groups.get(&group) {
            if state.active_auras > 0 || now_ms < state.last_applied_ms + DR_RESET_TIME_MS {
                return state.level >= DR_MAX_LEVEL;
            }
        }
        false
    }

    /// Snapshot the diminishing decision for one spell hit, then charge the target for it.
    ///
    /// `applies_aura` mirrors `IsSpellAppliesAura(effectMask)` — a hit that applies no aura
    /// reads the level but never increments it. `caster_is_friendly` folds in the
    /// `ApplyDiminishingToDuration` exemption for friendly casters, which a reflected cast
    /// bypasses.
    pub fn snapshot_for_hit(
        &mut self,
        group: DRGroup,
        target_is_player_like: bool,
        caster_is_player_like: bool,
        applies_aura: bool,
        caster_is_friendly: bool,
        is_reflected: bool,
        now_ms: u64,
    ) -> DiminishSnapshot {
        let group_type = dr_type(group);
        if group_type == DRType::None {
            return DiminishSnapshot::default();
        }

        let level = self.get_diminishing(group, now_ms);

        // The counter advances for the *next* cast, gated on this hit actually applying an
        // aura and on the group covering this target.
        if applies_aura && (group_type == DRType::All || target_is_player_like) {
            self.incr_diminishing(group, now_ms);
        }

        // DRTYPE_PLAYER only diminishes durations between two player-like units; DRTYPE_ALL
        // (the stun family) diminishes everywhere.
        let pvp = target_is_player_like && caster_is_player_like;
        let diminishes_duration = (!caster_is_friendly || is_reflected)
            && (group_type == DRType::All || (group_type == DRType::Player && pvp));

        DiminishSnapshot {
            group,
            level,
            diminishes_duration,
        }
    }

    /// Clear expired DR states (housekeeping, called periodically).
    pub fn clear_expired(&mut self, now_ms: u64) {
        self.groups
            .retain(|_, state| now_ms < state.last_applied_ms + DR_RESET_TIME_MS);
    }
}

/// Classify a spell into a diminishing returns group.
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
    if matches!(spell.id, 7922 | 20253 | 20614 | 20615) {
        return DRGroup::Stun;
    }
    let mechanics = spell
        .effect_mechanic
        .iter()
        .fold(1u32 << spell.mechanic.saturating_sub(1), |mask, &m| {
            mask | (1u32 << m.saturating_sub(1))
        });
    let has = |mechanic: u32| mechanics & (1 << mechanic.saturating_sub(1)) != 0;
    if has(12) {
        return if triggered_by_aura {
            DRGroup::TriggerStun
        } else {
            DRGroup::Stun
        };
    }
    if has(10) {
        return DRGroup::Sleep;
    }
    if has(17) {
        return DRGroup::Polymorph;
    }
    if has(7) {
        return if triggered_by_aura {
            DRGroup::TriggerRoot
        } else {
            DRGroup::Root
        };
    }
    if has(5) {
        return DRGroup::Fear;
    }
    if has(1) {
        return DRGroup::MindControl;
    }
    if has(9) {
        return DRGroup::Silence;
    }
    if has(2) {
        return DRGroup::Disarm;
    }
    if has(3) {
        return DRGroup::Freeze;
    }
    if has(14) || has(15) {
        return DRGroup::Knockout;
    }
    if has(18) {
        return DRGroup::Banish;
    }
    if has(24) {
        return DRGroup::DeathCoil;
    }
    DRGroup::None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One player-versus-player hit: sample, charge the counter, reduce the duration.
    fn pvp_hit(state: &mut DiminishingState, group: DRGroup, now_ms: u64) -> DiminishSnapshot {
        state.snapshot_for_hit(group, true, true, true, false, false, now_ms)
    }

    #[test]
    fn repeated_applications_reduce_duration_then_make_target_immune() {
        let mut state = DiminishingState::default();

        assert_eq!(pvp_hit(&mut state, DRGroup::Stun, 1_000).level, 0);
        assert_eq!(pvp_hit(&mut state, DRGroup::Stun, 2_000).level, 1);
        assert_eq!(pvp_hit(&mut state, DRGroup::Stun, 3_000).level, 2);

        let immune = pvp_hit(&mut state, DRGroup::Stun, 4_000);
        assert_eq!(immune.level, 3);
        assert!(immune.is_fully_diminished());
        assert!(state.is_immune(DRGroup::Stun, 4_000));
    }

    #[test]
    fn snapshot_scales_duration_by_the_sampled_level() {
        let mut state = DiminishingState::default();

        assert_eq!(
            pvp_hit(&mut state, DRGroup::Fear, 1_000).apply_to_duration(Some(8_000)),
            Some(8_000)
        );
        assert_eq!(
            pvp_hit(&mut state, DRGroup::Fear, 2_000).apply_to_duration(Some(8_000)),
            Some(4_000)
        );
        assert_eq!(
            pvp_hit(&mut state, DRGroup::Fear, 3_000).apply_to_duration(Some(8_000)),
            Some(2_000)
        );
    }

    /// A permanent aura has no duration to diminish.
    #[test]
    fn permanent_durations_are_never_diminished() {
        let mut state = DiminishingState::default();
        pvp_hit(&mut state, DRGroup::Stun, 1_000);
        let snapshot = pvp_hit(&mut state, DRGroup::Stun, 2_000);

        assert_eq!(snapshot.apply_to_duration(None), None);
    }

    #[test]
    fn expired_diminishing_state_restores_full_duration() {
        let mut state = DiminishingState::default();

        pvp_hit(&mut state, DRGroup::Fear, 1_000);
        pvp_hit(&mut state, DRGroup::Fear, 2_000);

        // The reset only lands once no aura of the group is still active.
        state.remove_aura(DRGroup::Fear, 2_000);
        assert_eq!(pvp_hit(&mut state, DRGroup::Fear, 18_000).level, 0);
        assert!(!state.is_immune(DRGroup::Fear, 18_000));
    }

    /// While an aura of the group is still up, the 15-second window has not started.
    #[test]
    fn active_aura_holds_the_reset_window_open() {
        let mut state = DiminishingState::default();

        pvp_hit(&mut state, DRGroup::Stun, 1_000);
        pvp_hit(&mut state, DRGroup::Stun, 2_000);
        state.add_aura(DRGroup::Stun, 2_000);

        assert_eq!(state.get_diminishing(DRGroup::Stun, 100_000), 2);
    }

    #[test]
    fn no_diminishing_group_never_records_state() {
        let mut state = DiminishingState::default();

        let snapshot = pvp_hit(&mut state, DRGroup::None, 1_000);
        assert_eq!(snapshot, DiminishSnapshot::default());
        assert_eq!(snapshot.apply_to_duration(Some(8_000)), Some(8_000));
        assert!(state.groups.is_empty());
    }

    /// DRTYPE_PLAYER groups (fear, root, polymorph…) only shorten durations between two
    /// players; DRTYPE_ALL groups (the stun family) also apply against creatures.
    #[test]
    fn player_only_groups_do_not_diminish_creature_targets() {
        let mut state = DiminishingState::default();
        state.incr_diminishing(DRGroup::Fear, 1_000);

        let vs_creature =
            state.snapshot_for_hit(DRGroup::Fear, false, true, true, false, false, 2_000);
        assert_eq!(vs_creature.level, 1);
        assert!(!vs_creature.diminishes_duration);
        assert_eq!(vs_creature.apply_to_duration(Some(8_000)), Some(8_000));

        let mut stun_state = DiminishingState::default();
        stun_state.incr_diminishing(DRGroup::Stun, 1_000);
        let stun_vs_creature =
            stun_state.snapshot_for_hit(DRGroup::Stun, false, true, true, false, false, 2_000);
        assert!(stun_vs_creature.diminishes_duration);
        assert_eq!(stun_vs_creature.apply_to_duration(Some(8_000)), Some(4_000));
    }

    /// A DRTYPE_PLAYER group never charges a creature's counter, but DRTYPE_ALL does.
    #[test]
    fn creature_counters_only_advance_for_all_type_groups() {
        let mut state = DiminishingState::default();
        state.snapshot_for_hit(DRGroup::Fear, false, true, true, false, false, 1_000);
        assert_eq!(state.get_diminishing(DRGroup::Fear, 1_000), 0);

        state.snapshot_for_hit(DRGroup::Stun, false, true, true, false, false, 1_000);
        assert_eq!(state.get_diminishing(DRGroup::Stun, 1_000), 1);
    }

    /// A hit that applies no aura reads the level but leaves the counter alone.
    #[test]
    fn hits_without_auras_do_not_charge_the_counter() {
        let mut state = DiminishingState::default();

        state.snapshot_for_hit(DRGroup::Stun, true, true, false, false, false, 1_000);
        assert_eq!(state.get_diminishing(DRGroup::Stun, 1_000), 0);
    }

    /// A friendly caster does not diminish its target — unless the spell was reflected
    /// back at it, in which case the exemption is skipped.
    #[test]
    fn friendly_casters_do_not_diminish_unless_reflected() {
        let mut state = DiminishingState::default();
        state.incr_diminishing(DRGroup::Stun, 1_000);

        let friendly = state.snapshot_for_hit(DRGroup::Stun, true, true, true, true, false, 2_000);
        assert!(!friendly.diminishes_duration);
        assert_eq!(friendly.apply_to_duration(Some(8_000)), Some(8_000));

        let mut reflected_state = DiminishingState::default();
        reflected_state.incr_diminishing(DRGroup::Stun, 1_000);
        let reflected =
            reflected_state.snapshot_for_hit(DRGroup::Stun, true, true, true, true, true, 2_000);
        assert!(reflected.diminishes_duration);
        assert_eq!(reflected.apply_to_duration(Some(8_000)), Some(4_000));
    }
}
