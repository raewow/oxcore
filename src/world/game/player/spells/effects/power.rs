//! Power Effects
//!
//! Handles power drain, energize (restore power), and power burn.

use super::{EffectInput, EffectResult};
use crate::world::game::player::power::PowerType;
use crate::world::World;
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
            let idx = power_type as usize;
            let current = player.power.current[idx];
            let drain = drain_amount.min(current);
            player.power.current[idx] = current - drain;
            drain
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

    let energize_amount = input.base_value.max(0) as u32;

    // Determine power type from misc_value (0=Mana, 1=Rage, 3=Energy)
    let power_type = match input.misc_value {
        0 => PowerType::Mana,
        1 => PowerType::Rage,
        3 => PowerType::Energy,
        _ => PowerType::Mana,
    };

    world
        .systems
        .power
        .restore_power(target_guid, power_type, energize_amount, world)?;

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
            let idx = power_type as usize;
            let current = player.power.current[idx];
            // Burn up to the current amount
            let burned = current;
            player.power.current[idx] = 0;
            burned
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
                let new_health = current_health.saturating_sub(damage);
                player.stats.health = new_health;

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
