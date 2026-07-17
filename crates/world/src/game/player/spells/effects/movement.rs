//! Movement Effects
//!
//! Handles charge, knockback, leap, pull, and other movement-based spell effects.

use super::{EffectInput, EffectResult};
use crate::World;
use anyhow::Result;

/// SPELL_EFFECT_CHARGE (96)
///
/// Charge to the target (Warrior Charge, etc.)
pub async fn effect_charge(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // TODO: Implement charge movement
    // This requires pathfinding and movement system integration

    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    tracing::debug!(
        "Charge effect: caster {} charging to target {}",
        input.caster_guid,
        target_guid
    );

    // TODO:
    // 1. Get target position
    // 2. Calculate path
    // 3. Start charge movement
    // 4. Apply stun/root when reaching target

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_KNOCK_BACK (98)
///
/// Knock the target back (Thunderfury proc, etc.)
pub async fn effect_knock_back(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let horizontal_speed = input.base_value.max(0) as f32 / 10.0;
    let vertical_speed = input.misc_value.max(0) as f32 / 10.0;

    let Some(target_position) = world.managers.player_mgr.get_position(target_guid) else {
        // Creature knockback needs a separate controller/movement owner.
        return Ok(EffectResult::empty());
    };
    let caster_position = world
        .managers
        .player_mgr
        .get_position(input.caster_guid)
        .or_else(|| world.managers.creature_mgr.get_position(input.caster_guid));
    let angle = caster_position.map_or(target_position.o + std::f32::consts::PI, |caster| {
        (target_position.y - caster.y).atan2(target_position.x - caster.x)
    });

    tracing::debug!(
        "Knockback effect: target {} knocked back by caster {}",
        target_guid,
        input.caster_guid
    );

    world.systems.player.movement().launch_knockback(
        target_guid,
        angle.cos(),
        angle.sin(),
        horizontal_speed,
        vertical_speed,
        world,
    )?;

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_LEAP (29)
///
/// Leap to target location (Heroic Leap, etc.)
pub async fn effect_leap(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // TODO: Implement leap movement
    // Similar to charge but with arc trajectory

    tracing::debug!("Leap effect: caster {}", input.caster_guid);

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PULL (70)
///
/// Pull the target toward the caster.
pub async fn effect_pull(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // TODO: Implement pull effect
    // Opposite of knockback

    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    tracing::debug!(
        "Pull effect: caster {} pulling target {}",
        input.caster_guid,
        target_guid
    );

    Ok(EffectResult::empty())
}
