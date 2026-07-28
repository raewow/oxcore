//! Spell Healing Effects
//!
//! Handles direct heals and full health heals.
//! Formulas ported from old system.

use super::{EffectInput, EffectResult};
use crate::World;
use anyhow::Result;
use oxcore_shared::messages::spells::ExecuteLogData;
use oxcore_shared::protocol::{Opcode, WorldPacket};

/// Apply a critical-heal bonus to a heal amount (faithful `SpellCaster::SpellCriticalHealingBonus`).
///
/// Melee/ranged-class heals crit for +100% (double); all other classes crit for +50%
/// (the `damage / 2` integer halving the C++ uses). The `MOD_CRIT_PERCENT_VERSUS`
/// aura multiplier is deferred until that aura type is ported.
fn spell_critical_healing_bonus(dmg_class: u32, heal: u32) -> u32 {
    const SPELL_DAMAGE_CLASS_MELEE: u32 = 2;
    const SPELL_DAMAGE_CLASS_RANGED: u32 = 3;

    let crit_bonus = match dmg_class {
        SPELL_DAMAGE_CLASS_MELEE | SPELL_DAMAGE_CLASS_RANGED => heal,
        _ => heal / 2,
    };

    heal + crit_bonus
}

/// SPELL_EFFECT_HEAL (10)
///
/// Direct heal (Flash Heal, Healing Touch, etc.)
///
/// Calculation:
/// 1. Base value with dice roll + level scaling
/// 2. + healing_power * coefficient (from DBC or cast_time / 3500)
/// 3. Roll crit (spell_crit_pct)
/// 4. If crit: apply SpellCriticalHealingBonus (+50%, or +100% for melee/ranged-class heals)
pub async fn effect_heal(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // Get caster stats for healing power and crit
    let caster_stats = world
        .systems
        .player
        .manager()
        .with_player(input.caster_guid, |player| {
            (
                player.stats.healing_power,
                player.stats.spell_crit_pct,
                player.level,
            )
        });

    let caster_level = caster_stats.as_ref().map(|s| s.2).unwrap_or(1);

    // Step 1: Base heal with dice roll + level scaling
    let base_heal = input.calculate_base_value(caster_level).max(0) as f32;
    let mut final_heal = base_heal;

    // Step 2: Add healing power bonus with coefficient
    if let Some((healing_power, _, _)) = caster_stats {
        let coefficient = input.get_spell_coefficient();
        let mut heal_bonus = healing_power as f32 * coefficient;
        // Downranking penalty applies only to the computed (non-DBC) coefficient.
        if input.spell_coefficient <= 0.01 {
            let spell_level = world
                .managers
                .spell_mgr
                .get(input.spell_id)
                .map(|e| e.spell_level)
                .unwrap_or(0);
            heal_bonus *= super::calculate_level_penalty(spell_level);
        }
        final_heal += heal_bonus;
    }

    // Step 3: Roll for crit (healing crit = 150% heal)
    let is_crit = if let Some((_, crit_pct, _)) = caster_stats {
        let crit_roll = rand::random::<f32>() * 100.0;
        crit_roll < crit_pct
    } else {
        false
    };

    let mut heal_amount = final_heal as u32;
    if is_crit {
        let dmg_class = world
            .managers
            .spell_mgr
            .get(input.spell_id)
            .map(|e| e.dmg_class)
            .unwrap_or(0);
        heal_amount = spell_critical_healing_bonus(dmg_class, heal_amount);
    }

    // Apply healing via modify_health (clamps to max_health, returns actual gain).
    let healed = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let before = player.stats.health;
            let gain = player.stats.modify_health(heal_amount as i32);
            tracing::debug!(
                "Spell heal: {} healed for {} (crit: {}), health: {} -> {}",
                player.name,
                gain,
                is_crit,
                before,
                player.stats.health
            );
            gain as u32
        })
        .unwrap_or(0);

    // Send SMSG_SPELLHEALLOG (P5)
    let overheal = heal_amount.saturating_sub(healed);
    send_spell_heal_log(
        input.caster_guid,
        target_guid,
        input.spell_id,
        healed,
        overheal,
        is_crit,
        world,
    );

    // Fire proc checks for healing
    if healed > 0 {
        use crate::game::player::auras::proc::{proc_flags, proc_flags_ex};
        let proc_ex = if is_crit {
            proc_flags_ex::CRITICAL_HIT
        } else {
            proc_flags_ex::NORMAL_HIT
        };
        // Caster: cast a helpful (healing) spell.
        let _ = world
            .systems
            .auras
            .check_procs(
                input.caster_guid,
                proc_flags::DEAL_HELPFUL_SPELL,
                proc_ex,
                Some(input.spell_id),
                healed,
                world,
            )
            .await;
        // Target: received a helpful (healing) spell.
        if target_guid.is_player() {
            let _ = world
                .systems
                .auras
                .check_procs(
                    target_guid,
                    proc_flags::TAKE_HELPFUL_SPELL,
                    proc_ex,
                    Some(input.spell_id),
                    healed,
                    world,
                )
                .await;
        }
    }

    let mut result = EffectResult::with_healing(healed);
    result.execute_log = Some(ExecuteLogData::Heal {
        amount: healed,
        critical: is_crit,
    });
    Ok(result)
}

/// SPELL_EFFECT_HEAL_MAX_HEALTH (67)
///
/// Heals target to full health (Lay on Hands).
pub async fn effect_heal_max_health(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let healed = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let current = player.stats.health;
            let max = player.stats.max_health;
            let heal_amount = max.saturating_sub(current);
            player.stats.health = max;

            tracing::debug!(
                "Lay on Hands: {} healed to full, health: {} -> {}",
                player.name,
                current,
                max
            );

            heal_amount
        })
        .unwrap_or(0);

    Ok(EffectResult::with_healing(healed))
}

/// SPELL_EFFECT_HEAL_MECHANICAL (75)
///
/// Heals mechanical units (repair abilities).
/// Target creature type is checked by targetCreatureType field.
pub async fn effect_heal_mechanical(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let base_heal = input.base_value.max(0) as u32;

    // TODO: Check if target is mechanical creature type

    let healed = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let max_heal = player.stats.max_health.saturating_sub(player.stats.health);
            let actual_heal = base_heal.min(max_heal);
            player.stats.health += actual_heal;

            tracing::debug!(
                "Mechanical heal: {} healed for {}, health: {} -> {}",
                player.name,
                actual_heal,
                player.stats.health - actual_heal,
                player.stats.health
            );

            actual_heal
        })
        .unwrap_or(0);

    Ok(EffectResult::with_healing(healed))
}

/// SPELL_EFFECT_SPIRIT_HEAL (117)
///
/// Spirit healer resurrection heal.
/// Only works on dead players with "Waiting to Resurrect" aura.
pub async fn effect_spirit_heal(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // Check if target is a player and is dead
    let can_resurrect = world
        .systems
        .player
        .manager()
        .with_player(target_guid, |player| player.stats.health == 0)
        .unwrap_or(false);

    if !can_resurrect {
        return Ok(EffectResult::empty());
    }

    // Resurrect player at full health
    world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            player.stats.health = player.stats.max_health;

            // TODO: Remove "Waiting to Resurrect" aura (spell 2584)
            // TODO: Apply resurrection sickness
            // TODO: Spawn corpse bones
            // TODO: Auto-resummon pet

            tracing::debug!("Spirit heal: {} resurrected at full health", player.name);
        });

    Ok(EffectResult::with_healing(0))
}

/// Build and broadcast SMSG_SPELLHEALLOG packet (P5).
fn send_spell_heal_log(
    caster_guid: oxcore_shared::protocol::ObjectGuid,
    target_guid: oxcore_shared::protocol::ObjectGuid,
    spell_id: u32,
    heal_amount: u32,
    overheal: u32,
    is_crit: bool,
    world: &crate::World,
) {
    let mut packet = WorldPacket::new(Opcode::SMSG_SPELLHEALLOG);
    packet.write_packed_guid_raw(target_guid.raw());
    packet.write_packed_guid_raw(caster_guid.raw());
    packet.write_u32(spell_id);
    packet.write_u32(heal_amount);
    packet.write_u32(overheal);
    packet.write_u8(if is_crit { 1 } else { 0 });
    packet.write_u8(0); // unused

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_packet(caster_guid, &packet, true);
}
