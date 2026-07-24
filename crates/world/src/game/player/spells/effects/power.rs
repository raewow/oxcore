//! Power Effects
//!
//! Handles power drain, energize (restore power), and power burn.

use super::{EffectInput, EffectResult};
use crate::game::player::power::PowerType;
use crate::World;
use anyhow::Result;

/// SPELL_EFFECT_POWER_DRAIN (8)
///
/// Drains power (mana/energy/rage) from target and gives to caster.
/// Used by mana drain effects.
pub async fn effect_power_drain(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let drain_amount = input.base_value.max(0) as u32;

    // Determine power type from misc_value (0=Mana, 1=Rage, 3=Energy)
    let power_type = match input.misc_value {
        0 => PowerType::Mana,
        1 => PowerType::Rage,
        3 => PowerType::Energy,
        _ => PowerType::Mana,
    };

    // Drain from target
    let actual_drain = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            player
                .power
                .modify_power(power_type, -(drain_amount as i32))
                .unsigned_abs()
        })
        .unwrap_or(0);

    // Give to caster (if caster is not the same as target)
    if target_guid != input.caster_guid {
        world
            .systems
            .power
            .restore_power(input.caster_guid, power_type, actual_drain, world)?;
    }

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ENERGIZE (30)
///
/// Restores power (mana/energy/rage) to target.
/// Used by potions, mana gems, etc.
pub async fn effect_energize(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => input.caster_guid,
    };

    // Reject misc values outside the Powers enum range (< 0 or >= MAX_POWERS).
    let power_type = match input.misc_value {
        0 => PowerType::Mana,
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        _ => return Ok(EffectResult::empty()),
    };

    // Negative energize amounts are ignored (matches `if (damage < 0) return`).
    if input.base_value < 0 {
        return Ok(EffectResult::empty());
    }
    let energize_amount = input.base_value as u32;

    // Skip dead targets and targets that cannot hold this power (GetMaxPower == 0).
    // For non-player targets we have no state here, so fall through (restore_power
    // is itself a no-op for them).
    let skip = world
        .systems
        .player
        .manager()
        .with_player(target_guid, |player| {
            player.stats.health == 0 || player.power.max[power_type as usize] == 0
        })
        .unwrap_or(false);
    if skip {
        return Ok(EffectResult::empty());
    }

    world.systems.spells.energize_by_spell(
        input.caster_guid,
        target_guid,
        input.spell_id,
        energize_amount,
        power_type,
        world,
    )?;

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_POWER_BURN (62)
///
/// Burns power from target and deals damage based on amount burned.
/// Used by effects like Mana Burn.
pub async fn effect_power_burn(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // misc_value usually contains the power type to burn
    // base_value contains the damage multiplier per point of power burned
    let power_type = match input.misc_value {
        0 => PowerType::Mana,
        1 => PowerType::Rage,
        3 => PowerType::Energy,
        _ => PowerType::Mana,
    };

    let damage_per_power = input.base_value.max(0) as u32;

    // Burn power from target
    let power_burned = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            // Burn up to the current amount
            player
                .power
                .modify_power(power_type, -i32::MAX)
                .unsigned_abs()
        })
        .unwrap_or(0);

    // Deal damage based on power burned
    let damage = power_burned * damage_per_power;

    if damage > 0 {
        // Apply damage to target
        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let current_health = player.stats.health;
                player.apply_damage(damage);
                let new_health = player.stats.health;

                tracing::debug!(
                    "Power Burn: {} took {} damage, health: {} -> {}",
                    player.name,
                    damage,
                    current_health,
                    new_health
                );
            });
    }

    Ok(EffectResult::with_damage(damage))
}
