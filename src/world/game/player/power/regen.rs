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

/// Calculate mana regen per tick (called every 2 seconds)
/// spirit_regen: base mana regen from spirit (from StatsState.mana_regen_base)
/// mp5: mana per 5 seconds from gear
/// spirit_regen_active: whether 5-second rule allows spirit regen
/// casting_regen_pct: % of spirit regen allowed while casting (0-100)
pub fn calculate_mana_regen_per_tick(
    spirit_regen: f32,
    mp5: f32,
    spirit_regen_active: bool,
    casting_regen_pct: f32,
) -> f32 {
    // MP5 always applies (converted to per-2-second tick)
    let mp5_per_tick = mp5 * 2.0 / 5.0;

    // Spirit regen component
    let spirit_component = if spirit_regen_active {
        // Full spirit regen (not casting or 5s elapsed)
        spirit_regen * 2.0 / 5.0
    } else {
        // Partial spirit regen while casting (from talents)
        spirit_regen * 2.0 / 5.0 * (casting_regen_pct / 100.0)
    };

    mp5_per_tick + spirit_component
}

/// === RAGE MECHANICS ===
///
/// Rage generation:
/// - From damage dealt (melee): rage = damage * 7.5 / player_level
/// - From damage taken: rage = damage * 2.5 / player_level
/// - Capped at 100
///
/// Rage decay:
/// - Out of combat: loses 2 rage per second (4 per 2-second tick)

/// Calculate rage from damage dealt
pub fn rage_from_damage_dealt(damage: u32, level: u8) -> u32 {
    if level == 0 {
        return 0;
    }
    let rage = damage as f32 * 7.5 / level as f32;
    rage.min(100.0) as u32
}

/// Calculate rage from damage taken
pub fn rage_from_damage_taken(damage: u32, level: u8) -> u32 {
    if level == 0 {
        return 0;
    }
    let rage = damage as f32 * 2.5 / level as f32;
    rage.min(100.0) as u32
}

/// Rage decay per 2-second tick (out of combat only)
pub const RAGE_DECAY_PER_TICK: u32 = 4;

/// Maximum rage value
pub const MAX_RAGE: u32 = 100;

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

/// Calculate health regen per tick (2 seconds) from spirit
/// This is a simplified formula - actual implementation may vary
pub fn calculate_health_regen_per_tick(spirit: u32, level: u8) -> u32 {
    if level == 0 {
        return 0;
    }
    // Simplified: spirit * 0.5 per 2 seconds
    let regen = (spirit as f32 * 0.5).ceil() as u32;
    regen
}
