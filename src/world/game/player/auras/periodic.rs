//! Periodic effect handlers for aura ticks

use crate::shared::messages::auras::SmsgPeriodicAuraLog;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::player::auras::effects::*;
use crate::world::game::player::auras::system::AuraTickSnapshot;
use crate::world::World;
use std::sync::Arc;

use anyhow::Result;

/// Dispatch a periodic tick based on aura type.
pub async fn dispatch_periodic_tick(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
    broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    match snapshot.aura_type {
        AURA_OBS_MOD_HEALTH => {
            handle_obs_mod_health(target_guid, snapshot, world, broadcast_mgr)
        }
        AURA_OBS_MOD_MANA => {
            handle_obs_mod_mana(target_guid, snapshot, world)
        }
        AURA_PERIODIC_DAMAGE => {
            handle_periodic_damage(target_guid, snapshot, world, broadcast_mgr)
        }
        AURA_PERIODIC_HEAL => {
            handle_periodic_heal(target_guid, snapshot, world, broadcast_mgr)
        }
        AURA_PERIODIC_ENERGIZE => handle_periodic_energize(target_guid, snapshot, world),
        AURA_PERIODIC_LEECH => handle_periodic_leech(target_guid, snapshot, world),
        AURA_PERIODIC_MANA_LEECH => handle_periodic_mana_leech(target_guid, snapshot, world).await,
        AURA_PERIODIC_TRIGGER_SPELL => {
            handle_periodic_trigger_spell(target_guid, snapshot, world).await
        }
        AURA_PERIODIC_DAMAGE_PERCENT => {
            handle_periodic_damage_percent(target_guid, snapshot, world).await
        }
        _ => {
            tracing::debug!(
                "Unhandled periodic aura type {} for spell {}",
                snapshot.aura_type,
                snapshot.spell_id
            );
            Ok(())
        }
    }
}

/// Handle periodic damage (DoT).
///
/// Examples: Corruption (18), Shadow Word: Pain (589), Immolate (348)
/// Each tick deals base_value damage (already scaled at apply time).
fn handle_periodic_damage(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
    broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    let damage = snapshot.current_value.max(0) as u32;
    if damage == 0 {
        return Ok(());
    }

    // Apply damage to target (player or creature)
    if target_guid.is_player() {
        let died = world.systems.player.manager().with_player_mut(target_guid, |player| {
            let current_health = player.stats.health;
            let new_health = current_health.saturating_sub(damage);
            player.stats.health = new_health;
            player.stats.dirty = true;

            tracing::debug!(
                "Periodic damage: {} took {} damage from spell {}, health: {} -> {}",
                player.name, damage, snapshot.spell_id, current_health, new_health
            );

            new_health == 0 && current_health > 0
        }).unwrap_or(false);

        if died {
            if let Err(e) = world.systems.death.on_killed(
                target_guid, Some(snapshot.caster_guid), Some(snapshot.spell_id), world,
            ) {
                tracing::error!("Failed to handle player death from DoT: {}", e);
            }
        }
    } else if target_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let result = world.managers.creature_mgr.apply_damage(
            target_guid, damage, snapshot.caster_guid, timestamp,
        );

        if let Some((_actual_damage, is_dead)) = result {
            if is_dead {
                tracing::info!("Creature {:?} killed by periodic spell {}", target_guid, snapshot.spell_id);
            }
        }
    }

    // Send SMSG_PERIODICAURALOG to nearby players
    let msg = SmsgPeriodicAuraLog {
        target_guid,
        caster_guid: snapshot.caster_guid,
        spell_id: snapshot.spell_id,
        aura_type: snapshot.aura_type,
        damage,
        school: 0, // TODO: Get from spell data
    };
    broadcast_mgr
        .broadcast_nearby(target_guid, &msg.to_world_packet(), true)
        ;

    Ok(())
}

/// Handle periodic heal (HoT).
///
/// Examples: Renew (139), Rejuvenation (774), Regrowth HoT component
fn handle_periodic_heal(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
    broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    let heal_amount = snapshot.current_value.max(0) as u32;

    // Apply healing
    let _actual_heal = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let max_health = player.stats.max_health;
            let current_health = player.stats.health;
            let actual_heal = heal_amount.min(max_health.saturating_sub(current_health));
            player.stats.health += actual_heal;
            actual_heal
        })
        .unwrap_or(0);

    // Send SMSG_PERIODICAURALOG
    let msg = SmsgPeriodicAuraLog {
        target_guid,
        caster_guid: snapshot.caster_guid,
        spell_id: snapshot.spell_id,
        aura_type: snapshot.aura_type,
        damage: heal_amount, // "damage" field used for healing amount too
        school: 0,
    };
    broadcast_mgr
        .broadcast_nearby(target_guid, &msg.to_world_packet(), true)
        ;

    Ok(())
}

/// Handle periodic energize (power restore).
///
/// Examples: Innervate (29166), Evocation (12051)
/// misc_value = power type (0=mana, 1=rage, 3=energy)
fn handle_periodic_energize(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
) -> Result<()> {
    let power_amount = snapshot.current_value.max(0) as u32;
    let power_type = snapshot.misc_value as u8; // 0=Mana, 1=Rage, etc.

    // Delegate to PowerSystem
    if let Some(pt) = super::super::power::state::PowerType::from_u8(power_type) {
        world
            .systems
            .power
            .restore_power(target_guid, pt, power_amount, world)?;
    }

    Ok(())
}

/// Handle periodic leech (drain life).
///
/// Examples: Drain Life (689)
/// Damages target and heals caster for the same amount.
fn handle_periodic_leech(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
) -> Result<()> {
    let leech_amount = snapshot.current_value.max(0) as u32;
    if leech_amount == 0 {
        return Ok(());
    }

    // Damage target (player or creature)
    if target_guid.is_player() {
        let died = world.systems.player.manager().with_player_mut(target_guid, |player| {
            let current = player.stats.health;
            let new_health = current.saturating_sub(leech_amount);
            player.stats.health = new_health;
            player.stats.dirty = true;
            new_health == 0 && current > 0
        }).unwrap_or(false);

        if died {
            let _ = world.systems.death.on_killed(
                target_guid, Some(snapshot.caster_guid), Some(snapshot.spell_id), world,
            );
        }
    } else if target_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let result = world.managers.creature_mgr.apply_damage(
            target_guid, leech_amount, snapshot.caster_guid, timestamp,
        );
        // If creature died from the damage, it will be processed by the
        // main loop's process_deaths() which picks up JustDied creatures.
    }

    // Heal caster
    world
        .systems
        .player
        .manager()
        .with_player_mut(snapshot.caster_guid, |player| {
            let max_health = player.stats.max_health;
            let current_health = player.stats.health;
            let actual_heal = leech_amount.min(max_health.saturating_sub(current_health));
            player.stats.health += actual_heal;
            player.stats.dirty = true;
        });

    Ok(())
}

/// Handle periodic mana leech.
///
/// Examples: Mana Burn (8129) DoT, Drain Mana (5138)
async fn handle_periodic_mana_leech(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
) -> Result<()> {
    let drain_amount = snapshot.current_value.max(0) as u32;

    // Drain mana from target, restore to caster
    // Both operations go through PowerSystem
    world.systems.power.consume_power(
        target_guid,
        super::super::power::state::PowerType::Mana,
        drain_amount,
        world,
    )?;

    world.systems.power.restore_power(
        snapshot.caster_guid,
        super::super::power::state::PowerType::Mana,
        drain_amount,
        world,
    )?;

    Ok(())
}

/// Handle periodic trigger spell.
///
/// Examples: Lightning Shield charges triggering on melee, some trinket effects
async fn handle_periodic_trigger_spell(
    _target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    _world: &World,
) -> Result<()> {
    // The triggered spell ID is stored in the spell's effect_trigger_spell field
    // This requires looking up the original spell entry to find the triggered spell
    // TODO: Pass triggered_spell_id through the snapshot
    tracing::debug!(
        "Periodic trigger spell tick for spell {} on target {:?}",
        snapshot.spell_id,
        _target_guid
    );
    Ok(())
}

/// Handle OBS_MOD_HEALTH (food regen).
///
/// Restores X% of max health per tick. Used by food items.
/// Examples: Conjured Bread, various food items
fn handle_obs_mod_health(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
    broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    let pct = snapshot.current_value.max(0) as u32;

    let heal_amount = world
        .systems
        .player
        .manager()
        .with_player_mut(target_guid, |player| {
            let max_health = player.stats.max_health;
            let current_health = player.stats.health;
            let regen = max_health * pct / 100;
            let actual_heal = regen.min(max_health.saturating_sub(current_health));
            player.stats.health += actual_heal;
            player.stats.dirty = true;
            actual_heal
        })
        .unwrap_or(0);

    if heal_amount > 0 {
        let msg = SmsgPeriodicAuraLog {
            target_guid,
            caster_guid: snapshot.caster_guid,
            spell_id: snapshot.spell_id,
            aura_type: snapshot.aura_type,
            damage: heal_amount,
            school: 0,
        };
        broadcast_mgr.broadcast_nearby(target_guid, &msg.to_world_packet(), true);
    }

    Ok(())
}

/// Handle OBS_MOD_MANA (drink regen).
///
/// Restores X% of max mana per tick. Used by drink items.
/// Examples: Conjured Water, various drink items
fn handle_obs_mod_mana(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
) -> Result<()> {
    let pct = snapshot.current_value.max(0) as u32;

    // Calculate mana to restore as % of max mana
    let mana_amount = world
        .systems
        .player
        .manager()
        .with_player(target_guid, |player| {
            let max_mana = player.power.max[0]; // index 0 = Mana
            max_mana * pct / 100
        })
        .unwrap_or(0);

    if mana_amount > 0 {
        world.systems.power.restore_power(
            target_guid,
            super::super::power::state::PowerType::Mana,
            mana_amount,
            world,
        )?;
    }

    Ok(())
}

/// Handle periodic damage percent.
///
/// Deals X% of max health per tick.
async fn handle_periodic_damage_percent(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
) -> Result<()> {
    let pct = snapshot.current_value.max(0) as f32 / 100.0;

    // Calculate damage from target's max health
    let damage: u32 = if target_guid.is_player() {
        world.systems.player.manager().with_player(target_guid, |player| {
            (player.stats.max_health as f32 * pct) as u32
        }).unwrap_or(0)
    } else if target_guid.is_creature() {
        world.managers.creature_mgr.with_creature(target_guid, |creature| {
            (creature.max_health as f32 * pct) as u32
        }).unwrap_or(0)
    } else {
        0
    };

    if damage > 0 {
        // Apply the damage
        if target_guid.is_player() {
            let died = world.systems.player.manager().with_player_mut(target_guid, |player| {
                let current = player.stats.health;
                let new_health = current.saturating_sub(damage);
                player.stats.health = new_health;
                player.stats.dirty = true;
                new_health == 0 && current > 0
            }).unwrap_or(false);

            if died {
                let _ = world.systems.death.on_killed(
                    target_guid, Some(snapshot.caster_guid), Some(snapshot.spell_id), world,
                );
            }
        } else if target_guid.is_creature() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let result = world.managers.creature_mgr.apply_damage(
                target_guid, damage, snapshot.caster_guid, timestamp,
            );
            if let Some((_actual, is_dead)) = result {
                if is_dead {
                    tracing::info!("Creature {:?} killed by periodic damage percent spell {}", target_guid, snapshot.spell_id);
                }
            }
        }
    }

    Ok(())
}
