//! Aura Application Effects
//!
use super::super::target_info::TargetInfo;
/// Applies buff/debuff auras and area auras to targets.
/// This is the bridge between the spell system and the aura system.
use super::{EffectInput, EffectResult};
use crate::dbc::structures::SpellEntry;
use crate::game::player::auras::AuraFlags;
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;

/// SPELL_AURA_ADD_TARGET_TRIGGER (109) — aura triggers a spell when the caster's spell hits a target.
const SPELL_AURA_ADD_TARGET_TRIGGER: u32 = 109;
/// SPELL_ATTR_EX4_CLASS_TRIGGER_ONLY_ON_TARGET (bit 1) — only trigger if target matches caster's current target.
const SPELL_ATTR_EX4_CLASS_TRIGGER_ONLY_ON_TARGET: u32 = 0x0000_0002;

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

/// Check whether an ADD_TARGET_TRIGGER aura's spell entry affects the cast spell.
/// Faithful port of MaNGOS `IsAffectedOnSpell` used in `HandleAddTargetTriggerAuras`.
fn is_affected_on_spell(aura_spell_entry: &SpellEntry, cast_spell_entry: &SpellEntry) -> bool {
    aura_spell_entry.spell_family_name == 0
        || (aura_spell_entry.spell_family_name == cast_spell_entry.spell_family_name
            && (aura_spell_entry.spell_family_flags == 0
                || (aura_spell_entry.spell_family_flags & cast_spell_entry.spell_family_flags)
                    != 0))
}

/// Handle SPELL_AURA_ADD_TARGET_TRIGGER auras on the caster.
///
/// Iterates the caster's `SPELL_AURA_ADD_TARGET_TRIGGER` auras; for each aura that
/// affects the cast spell, iterates the hit targets and conditionally casts the
/// triggered spell. Faithful Rust port of MaNGOS `Spell::HandleAddTargetTriggerAuras`.
pub async fn handle_add_target_trigger_auras(
    caster_guid: ObjectGuid,
    spell_entry: &SpellEntry,
    targets: &[TargetInfo],
    world: &World,
) -> Result<()> {
    let trigger_auras = world
        .systems
        .player
        .manager()
        .with_player(caster_guid, |player| {
            player
                .auras
                .container
                .get_auras_by_type(SPELL_AURA_ADD_TARGET_TRIGGER)
                .into_iter()
                .map(|a| (a.spell_id, a.effect_index, a.current_value()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if trigger_auras.is_empty() {
        return Ok(());
    }

    let caster_target = world
        .systems
        .player
        .manager()
        .with_player(caster_guid, |p| p.combat.attack_target)
        .flatten();

    for (aura_spell_id, aura_effect_idx, aura_value) in &trigger_auras {
        let aura_spell_entry = match world.managers.spell_mgr.get(*aura_spell_id) {
            Some(e) => e,
            None => continue,
        };

        if !is_affected_on_spell(&aura_spell_entry, spell_entry) {
            continue;
        }

        let trigger_spell_id = aura_spell_entry.effect_trigger_spell[*aura_effect_idx as usize];
        if trigger_spell_id == 0 {
            continue;
        }

        let _trigger_spell_entry = match world.managers.spell_mgr.get(trigger_spell_id) {
            Some(e) => e,
            None => continue,
        };

        for target_info in targets {
            let target_guid = target_info.target_guid;

            let is_hit = matches!(
                target_info.miss_condition,
                super::super::hit::SpellHitOutcome::Hit
            );
            let is_reflect_to_caster = matches!(
                target_info.miss_condition,
                super::super::hit::SpellHitOutcome::Reflect
            );

            if !is_hit && !is_reflect_to_caster {
                continue;
            }

            let target = if is_reflect_to_caster {
                caster_guid
            } else if target_guid == caster_guid {
                caster_guid
            } else {
                target_guid
            };

            if spell_entry.attributes_ex4 & SPELL_ATTR_EX4_CLASS_TRIGGER_ONLY_ON_TARGET != 0 {
                if let Some(ct) = caster_target {
                    if target != ct {
                        continue;
                    }
                }
            }

            let chance = *aura_value;
            if chance > 0 && rand::thread_rng().gen_range(0..100) < chance {
                world
                    .systems
                    .spells
                    .trigger_procced_spell(caster_guid, Some(target), trigger_spell_id, 0, world)
                    .await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use oxcore_shared::database::Databases;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn make_spell_entry(id: u32, family: u32, flags: u64) -> SpellEntry {
        SpellEntry {
            id,
            name: String::new(),
            rank_text: String::new(),
            school: 0,
            category: 0,
            dispel: 0,
            mechanic: 0,
            attributes: 0,
            attributes_ex: 0,
            attributes_ex2: 0,
            attributes_ex3: 0,
            attributes_ex4: 0,
            stances: 0,
            stances_not: 0,
            targets: 0,
            target_creature_type: 0,
            requires_spell_focus: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            casting_time_index: 0,
            recovery_time: 0,
            category_recovery_time: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: 0,
            channel_interrupt_flags: 0,
            proc_flags: 0,
            proc_chance: 0,
            proc_charges: 0,
            max_level: 0,
            base_level: 0,
            spell_level: 0,
            duration_index: 0,
            power_type: 0,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            mana_per_second_per_level: 0,
            range_index: 0,
            speed: 0.0,
            stack_amount: 0,
            totem: [0; 2],
            reagent: [0; 8],
            reagent_count: [0; 8],
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            effect: [0; 3],
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [0; 3],
            effect_bonus_coefficient: [0.0; 3],
            effect_mechanic: [0; 3],
            effect_implicit_target_a: [0; 3],
            effect_implicit_target_b: [0; 3],
            effect_radius_index: [0; 3],
            effect_apply_aura_name: [0; 3],
            effect_amplitude: [0; 3],
            effect_multiple_value: [0.0; 3],
            effect_chain_target: [0; 3],
            effect_item_type: [0; 3],
            effect_misc_value: [0; 3],
            effect_trigger_spell: [0; 3],
            effect_points_per_combo_point: [0.0; 3],
            spell_visual: 0,
            spell_icon_id: 0,
            active_icon_id: 0,
            spell_priority: 0,
            min_target_level: 0,
            mana_cost_percentage: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            max_target_level: 0,
            spell_family_name: family,
            spell_family_flags: flags,
            max_affected_targets: 0,
            dmg_class: 0,
            prevention_type: 0,
            custom: 0,
            internal: 0,
            allowed_target_mask: 0,
            script_id: 0,
            dmg_multiplier: [0.0; 3],
        }
    }

    #[test]
    fn affected_when_aura_family_is_zero() {
        let aura_entry = make_spell_entry(1, 0, 0);
        let cast_entry = make_spell_entry(2, 8, 0x100);
        assert!(is_affected_on_spell(&aura_entry, &cast_entry));
    }

    #[test]
    fn not_affected_when_families_differ_and_aura_family_nonzero() {
        let aura_entry = make_spell_entry(1, 5, 0);
        let cast_entry = make_spell_entry(2, 8, 0);
        assert!(!is_affected_on_spell(&aura_entry, &cast_entry));
    }

    #[test]
    fn affected_when_families_match_and_aura_flags_zero() {
        let aura_entry = make_spell_entry(1, 8, 0);
        let cast_entry = make_spell_entry(2, 8, 0x100);
        assert!(is_affected_on_spell(&aura_entry, &cast_entry));
    }

    #[test]
    fn affected_when_families_and_flags_overlap() {
        let aura_entry = make_spell_entry(1, 8, 0x100);
        let cast_entry = make_spell_entry(2, 8, 0x100);
        assert!(is_affected_on_spell(&aura_entry, &cast_entry));
    }

    #[test]
    fn not_affected_when_flags_dont_overlap() {
        let aura_entry = make_spell_entry(1, 8, 0x200);
        let cast_entry = make_spell_entry(2, 8, 0x100);
        assert!(!is_affected_on_spell(&aura_entry, &cast_entry));
    }

    #[test]
    fn not_affected_when_family_name_differs() {
        let aura_entry = make_spell_entry(1, 3, 0x100);
        let cast_entry = make_spell_entry(2, 8, 0x100);
        assert!(!is_affected_on_spell(&aura_entry, &cast_entry));
    }

    #[tokio::test]
    async fn no_trigger_auras_returns_ok() {
        let world = test_world();
        let cast_entry = make_spell_entry(100, 8, 0);
        let targets = vec![TargetInfo::new(ObjectGuid::new_player(1), 0b001)];
        let result = handle_add_target_trigger_auras(
            ObjectGuid::new_player(10),
            &cast_entry,
            &targets,
            &world,
        )
        .await;
        assert!(result.is_ok());
    }
}
