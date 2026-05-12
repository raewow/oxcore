//! Stacking rules for Vanilla WoW auras
//!
//! Vanilla WoW aura stacking follows specific rules that differ from later expansions.

use crate::shared::protocol::ObjectGuid;

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

/// Determine the stack action based on existing and new aura data.
pub fn determine_stack_action(
    existing_spell_id: u32,
    existing_caster: ObjectGuid,
    existing_value: i32,
    existing_stacks: u8,
    existing_max_stacks: u8,
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
