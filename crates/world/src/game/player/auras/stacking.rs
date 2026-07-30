//! Stacking rules for Vanilla WoW auras
//!
//! Vanilla WoW aura stacking follows specific rules that differ from later expansions.

use oxcore_shared::protocol::ObjectGuid;

use super::effects::AURA_MOD_RESISTANCE_EXCLUSIVE;

/// Stacking rules for Vanilla 1.12.
///
/// General principles:
/// 1. Same spell from same caster: REFRESH duration (no extra stack unless spell is stackable)
/// 2. Same spell from different caster: highest value wins (no stacking in most cases)
/// 3. Different spells of same aura type: usually stack additively
/// 4. Explicitly stackable spells (max_stack > 1): increment stack count
///
/// Notable exceptions handled per spell:
/// - Sunder Armor: stacks 5 times from any caster (shared debuff)
/// - Mortal Strike heal debuff: does NOT stack with Aimed Shot heal debuff
/// - Power Word: Shield Weakened Soul: prevents reapplication
///
/// See old implementation: server/src/world/game/aura/container.rs lines 79-109

/// Determine how to handle a new aura application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackAction {
    /// Add new aura (first application)
    AddNew,
    /// Refresh existing aura's duration
    RefreshDuration,
    /// Increment stack count and refresh duration
    AddStack,
    /// Replace existing aura (higher value wins)
    Replace,
    /// Do nothing (existing aura is better)
    Ignore,
    /// Cannot apply (exclusion rule, e.g., Weakened Soul)
    Blocked,
}

/// Resolution for passive exclusive aura effects in the same modifier domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusiveAuraAction {
    Coexist,
    Reject,
    ReplaceExisting,
}

/// Whether an effect participates in the explicit passive exclusive-resistance domain.
pub fn compute_exclusive(is_passive: bool, aura_type: u32) -> bool {
    is_passive && aura_type == AURA_MOD_RESISTANCE_EXCLUSIVE
}

/// Whether two exclusive effects compete for the same modifier domain.
pub fn check_exclusive_with(
    first_passive: bool,
    first_aura_type: u32,
    first_misc_value: i32,
    second_passive: bool,
    second_aura_type: u32,
    second_misc_value: i32,
) -> bool {
    compute_exclusive(first_passive, first_aura_type)
        && compute_exclusive(second_passive, second_aura_type)
        && first_misc_value == second_misc_value
}

/// Decide whether a new exclusive effect may apply beside an existing effect.
pub fn exclusive_aura_can_apply(
    existing_passive: bool,
    existing_aura_type: u32,
    existing_misc_value: i32,
    existing_value: i32,
    new_passive: bool,
    new_aura_type: u32,
    new_misc_value: i32,
    new_value: i32,
) -> ExclusiveAuraAction {
    if !check_exclusive_with(
        existing_passive,
        existing_aura_type,
        existing_misc_value,
        new_passive,
        new_aura_type,
        new_misc_value,
    ) {
        return ExclusiveAuraAction::Coexist;
    }

    if new_value > existing_value {
        ExclusiveAuraAction::ReplaceExisting
    } else {
        ExclusiveAuraAction::Reject
    }
}

/// Determine the stack action based on existing and new aura data.
///
/// The same-caster branch: a holder can only be refreshed in place (duration reset, no new stack)
/// when the spell has neither a stack amount nor proc charges. `existing_max_charges` corresponds
/// to the spell's proc charges — a charge-based aura (e.g. Enrage-style procs with limited
/// charges) must never be silently refreshed, even if it also isn't stackable, since that would
/// reset the charge count for free. Such auras fall through to plain replacement.
#[allow(clippy::too_many_arguments)]
pub fn determine_stack_action(
    existing_spell_id: u32,
    existing_caster: ObjectGuid,
    existing_value: i32,
    existing_stacks: u8,
    existing_max_stacks: u8,
    existing_max_charges: u8,
    new_spell_id: u32,
    new_caster: ObjectGuid,
    new_value: i32,
) -> StackAction {
    if existing_spell_id != new_spell_id {
        // Different spell - check exclusion rules
        if is_exclusive_pair(existing_spell_id, new_spell_id) {
            return StackAction::Blocked;
        }
        return StackAction::AddNew;
    }

    // Same spell
    if existing_caster == new_caster {
        // Same caster, same spell
        if existing_max_stacks > 1 && existing_stacks < existing_max_stacks {
            return StackAction::AddStack;
        }
        // CanBeRefreshedBy: a charge-based aura is neither stacked nor plainly refreshed —
        // refreshing would reset its charge count, so treat it like a replace instead.
        if existing_max_charges != 0 {
            return StackAction::Replace;
        }
        return StackAction::RefreshDuration;
    }

    // Different caster, same spell
    if is_stackable_from_different_casters(existing_spell_id) {
        if existing_max_stacks > 1 && existing_stacks < existing_max_stacks {
            return StackAction::AddStack;
        }
        return StackAction::RefreshDuration;
    }

    // Default: highest value wins
    if new_value > existing_value {
        StackAction::Replace
    } else {
        StackAction::Ignore
    }
}

/// Spells that are mutually exclusive (can't have both at once).
fn is_exclusive_pair(spell_a: u32, spell_b: u32) -> bool {
    // Mortal Strike debuff and Aimed Shot debuff (healing reduction)
    let healing_reduce = [
        12294, 21551, 21552, 21553, // Mortal Strike ranks
        19434, 20900, 20901, 20902, 20903, 20904, // Aimed Shot ranks
    ];
    if healing_reduce.contains(&spell_a) && healing_reduce.contains(&spell_b) {
        return spell_a != spell_b; // Same rank can refresh, different rank blocks
    }

    // Weakened Soul prevents Power Word: Shield
    if (spell_a == 6788 && spell_b == 17) || (spell_a == 17 && spell_b == 6788) {
        return true;
    }

    false
}

/// Spells that can stack from different casters.
fn is_stackable_from_different_casters(spell_id: u32) -> bool {
    // Sunder Armor (all ranks)
    matches!(spell_id, 7386 | 7405 | 8380 | 11596 | 11597)
}

/// Check if two spells are the same effect at different ranks.
pub fn is_same_spell_different_rank(spell_a: u32, spell_b: u32) -> bool {
    // This is a simplified check - in practice you'd look up spell_family_name
    // and spell_family_flags in the DBC to determine if two spells are the same
    // base effect at different ranks.

    // For now, just check some known spell families
    let sunder_armor = [7386, 7405, 8380, 11596, 11597];
    if sunder_armor.contains(&spell_a) && sunder_armor.contains(&spell_b) {
        return true;
    }

    let mortal_strike = [12294, 21551, 21552, 21553];
    if mortal_strike.contains(&spell_a) && mortal_strike.contains(&spell_b) {
        return true;
    }

    let battle_shout = [6673, 5242, 6192, 11549, 11550, 11551, 25289];
    if battle_shout.contains(&spell_a) && battle_shout.contains(&spell_b) {
        return true;
    }

    false
}

/// Decide which of two auras competing for a limited visible buff/debuff slot should win,
/// when the visible-slot cap (31 buffs / 16 debuffs) has been exceeded.
///
/// `self` wins the slot if `self_score > other_score`; ties are broken by apply time, with the
/// more recently applied aura winning. `score` is the visible-slot-limit priority score (out of
/// scope here — no visible-slot-limit eviction path exists yet in `AuraContainer`, which
/// currently just refuses new auras once all slots in a category are full instead of evicting a
/// lower-priority one).
pub fn is_more_important_visual_aura_than(
    self_score: i32,
    self_apply_time: u64,
    other_score: i32,
    other_apply_time: u64,
) -> bool {
    if self_score == other_score {
        return self_apply_time > other_apply_time;
    }
    self_score > other_score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(low: u32) -> ObjectGuid {
        ObjectGuid::new_player(low)
    }

    // --- determine_stack_action: different spell ---

    #[test]
    fn different_spell_default_adds_new() {
        let action = determine_stack_action(100, guid(1), 10, 1, 1, 0, 200, guid(1), 10);
        assert_eq!(action, StackAction::AddNew);
    }

    #[test]
    fn different_spell_exclusive_pair_blocked() {
        // Mortal Strike rank 1 vs Aimed Shot rank 1: mutually exclusive healing-reduce debuffs.
        let action = determine_stack_action(12294, guid(1), 10, 1, 1, 0, 19434, guid(2), 10);
        assert_eq!(action, StackAction::Blocked);
    }

    #[test]
    fn different_spell_exclusive_pair_same_rank_not_blocked() {
        // Same spell id never hits the exclusion check (handled by the "same spell" branch).
        let action = determine_stack_action(12294, guid(1), 10, 1, 1, 0, 12294, guid(1), 10);
        assert_ne!(action, StackAction::Blocked);
    }

    // --- determine_stack_action: same spell, same caster ---

    #[test]
    fn same_caster_non_stackable_no_charges_refreshes() {
        let action = determine_stack_action(100, guid(1), 10, 1, 1, 0, 100, guid(1), 10);
        assert_eq!(action, StackAction::RefreshDuration);
    }

    #[test]
    fn same_caster_stackable_below_cap_adds_stack() {
        let action = determine_stack_action(100, guid(1), 10, 2, 5, 0, 100, guid(1), 10);
        assert_eq!(action, StackAction::AddStack);
    }

    #[test]
    fn same_caster_stackable_at_cap_refreshes_instead_of_stacking() {
        let action = determine_stack_action(100, guid(1), 10, 5, 5, 0, 100, guid(1), 10);
        assert_eq!(action, StackAction::RefreshDuration);
    }

    #[test]
    fn same_caster_charge_based_replaces_instead_of_refreshing() {
        // CanBeRefreshedBy excludes procCharges auras from plain refresh even when not
        // stackable, since refreshing would silently reset the remaining charge count.
        let action = determine_stack_action(100, guid(1), 10, 1, 1, 3, 100, guid(1), 10);
        assert_eq!(action, StackAction::Replace);
    }

    // --- determine_stack_action: same spell, different caster ---

    #[test]
    fn different_caster_stackable_spell_adds_stack() {
        // Sunder Armor: stacks from any caster.
        let action = determine_stack_action(7386, guid(1), 10, 1, 5, 0, 7386, guid(2), 10);
        assert_eq!(action, StackAction::AddStack);
    }

    #[test]
    fn different_caster_non_stackable_higher_value_replaces() {
        let action = determine_stack_action(100, guid(1), 10, 1, 1, 0, 100, guid(2), 20);
        assert_eq!(action, StackAction::Replace);
    }

    #[test]
    fn different_caster_non_stackable_lower_value_ignored() {
        let action = determine_stack_action(100, guid(1), 20, 1, 1, 0, 100, guid(2), 10);
        assert_eq!(action, StackAction::Ignore);
    }

    #[test]
    fn exclusive_passive_effects_compete_only_in_the_same_domain() {
        assert!(compute_exclusive(true, AURA_MOD_RESISTANCE_EXCLUSIVE));
        assert!(!compute_exclusive(false, AURA_MOD_RESISTANCE_EXCLUSIVE));

        assert_eq!(
            exclusive_aura_can_apply(
                true,
                AURA_MOD_RESISTANCE_EXCLUSIVE,
                4,
                10,
                true,
                AURA_MOD_RESISTANCE_EXCLUSIVE,
                4,
                15,
            ),
            ExclusiveAuraAction::ReplaceExisting
        );
        assert_eq!(
            exclusive_aura_can_apply(
                true,
                AURA_MOD_RESISTANCE_EXCLUSIVE,
                4,
                10,
                true,
                AURA_MOD_RESISTANCE_EXCLUSIVE,
                4,
                10,
            ),
            ExclusiveAuraAction::Reject
        );
        assert_eq!(
            exclusive_aura_can_apply(
                true,
                AURA_MOD_RESISTANCE_EXCLUSIVE,
                4,
                10,
                true,
                AURA_MOD_RESISTANCE_EXCLUSIVE,
                5,
                15,
            ),
            ExclusiveAuraAction::Coexist
        );
    }

    // --- is_more_important_visual_aura_than ---

    #[test]
    fn higher_score_wins_regardless_of_apply_time() {
        assert!(is_more_important_visual_aura_than(5, 100, 3, 999));
        assert!(!is_more_important_visual_aura_than(3, 999, 5, 100));
    }

    #[test]
    fn equal_score_more_recently_applied_wins() {
        assert!(is_more_important_visual_aura_than(5, 200, 5, 100));
        assert!(!is_more_important_visual_aura_than(5, 100, 5, 200));
    }

    #[test]
    fn equal_score_and_time_is_not_more_important() {
        assert!(!is_more_important_visual_aura_than(5, 100, 5, 100));
    }
}
