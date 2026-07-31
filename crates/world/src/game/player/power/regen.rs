//! Power regeneration formulas
//!
//! This module contains pure functions for calculating power regeneration.
//! All functions are stateless and operate only on their inputs.

/// === MANA REGENERATION ===
///
/// Mana regen has two components:
/// 1. Spirit-based regen: only active when NOT casting (5-second rule)
///    - OR partially active if player has "mana regen while casting" (Meditation talent)
/// 2. MP5 (mana per 5): always active regardless of casting
///
/// The 5-second rule:
/// - After using a mana-costing ability, spirit-based regen stops for 5 seconds
/// - MP5 from gear/enchants always works regardless
/// - Talents like Meditation allow a % of spirit regen to work while casting

/// Calculate mana regen per tick (called every 2 seconds).
///
/// `full_regen_per_second` and `interrupt_regen_per_second` are precomputed by
/// StatsSystem from spirit, MP5 auras, and interrupt-percentage auras.
pub fn calculate_mana_regen_per_tick(
    full_regen_per_second: f32,
    interrupt_regen_per_second: f32,
    spirit_regen_active: bool,
) -> f32 {
    let regen_per_second = if spirit_regen_active {
        full_regen_per_second
    } else {
        interrupt_regen_per_second
    };

    regen_per_second.max(0.0) * 2.0
}

/// === RAGE MECHANICS ===
///
/// Rage generation:
/// - From damage dealt (melee): rage = damage / conversion * 7.5
/// - From damage taken: rage = damage / conversion * 2.5
/// - Stored in tenths: 1000 internal rage is displayed as 100 rage by the client
///
/// Rage decay:
/// - Out of combat: loses 2 rage per second (4 per 2-second tick)

fn rage_conversion(level: u8) -> f32 {
    let level = level as f32;
    (0.009_110_784 * level * level) + (3.225_598_1 * level) + 4.265_291
}

/// Calculate internal rage from damage dealt
pub fn rage_from_damage_dealt(damage: u32, level: u8) -> u32 {
    if level == 0 {
        return 0;
    }
    let rage = (damage as f32 / rage_conversion(level)) * 7.5;
    (rage * 10.0) as u32
}

/// Calculate internal rage from damage taken
pub fn rage_from_damage_taken(damage: u32, level: u8) -> u32 {
    if level == 0 {
        return 0;
    }
    let rage = (damage as f32 / rage_conversion(level)) * 2.5;
    (rage * 10.0) as u32
}

/// Internal rage decay per 2-second tick (out of combat only)
pub const RAGE_DECAY_PER_TICK: u32 = 40;

/// Maximum internal rage value
pub const MAX_RAGE: u32 = 1000;

/// === ENERGY MECHANICS ===
///
/// Energy regeneration:
/// - Fixed 20 energy per 2 seconds
/// - Always regenerates (in combat and out)
/// - Capped at 100

pub const ENERGY_REGEN_PER_TICK: u32 = 20;
pub const MAX_ENERGY: u32 = 100;

/// === FOCUS MECHANICS (Hunter Pet) ===
pub const FOCUS_REGEN_PER_TICK: u32 = 24;
pub const MAX_FOCUS: u32 = 100;

/// === HAPPINESS MECHANICS (Hunter Pet) ===
/// Internal happiness is in tenths of a point; the client shows 0–100 happiness bars
/// mapping to three tiers: unhappy < 333333, content < 666666, happy = 1050000.
pub const MAX_HAPPINESS: u32 = 1050000;

/// === EATING/DRINKING ===
///
/// Food/drink are auras that provide regeneration:
/// - Food: restores X health per tick while sitting
/// - Drink: restores X mana per tick while sitting
/// These are handled by the aura system (Phase 4) as periodic heal/energize auras

/// === HEALTH REGENERATION (for reference) ===
///
/// Health regen is handled separately from power regen:
/// - Out of combat: spirit-based regen
/// - In combat: 0 for most classes (druids have talents for in-combat regen)
/// - Eating: additional health regen from food
///
/// Formula (out of combat):
/// HP/second = (Spirit * 0.25) + (some level-based factor)
///
/// Note: Health regen is typically handled in the stats or combat system

/// Calculate health regen per tick (2 seconds) from spirit.
/// The coefficient scales with level.
/// Returns f32 because callers use a carry accumulator for fractional amounts.
pub fn calculate_health_regen_per_tick(spirit: u32, level: u8) -> f32 {
    let hp_per_spirit: f32 = if level < 20 {
        0.20
    } else if level < 30 {
        0.22
    } else if level < 40 {
        0.25
    } else if level < 50 {
        0.27
    } else if level < 60 {
        0.28
    } else {
        0.30
    };
    hp_per_spirit * spirit as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mana_regen_tick_uses_full_regen_outside_five_second_rule() {
        assert_eq!(calculate_mana_regen_per_tick(12.5, 4.0, true), 25.0);
    }

    #[test]
    fn mana_regen_tick_uses_interrupt_regen_inside_five_second_rule() {
        assert_eq!(calculate_mana_regen_per_tick(12.5, 4.0, false), 8.0);
    }

    #[test]
    fn rage_from_damage_dealt_uses_reference_conversion_and_tenths() {
        // uint32((damage / rageConversion * 7.5) * 10).
        assert_eq!(rage_from_damage_dealt(100, 60), 32);
    }

    #[test]
    fn rage_from_damage_taken_uses_reference_conversion_and_tenths() {
        // uint32((damage / rageConversion * 2.5) * 10).
        assert_eq!(rage_from_damage_taken(100, 60), 10);
    }

    #[test]
    fn rage_constants_are_internal_tenths_not_display_points() {
        assert_eq!(MAX_RAGE, 1000);
        assert_eq!(RAGE_DECAY_PER_TICK, 40);
    }
}
