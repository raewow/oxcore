//! Dispel Effects
//!
//! Handles dispel magic and spell steal.

use super::{EffectInput, EffectResult};
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_DISPEL (38)
///
/// Dispels magic effects from the target.
/// Used by Dispel Magic, Purge, etc.
pub async fn effect_dispel(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // TODO: Implement dispel logic

    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => input.caster_guid,
    };

    // misc_value contains the dispel type (Magic, Curse, Disease, Poison)
    let _dispel_type = input.misc_value;

    // base_value contains the number of effects to dispel
    let _dispel_count = input.base_value.max(1) as u32;

    tracing::debug!(
        "Dispel effect: caster {} dispelling target {} (type: {}, count: {})",
        input.caster_guid, target_guid, _dispel_type, _dispel_count
    );

    // TODO:
    // 1. Get target's active auras
    // 2. Filter by dispel type
    // 3. Randomly select N auras to dispel
    // 4. Remove those auras

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DISPEL_MECHANIC (108)
///
/// Dispel effects by mechanic type (stun, root, etc.).
/// misc_value = mechanic type to dispel
pub async fn effect_dispel_mechanic(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => input.caster_guid,
    };

    // misc_value contains the mechanic type to dispel
    let mechanic_type = input.misc_value;

    // base_value contains the number of effects to dispel
    let dispel_count = input.base_value.max(1) as u32;

    tracing::debug!(
        "Dispel mechanic effect: caster {} dispelling target {} (mechanic: {}, count: {})",
        input.caster_guid, target_guid, mechanic_type, dispel_count
    );

    // TODO:
    // 1. Get target's active auras
    // 2. Filter by mechanic type
    // 3. Remove N auras with that mechanic

    Ok(EffectResult::empty())
}
