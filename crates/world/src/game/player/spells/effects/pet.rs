//! Pet Spell Effects
//!
//! Handles all pet-related spell effects.

use super::{EffectInput, EffectResult};
use crate::World;
use anyhow::Result;

/// SPELL_EFFECT_LEARN_PET_SPELL (57)
///
/// Teach a spell to the caster's pet.
pub async fn effect_learn_pet_spell(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let spell_to_learn = input.misc_value as u32;

    // TODO: Get the active pet and teach spell

    tracing::debug!(
        "Learn pet spell: caster={:?} spell={}",
        input.caster_guid,
        spell_to_learn
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DISMISS_PET (102)
///
/// Dismiss the caster's pet.
pub async fn effect_dismiss_pet(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // A tamed pet stays with the hunter, dismissed until called; a dead one stays put.
    let pet = &world.systems.pet;
    if pet.out_pet(input.caster_guid).is_some() {
        pet.dismiss_to_slot(
            input.caster_guid,
            crate::game::pet::pet_slot::NOT_IN_SLOT,
            world,
        );
    } else if pet.active_pet_alive(input.caster_guid) {
        pet.dismiss(input.caster_guid, world);
    }

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_DESTROY_ALL_TOTEMS (110)
///
/// Destroy all of the caster's active totems.
pub async fn effect_destroy_all_totems(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // TODO: Destroy all totems for this player

    tracing::debug!("Destroy all totems: caster={:?}", input.caster_guid);

    Ok(EffectResult::empty())
}
