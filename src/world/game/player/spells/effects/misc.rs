//! Miscellaneous Spell Effects
//!
//! Handles effects that don't fit in other categories:
//! - Dummy effects
//! - Instakill
//! - Learn spell
//! - Trigger spell
/// - Interrupt cast

use super::{EffectInput, EffectResult};
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_INSTAKILL (1)
///
/// Instantly kills the target.
pub async fn effect_insta_kill(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // Set target health to 0
    world.systems.player.manager().with_player_mut(target_guid, |player| {
        let old_health = player.stats.health;
        player.stats.health = 0;

        tracing::debug!(
            "Instakill: {} was killed by spell {}, health: {} -> 0",
            player.name, input.spell_id, old_health
        );
    });

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_LANGUAGE (39)
///
/// Learn a language.
/// misc_value = language ID
pub async fn effect_language(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let language_id = input.misc_value as u32;
    
    // TODO: Add language to player's known languages
    
    tracing::debug!(
        "Learn language: caster={:?} language={}",
        input.caster_guid, language_id
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SPAWN (46)
///
/// Spawn animation effect.
pub async fn effect_spawn(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // This is typically a visual effect only
    // Used for spawn-in animations
    
    tracing::debug!(
        "Spawn effect: caster={:?}",
        input.caster_guid
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_CREATE_HOUSE (81)
///
/// Create guild housing (TEST - unused).
pub async fn effect_create_house(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // This was a test effect for guild housing
    // Not used in production
    
    tracing::debug!(
        "Create house (TEST): caster={:?}",
        input.caster_guid
    );
    
    Ok(EffectResult::empty())
}


/// SPELL_EFFECT_LEARN_SPELL (36)
///
/// Teaches the caster a new spell.
pub async fn effect_learn_spell(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains the spell ID to learn
    let spell_to_learn = input.misc_value as u32;

    if spell_to_learn == 0 {
        return Ok(EffectResult::empty());
    }

    // Use the spell learning system
    // TODO: Need access to broadcast_mgr here - for now just log it
    tracing::info!(
        "Learn spell effect: caster {} learning spell {}",
        input.caster_guid, spell_to_learn
    );

    // Mark spell as learned in player state
    world.systems.player.manager().with_player_mut(input.caster_guid, |player| {
        player.spells.learn_spell(spell_to_learn);
    });

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_TRIGGER_SPELL (64)
///
/// Triggers another spell. The triggered spell ID comes from effect_trigger_spell
/// in the spell entry, NOT from misc_value.
pub async fn effect_trigger_spell(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Get triggered spell ID from spell entry's effect_trigger_spell field
    let triggered_spell = {
        let spell_entry = world.managers.spell_mgr.get(input.spell_id);
        spell_entry
            .map(|s| s.effect_trigger_spell[input.effect_index as usize])
            .unwrap_or(0)
    };

    // Fallback to misc_value if effect_trigger_spell is 0
    let triggered_spell = if triggered_spell == 0 {
        input.misc_value as u32
    } else {
        triggered_spell
    };

    if triggered_spell == 0 {
        return Ok(EffectResult::empty());
    }

    tracing::debug!(
        "Trigger spell effect: caster {} triggering spell {} (from spell {})",
        input.caster_guid, triggered_spell, input.spell_id
    );

    // Cast the triggered spell
    world.systems.spells.cast_spell(
        input.caster_guid,
        triggered_spell,
        input.target_guid,
        true, // is_triggered
        world,
    ).await?;

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_INTERRUPT_CAST (68)
///
/// Interrupts the target's spell cast.
pub async fn effect_interrupt_cast(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // misc_value contains the lockout school (if any)
    let lockout_school = input.misc_value as u8;

    // base_value contains the lockout duration in milliseconds
    let lockout_duration_ms = input.base_value.max(0) as u32;

    tracing::debug!(
        "Interrupt cast effect: caster {} interrupting target {} (school: {}, duration: {}ms)",
        input.caster_guid, target_guid, lockout_school, lockout_duration_ms
    );

    // TODO: Interrupt the target's cast via SpellSystem
    // This would typically be:
    // world.systems.spells.interrupt_cast(
    //     target_guid,
    //     input.caster_guid,
    //     lockout_duration_ms,
    //     world
    // ).await?;

    // Apply school lockout
    if lockout_duration_ms > 0 && lockout_school < 7 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        world.systems.player.manager().with_player_mut(target_guid, |player| {
            player.spells.apply_school_lockout(lockout_school, lockout_duration_ms, now);
        });
    }

    Ok(EffectResult::empty())
}
