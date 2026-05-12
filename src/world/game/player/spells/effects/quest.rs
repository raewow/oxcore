//! Quest and Reputation Spell Effects
//!
//! Handles quest completion and reputation changes.

use super::{EffectInput, EffectResult};
use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_QUEST_COMPLETE (16)
///
/// Complete a quest for the target.
/// misc_value = quest ID
pub async fn effect_quest_complete(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let quest_id = input.misc_value as u32;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    
    // TODO: Complete the quest
    // Need to use quest system to mark quest as complete and give rewards
    
    tracing::debug!(
        "Quest complete: target={:?} quest={}",
        target_guid, quest_id
    );
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_ADD_HONOR (45)
///
/// Add honor points to the target.
/// base_value = honor points to add
pub async fn effect_add_honor(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let honor_points = input.base_value.max(0) as u32;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    
    // Add honor
    world.systems.player.manager().with_player_mut(target_guid, |player| {
        // TODO: Add honor to player's honor points
        // This would typically be stored in player data
        tracing::debug!(
            "Add honor: target={:?} points={}",
            target_guid, honor_points
        );
    });
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_REPUTATION (103)
///
/// Modify reputation with a faction.
/// misc_value = faction ID
/// base_value = reputation change (positive or negative)
pub async fn effect_reputation(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let faction_id = input.misc_value as u32;
    let reputation_change = input.base_value;
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    
    // Modify reputation
    // modify_reputation takes: player_guid, faction_id, standing_change, world
    world.systems.reputation.modify_reputation(
        target_guid,
        faction_id,
        reputation_change,
        world,
    )?;
    
    tracing::debug!(
        "Reputation change: target={:?} faction={} change={}",
        target_guid, faction_id, reputation_change
    );
    
    Ok(EffectResult::empty())
}
