//! Profession and Skill Spell Effects
//!
//! Handles profession, skill, and crafting-related effects.

use super::{EffectInput, EffectResult};
use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_SKILL_STEP (44)
///
/// Increase a skill by a specified amount.
/// misc_value = skill ID
/// base_value = amount to increase
pub async fn effect_skill_step(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let skill_id = input.misc_value as u16;
    let skill_increase = input.base_value.max(0) as u16;
    
    // TODO: Implement skill increase
    // Need to get current skill value, add increase, and update
    
    tracing::debug!(
        "Skill step: caster={:?} skill={} increase={}",
        input.caster_guid, skill_id, skill_increase
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_TRADE_SKILL (47)
///
/// Perform a trade skill crafting action.
/// misc_value = recipe ID
pub async fn effect_trade_skill(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let recipe_id = input.misc_value as u32;
    
    // Perform crafting
    // TODO: Implement crafting system
    // This should:
    // 1. Check if player knows the recipe
    // 2. Check if player has required materials
    // 3. Consume materials
    // 4. Create item
    // 5. Give skill up
    
    tracing::debug!(
        "Trade skill: caster={:?} recipe={}",
        input.caster_guid, recipe_id
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PROFICIENCY (60)
///
/// Learn weapon or armor proficiency.
/// misc_value = proficiency mask (bitmask of weapon/armor types)
pub async fn effect_proficiency(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let proficiency_mask = input.misc_value as u32;
    
    // Learn proficiency
    world.systems.player.manager().with_player_mut(input.caster_guid, |player| {
        // Add proficiency to player's known proficiencies
        // This would typically be stored in player data
        tracing::debug!(
            "Learn proficiency: caster={:?} mask={:08x}",
            input.caster_guid, proficiency_mask
        );
    });
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SKILL (118)
///
/// Learn a skill or profession.
/// misc_value = skill ID
/// base_value = initial skill value
pub async fn effect_skill(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let skill_id = input.misc_value as u16;
    let skill_value = input.base_value.max(0) as u16;
    
    // Learn skill at specified value
    // learn_skill takes: player_guid, skill_id, current, max, step, world
    world.systems.skills.learn_skill(
        input.caster_guid,
        skill_id,
        skill_value,      // current
        skill_value + 50, // max (placeholder)
        1,                // step
        world,
    )?;
    
    tracing::debug!(
        "Learn skill: caster={:?} skill={} value={}",
        input.caster_guid, skill_id, skill_value
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SKINNING (95)
///
/// Skin a creature corpse.
pub async fn effect_skinning(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    // Verify target is skinnable (must be a creature)
    if !target_guid.is_creature() {
        return Err(anyhow::anyhow!("Target is not skinnable"));
    }
    
    // TODO: Check if creature is lootable and dead
    // TODO: Generate skinning loot based on creature entry
    // TODO: Give loot to player
    // TODO: Mark creature as skinned
    
    tracing::debug!(
        "Skinning: caster={:?} target={:?}",
        input.caster_guid, target_guid
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SKIN_PLAYER_CORPSE (116)
///
/// Skin a player corpse (remove insignia in battlegrounds).
pub async fn effect_skin_player_corpse(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    // Verify target is a player
    if !target_guid.is_player() {
        return Err(anyhow::anyhow!("Target is not a player"));
    }
    
    // Check if player is dead
    let is_dead = world.systems.player.manager().with_player(target_guid, |player| {
        player.stats.health == 0
    }).unwrap_or(false);
    
    if !is_dead {
        return Err(anyhow::anyhow!("Target is not dead"));
    }
    
    // TODO: Set player as lootable (remove insignia)
    // This allows enemy players to loot the corpse in battlegrounds
    
    tracing::debug!(
        "Skin player corpse: caster={:?} target={:?}",
        input.caster_guid, target_guid
    );
    
    Ok(EffectResult::empty())
}
