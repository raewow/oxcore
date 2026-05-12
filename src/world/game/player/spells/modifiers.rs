//! Spell Modifiers
//!
//! Spell modifiers come from talents (SPELL_AURA_ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER)
//! and some auras. They modify properties of spells the player casts.

use crate::shared::protocol::ObjectGuid;
use crate::world::game::player::spells::state::{SpellMod, SpellModOp, SpellModType};
use crate::world::World;
use anyhow::Result;

/// Add a spell modifier (from a talent or aura being applied).
///
/// Called by AuraSystem when applying SPELL_AURA_ADD_FLAT_MODIFIER (107)
/// or SPELL_AURA_ADD_PCT_MODIFIER (108) auras.
///
/// Parameters:
/// - `op`: Which property to modify (from spell DBC effect_misc_value)
/// - `mod_type`: Flat or Pct (from aura type: 107=Flat, 108=Pct)
/// - `value`: Modifier value (from aura current_value)
/// - `spell_family_mask`: Which spells are affected (from spell DBC spell_family_flags)
/// - `spell_family_name`: Which spell family (from spell DBC spell_family_name)
/// - `source_spell_id`: The talent/aura spell providing this modifier
#[allow(dead_code)]
pub fn add_spell_modifier(
    player_guid: ObjectGuid,
    op: SpellModOp,
    mod_type: SpellModType,
    value: i32,
    spell_family_mask: u64,
    spell_family_name: u32,
    source_spell_id: u32,
    source_aura_slot: Option<u8>,
    world: &World,
) -> Result<()> {
    world.systems.player.manager().with_player_mut(player_guid, |player| {
        player.spells.spell_modifiers.push(SpellMod {
            op,
            mod_type,
            value,
            spell_family_mask,
            spell_family_name,
            source_spell_id,
            source_aura_slot,
        });
    });

    Ok(())
}

/// Remove all spell modifiers from a specific source spell.
///
/// Called by AuraSystem when removing a talent/aura that provided spell modifiers.
pub fn remove_spell_modifier(
    player_guid: ObjectGuid,
    source_spell_id: u32,
    world: &World,
) -> Result<()> {
    world.systems.player.manager().with_player_mut(player_guid, |player| {
        player.spells
            .spell_modifiers
            .retain(|m| m.source_spell_id != source_spell_id);
    });

    Ok(())
}

/// Apply all matching spell modifiers to a value.
///
/// Used during spell calculations to get the modified value of a property.
/// For example, to get modified damage:
///   let damage = apply_spell_modifiers(player, SpellModOp::Damage, base_damage, spell_entry);
#[allow(dead_code)]
pub fn apply_spell_modifiers_to_value(
    modifiers: &[SpellMod],
    op: SpellModOp,
    base_value: i32,
    spell_family_name: u32,
    spell_family_flags: u64,
) -> i32 {
    let mut flat_total = 0i32;
    let mut pct_total = 0i32;

    for spell_mod in modifiers {
        if spell_mod.op != op {
            continue;
        }

        // Check if this modifier applies to the spell
        if !does_modifier_apply(spell_mod, spell_family_name, spell_family_flags) {
            continue;
        }

        match spell_mod.mod_type {
            SpellModType::Flat => {
                flat_total += spell_mod.value;
            }
            SpellModType::Pct => {
                pct_total += spell_mod.value;
            }
        }
    }

    // Apply flat first, then percentage
    let after_flat = base_value + flat_total;
    let after_pct = (after_flat as f32 * (1.0 + pct_total as f32 / 100.0)) as i32;

    after_pct.max(0)
}

/// Check if a spell modifier applies to a specific spell.
fn does_modifier_apply(
    spell_mod: &SpellMod,
    spell_family_name: u32,
    spell_family_flags: u64,
) -> bool {
    // Must match spell family name
    if spell_mod.spell_family_name != 0 && spell_mod.spell_family_name != spell_family_name {
        return false;
    }

    // Must match spell family flags mask
    if spell_mod.spell_family_mask != 0 && (spell_mod.spell_family_mask & spell_family_flags) == 0 {
        return false;
    }

    true
}

/// Calculate modified cast time for a spell.
///
/// Applies haste and talent modifiers to base cast time.
#[allow(dead_code)]
pub fn calculate_modified_cast_time(
    player_guid: ObjectGuid,
    base_cast_time_ms: u32,
    _spell_family_name: u32,
    _spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_cast_time_ms as i32;

    // Apply cast time modifiers from talents/auras (SpellModOp::CastTime)
    world.systems.player.manager().with_player_mut(player_guid, |player| {
        for spell_mod in &player.spells.spell_modifiers {
            if spell_mod.op == SpellModOp::CastTime {
                // TODO: Check spell_family_mask matches
                match spell_mod.mod_type {
                    SpellModType::Flat => {
                        modified += spell_mod.value;
                    }
                    SpellModType::Pct => {
                        modified = (modified as f32 * (1.0 + spell_mod.value as f32 / 100.0))
                            as i32;
                    }
                }
            }
        }
    });

    modified.max(0) as u32
}

/// Calculate modified power cost for a spell.
#[allow(dead_code)]
pub fn calculate_modified_power_cost(
    player_guid: ObjectGuid,
    base_cost: u32,
    _spell_family_name: u32,
    _spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_cost as i32;

    // Apply cost modifiers from talents/auras (SpellModOp::Cost)
    world.systems.player.manager().with_player_mut(player_guid, |player| {
        for spell_mod in &player.spells.spell_modifiers {
            if spell_mod.op == SpellModOp::Cost {
                // TODO: Check spell_family_mask matches
                match spell_mod.mod_type {
                    SpellModType::Flat => {
                        modified += spell_mod.value;
                    }
                    SpellModType::Pct => {
                        modified = (modified as f32 * (1.0 + spell_mod.value as f32 / 100.0))
                            as i32;
                    }
                }
            }
        }
    });

    modified.max(0) as u32
}

/// Calculate modified cooldown for a spell.
#[allow(dead_code)]
pub fn calculate_modified_cooldown(
    player_guid: ObjectGuid,
    base_cooldown_ms: u32,
    _spell_family_name: u32,
    _spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_cooldown_ms as i32;

    // Apply cooldown modifiers from talents/auras (SpellModOp::Cooldown)
    world.systems.player.manager().with_player_mut(player_guid, |player| {
        for spell_mod in &player.spells.spell_modifiers {
            if spell_mod.op == SpellModOp::Cooldown {
                // TODO: Check spell_family_mask matches
                match spell_mod.mod_type {
                    SpellModType::Flat => {
                        modified += spell_mod.value;
                    }
                    SpellModType::Pct => {
                        modified = (modified as f32 * (1.0 + spell_mod.value as f32 / 100.0))
                            as i32;
                    }
                }
            }
        }
    });

    modified.max(0) as u32
}

/// Calculate modified GCD for a spell.
#[allow(dead_code)]
pub fn calculate_modified_gcd(
    player_guid: ObjectGuid,
    base_gcd_ms: u32,
    _spell_family_name: u32,
    _spell_family_flags: u64,
    world: &World,
) -> u32 {
    let mut modified = base_gcd_ms as i32;

    // Apply GCD modifiers from talents/auras (SpellModOp::GlobalCooldown)
    world.systems.player.manager().with_player_mut(player_guid, |player| {
        for spell_mod in &player.spells.spell_modifiers {
            if spell_mod.op == SpellModOp::GlobalCooldown {
                // TODO: Check spell_family_mask matches
                match spell_mod.mod_type {
                    SpellModType::Flat => {
                        modified += spell_mod.value;
                    }
                    SpellModType::Pct => {
                        modified = (modified as f32 * (1.0 + spell_mod.value as f32 / 100.0))
                            as i32;
                    }
                }
            }
        }

        // Minimum GCD is 1000ms (Vanilla cap)
        modified = modified.max(1000);
    });

    modified.max(0) as u32
}
