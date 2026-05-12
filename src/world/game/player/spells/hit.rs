//! Spell Hit Resolution
//!
//! Determines whether a spell hits, misses, or is resisted.
//! Equivalent to MaNGOS Unit::SpellHitResult() / MagicSpellHitResult().
//!
//! Vanilla WoW spell hit mechanics:
//! - Base miss rate: 4% for same-level targets
//! - +1% per level difference (target higher than caster)
//! - Spell hit rating from gear/talents reduces miss chance
//! - Binary spells: full resist or full land
//! - Non-binary spells: partial resist (0/25/50/75/100%)
//! - Physical spells use melee miss table instead

use crate::shared::protocol::ObjectGuid;
use crate::world::World;

/// Result of a spell hit roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellHitOutcome {
    /// Spell hits normally
    Hit,
    /// Spell misses entirely
    Miss,
    /// Full resist (binary spells)
    Resist,
    /// Partial resist: percentage of damage resisted (25, 50, or 75)
    PartialResist(u8),
    /// Target is immune to the spell's school or mechanic
    Immune,
    /// Spell reflected back at caster
    Reflect,
}

impl SpellHitOutcome {
    pub fn is_hit(&self) -> bool {
        matches!(self, Self::Hit | Self::PartialResist(_))
    }
}

/// Roll spell hit for a target.
///
/// Returns the outcome (hit, miss, resist, immune, reflect).
/// Physical spells (school=0) use melee hit table.
/// Magic spells use spell hit table with resist rolls.
pub fn roll_spell_hit(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    world: &World,
) -> SpellHitOutcome {
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return SpellHitOutcome::Hit, // Unknown spell = auto-hit
    };

    let school = spell_entry.school as u8;

    // Get caster level and spell hit bonus
    let (caster_level, spell_hit_bonus) = if caster_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(caster_guid, |p| {
                // Sum up hit chance from auras (AURA_MOD_SPELL_HIT_CHANCE + AURA_MOD_HIT_CHANCE)
                use crate::world::game::player::auras::effects::{
                    AURA_MOD_HIT_CHANCE, AURA_MOD_SPELL_HIT_CHANCE,
                };
                let mut hit_bonus = 0i32;
                for aura in p.auras.container.all_auras() {
                    if aura.aura_type == AURA_MOD_SPELL_HIT_CHANCE
                        || aura.aura_type == AURA_MOD_HIT_CHANCE
                    {
                        hit_bonus += aura.current_value();
                    }
                }
                (p.level as i32, hit_bonus)
            })
            .unwrap_or((60, 0))
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |c| (c.level as i32, 0i32))
            .unwrap_or((60, 0))
    };

    // Get target level and resistances
    let (target_level, target_resistance) = if target_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(target_guid, |p| {
                let resist = if school != 0 && (school as usize) < 7 {
                    p.stats.resistances[school as usize]
                } else {
                    0
                };
                (p.level as i32, resist)
            })
            .unwrap_or((60, 0))
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |c| {
                (c.level as i32, 0u32) // TODO: creature resistances
            })
            .unwrap_or((60, 0))
    };

    // Step 1: Miss check
    let level_diff = target_level - caster_level;

    // Base miss rate: 4% + 1% per level difference, min 1%
    let base_miss_pct = if level_diff > 2 {
        // Boss-level targets (3+ levels above): steeper miss rate
        5.0 + (level_diff as f32 - 2.0) * 2.0
    } else {
        (4.0 + level_diff as f32).max(1.0)
    };

    // Spell hit from gear/talents reduces miss chance
    let miss_pct = (base_miss_pct - spell_hit_bonus as f32).max(1.0);

    let miss_roll: f32 = rand::random::<f32>() * 100.0;
    if miss_roll < miss_pct {
        return SpellHitOutcome::Miss;
    }

    // Step 2: Resistance check (magic spells only)
    if school != 0 {
        // Check binary resist
        let is_binary = is_binary_spell(spell_id, world);

        if is_binary {
            // Binary: full resist or full land
            let resist_chance = calculate_resist_chance(caster_level, target_resistance, school);
            let resist_roll: f32 = rand::random::<f32>() * 100.0;
            if resist_roll < resist_chance {
                return SpellHitOutcome::Resist;
            }
        } else {
            // Non-binary: partial resist roll (0/25/50/75/100%)
            let partial = roll_partial_resist(caster_level, target_resistance, school);
            if partial == 100 {
                return SpellHitOutcome::Resist;
            }
            if partial > 0 {
                return SpellHitOutcome::PartialResist(partial);
            }
        }
    }

    SpellHitOutcome::Hit
}

/// Check if a spell is "binary" (all-or-nothing resist).
///
/// Binary spells are those with non-damage effects that either fully apply or
/// fully resist (like Polymorph, Fear, Silence). Damage-only spells use
/// partial resist.
fn is_binary_spell(spell_id: u32, world: &World) -> bool {
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return false,
    };

    // A spell is binary if ALL its effects are non-damage aura effects
    // (Damage spells use partial resist instead)
    for i in 0..3 {
        let effect = spell_entry.effect[i];
        if effect == 0 {
            continue;
        }
        // School damage (2), weapon damage (58, 17, 31, 121), health leech (9) = NOT binary
        if matches!(effect, 2 | 9 | 17 | 31 | 58 | 121) {
            return false;
        }
    }

    true
}

/// Calculate binary resist chance based on resistance and caster level.
/// Vanilla formula: resistance / (caster_level * 5) * 75
fn calculate_resist_chance(caster_level: i32, resistance: u32, _school: u8) -> f32 {
    if resistance == 0 {
        return 0.0;
    }
    let resist_pct = resistance as f32 / (caster_level as f32 * 5.0);
    (resist_pct * 75.0).min(75.0)
}

/// Roll for partial resistance.
/// Returns the percentage of damage resisted (0, 25, 50, 75, or 100).
///
/// Vanilla formula based on average resistance percentage:
/// avg_resist% = target_resistance / (caster_level * 5)
/// Then distributed across the 5 possible outcomes using a weighted table.
fn roll_partial_resist(caster_level: i32, resistance: u32, _school: u8) -> u8 {
    if resistance == 0 {
        return 0;
    }

    let avg_resist = (resistance as f32 / (caster_level as f32 * 5.0)).min(0.75);

    // Simplified distribution: roll against average resist
    let roll: f32 = rand::random::<f32>();

    if avg_resist < 0.01 {
        return 0; // Negligible resistance
    }

    // Weight table based on average resistance
    // Higher resistance shifts distribution toward more resist
    let threshold_100 = avg_resist * avg_resist; // Full resist very rare at low levels
    let threshold_75 = avg_resist * 0.5;
    let threshold_50 = avg_resist * 0.8;
    let threshold_25 = avg_resist * 1.2;

    if roll < threshold_100 {
        100
    } else if roll < threshold_100 + threshold_75 {
        75
    } else if roll < threshold_100 + threshold_75 + threshold_50 {
        50
    } else if roll < threshold_100 + threshold_75 + threshold_50 + threshold_25 {
        25
    } else {
        0
    }
}
