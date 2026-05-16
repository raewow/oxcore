//! Aura Application Effects
//!
/// Applies buff/debuff auras and area auras to targets.
/// This is the bridge between the spell system and the aura system.
use super::{EffectInput, EffectResult};
use crate::shared::protocol::ObjectGuid;
use crate::world::game::player::auras::AuraFlags;
use crate::world::World;
use anyhow::Result;

/// Area aura target types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaAuraTargetType {
    Party,
    Pet,
    Friend,
    Enemy,
    Raid,
}

/// SPELL_EFFECT_APPLY_AURA (6)
///
/// Applies a buff/debuff aura to the target.
/// This is the bridge between the spell system and the aura system.
///
/// The aura type, duration, and values come from the spell DBC entry.
pub async fn effect_apply_aura(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => {
            // Self-target
            input.caster_guid
        }
    };

    // Read from spell entry
    let spell_entry = match world.managers.spell_mgr.get(input.spell_id) {
        Some(entry) => entry,
        None => {
            tracing::warn!("Spell {} not found for aura application", input.spell_id);
            return Ok(EffectResult::empty());
        }
    };

    let effect_idx = input.effect_index as usize;
    let aura_type = spell_entry.effect_apply_aura_name[effect_idx];
    let periodic_interval_ms = spell_entry.effect_amplitude[effect_idx];
    let max_stacks = spell_entry.stack_amount.max(1) as u8;
    let max_charges = spell_entry.proc_charges as u8;

    // Get duration from Duration.dbc
    let duration_ms = if spell_entry.duration_index > 0 {
        world
            .dbc
            .read()
            .get_spell_duration(spell_entry.duration_index)
            .map(|entry| entry.duration as u32)
    } else {
        None
    };

    tracing::info!(
        "[AURA] effect_apply_aura: spell={} effect_idx={} aura_type={} base_value={} \
         periodic_interval={}ms duration={:?}ms duration_index={} effect_type={} \
         misc_value={} attributes=0x{:08X}",
        input.spell_id,
        effect_idx,
        aura_type,
        input.base_value,
        periodic_interval_ms,
        duration_ms,
        spell_entry.duration_index,
        spell_entry.effect[effect_idx],
        input.misc_value,
        spell_entry.attributes,
    );

    // Determine if positive or negative based on attributes
    // Most buffs are positive (food, drink, stat buffs). A spell is negative if it has
    // SPELL_ATTR_EX_NEGATIVE (0x80000000 in attributes_ex) set.
    let is_positive = (spell_entry.attributes_ex & 0x80000000) == 0;
    let flags = AuraFlags {
        is_positive,
        is_negative: !is_positive,
        is_passive: false,
        can_be_cancelled: is_positive, // Only positive auras can be cancelled
        is_hidden: false,
        is_permanent: duration_ms.is_none(),
    };

    // Delegate to AuraSystem
    world
        .systems
        .auras
        .apply_aura(
            target_guid,
            input.caster_guid,
            input.spell_id,
            input.effect_index,
            aura_type,
            input.misc_value,
            input.base_value,
            duration_ms,
            periodic_interval_ms,
            max_stacks,
            max_charges,
            flags,
            world,
        )
        .await?;

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PERSISTENT_AREA_AURA (27)
///
/// Creates a persistent ground effect (Consecration, Blizzard, etc.).
/// Spawns a DynamicObject that periodically applies auras to targets in range.
pub async fn effect_persistent_area_aura(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    // Get caster (or use target location if no caster)
    let caster_guid = input.caster_guid;

    // TODO: Get target location from spell targets
    // For now, use caster's position
    let target_location = world
        .systems
        .player
        .manager()
        .with_player(caster_guid, |player| player.movement.position.clone());

    let Some(_position) = target_location else {
        return Ok(EffectResult::empty());
    };

    // TODO: Calculate radius from spell radius entry
    let radius = 10.0f32; // Placeholder

    // TODO: Get duration from spell entry
    let duration_ms = Some(30_000u32); // Placeholder: 30 seconds

    // TODO: Create DynamicObject at target location
    // DynamicObject will periodically apply aura to targets in radius

    tracing::debug!(
        "Persistent area aura: spell_id={} radius={} duration={:?}",
        input.spell_id,
        radius,
        duration_ms
    );

    // TODO: Implement DynamicObject creation and management
    // For now, just apply the aura to the caster as a placeholder
    effect_apply_aura(input, world).await
}

/// SPELL_EFFECT_APPLY_AREA_AURA_PARTY (35)
///
/// Applies an aura to all party members within range.
pub async fn effect_apply_area_aura_party(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    apply_area_aura(input, world, AreaAuraTargetType::Party).await
}

/// SPELL_EFFECT_APPLY_AREA_AURA_PET (119)
///
/// Applies an aura to the caster's pet.
pub async fn effect_apply_area_aura_pet(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    apply_area_aura(input, world, AreaAuraTargetType::Pet).await
}

/// SPELL_EFFECT_APPLY_AREA_AURA_FRIEND (128)
///
/// Applies an aura to all friendly units within range.
pub async fn effect_apply_area_aura_friend(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    apply_area_aura(input, world, AreaAuraTargetType::Friend).await
}

/// SPELL_EFFECT_APPLY_AREA_AURA_ENEMY (129)
///
/// Applies an aura to all enemy units within range.
pub async fn effect_apply_area_aura_enemy(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    apply_area_aura(input, world, AreaAuraTargetType::Enemy).await
}

/// SPELL_EFFECT_APPLY_AREA_AURA_RAID (132)
///
/// Applies an aura to all raid members within range.
pub async fn effect_apply_area_aura_raid(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    apply_area_aura(input, world, AreaAuraTargetType::Raid).await
}

/// Generic area aura application
///
/// Creates an AreaAura that stays on the caster/target and periodically
/// applies the aura to valid targets within range.
async fn apply_area_aura(
    input: &EffectInput,
    world: &World,
    target_type: AreaAuraTargetType,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => input.caster_guid,
    };

    // TODO: Read from spell DBC entry
    let aura_type = 0u32; // Placeholder
    let duration_ms = Some(30_000u32);
    let periodic_interval_ms = 0u32;
    let max_stacks = 1u8;
    let max_charges = 0u8;

    // Area auras are typically positive buffs
    let flags = AuraFlags {
        is_positive: true,
        is_negative: false,
        is_passive: false,
        can_be_cancelled: true,
        is_hidden: false,
        is_permanent: duration_ms.is_none(),
    };

    // TODO: Create AreaAura instead of regular Aura
    // AreaAura handles target selection based on target_type
    // and periodically checks for valid targets in range

    tracing::debug!(
        "Area aura: spell_id={} target_type={:?} on {:?}",
        input.spell_id,
        target_type,
        target_guid
    );

    // For now, delegate to regular aura application
    // TODO: Implement proper AreaAura with target selection logic
    world
        .systems
        .auras
        .apply_aura(
            target_guid,
            input.caster_guid,
            input.spell_id,
            input.effect_index,
            aura_type,
            input.misc_value,
            input.base_value,
            duration_ms,
            periodic_interval_ms,
            max_stacks,
            max_charges,
            flags,
            world,
        )
        .await?;

    Ok(EffectResult::empty())
}
