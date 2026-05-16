//! Combat Mechanics Spell Effects
//!
//! Handles combat-related effects: threat, combo points, extra attacks, defense.

use super::{EffectInput, EffectResult};
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_ADD_EXTRA_ATTACKS (19)
///
/// Add extra melee attacks to the caster.
/// Used by Windfury and similar effects.
pub async fn effect_add_extra_attacks(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let attack_count = input.base_value.max(1) as u32;

    // TODO: Add extra attacks to caster's next swing

    tracing::debug!(
        "Add extra attacks: caster={:?} count={}",
        input.caster_guid,
        attack_count
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DODGE (20)
///
/// Enable dodge ability.
/// Used by passive dodge skills.
pub async fn effect_dodge(_input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // Dodge is typically handled by auras (SPELL_AURA_MOD_DODGE_PERCENT)
    // This effect is mostly a marker/passive
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PARRY (22)
///
/// Enable parry ability.
/// Used by passive parry skills.
pub async fn effect_parry(_input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // Parry is typically handled by auras (SPELL_AURA_MOD_PARRY_PERCENT)
    // This effect is mostly a marker/passive
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_BLOCK (23)
///
/// Enable block ability.
/// Used by passive block skills.
pub async fn effect_block(_input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // Block is typically handled by auras (SPELL_AURA_MOD_BLOCK_PERCENT)
    // This effect is mostly a marker/passive
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DUAL_WIELD (40)
///
/// Enable dual wielding for the player.
pub async fn effect_dual_wield(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Enable dual wield skill
    world
        .systems
        .player
        .manager()
        .with_player_mut(input.caster_guid, |player| {
            player.combat.can_dual_wield = true;

            // Learn dual wield spell (674)
            player.spells.learn_spell(674);

            tracing::debug!("Dual wield enabled: caster={:?}", input.caster_guid);
        });

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_THREAT (63)
///
/// Modify threat on the target.
pub async fn effect_threat(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let threat_amount = input.base_value;

    // TODO: Modify threat on target

    tracing::debug!(
        "Threat modify: caster={:?} target={:?} amount={}",
        input.caster_guid,
        target_guid,
        threat_amount
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_INTERRUPT_CAST (68)
///
/// Interrupt the target's spell cast.
pub async fn effect_interrupt_cast(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // Get interrupt parameters
    let lockout_school = input.misc_value as u8;
    let lockout_duration_ms = input.base_value.max(0) as u32;

    // TODO: Interrupt any active cast on target
    // Apply school lockout if specified

    if lockout_duration_ms > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                player
                    .spells
                    .apply_school_lockout(lockout_school, lockout_duration_ms, now);
            });
    }

    tracing::debug!(
        "Interrupt cast: caster={:?} target={:?} school={} duration={}ms",
        input.caster_guid,
        target_guid,
        lockout_school,
        lockout_duration_ms
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DISTRACT (69)
///
/// Distract NPCs at the target location.
/// Used by Rogue Distract.
pub async fn effect_distract(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // TODO: Get distract location from spell target
    // Find all NPCs in radius and distract them

    tracing::debug!("Distract: caster={:?}", input.caster_guid);

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SANCTUARY (79)
///
/// Apply sanctuary effect (PvP protection).
pub async fn effect_sanctuary(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // TODO: Apply sanctuary aura to target
    // This prevents PvP combat

    tracing::debug!("Sanctuary: target={:?}", target_guid);

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ADD_COMBO_POINTS (80)
///
/// Add combo points to the target.
pub async fn effect_add_combo_points(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let combo_points = input.base_value.max(0) as u8;

    // Add combo points
    world
        .systems
        .player
        .manager()
        .with_player_mut(input.caster_guid, |player| {
            player.combat.combo_target = Some(target_guid);
            player.combat.combo_points = (player.combat.combo_points + combo_points).min(5);

            tracing::debug!(
                "Add combo points: caster={:?} target={:?} points={} (total: {})",
                input.caster_guid,
                target_guid,
                combo_points,
                player.combat.combo_points
            );
        });

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ATTACK_ME (114)
///
/// Force the target to attack the caster (taunt).
pub async fn effect_attack_me(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // TODO: Taunt the target to attack caster

    tracing::debug!(
        "Taunt: caster={:?} target={:?}",
        input.caster_guid,
        target_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_MODIFY_THREAT_PERCENT (125)
///
/// Modify threat by percentage.
pub async fn effect_modify_threat_percent(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let threat_percent = input.base_value;

    // TODO: Modify threat by percent

    tracing::debug!(
        "Modify threat percent: caster={:?} target={:?} percent={}%",
        input.caster_guid,
        target_guid,
        threat_percent
    );

    Ok(EffectResult::empty())
}
