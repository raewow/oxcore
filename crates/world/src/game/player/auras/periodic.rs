//! Periodic effect handlers for aura ticks

use crate::game::broadcast_mgr::BroadcastManager;
use crate::game::player::auras::effects::*;
use crate::game::player::auras::system::AuraTickSnapshot;
use crate::World;
use oxcore_shared::messages::auras::SmsgPeriodicAuraLog;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

use anyhow::Result;

/// Recompute an aura's periodic timer against a new duration.
///
/// Called when a channeled aura's duration is refreshed against a dynamic object (or, via
/// `refresh_aura_periodic_timers`, whenever the holder's duration changes) so the
/// next tick lands at the same point within the period rather than resetting to a full period —
/// avoids losing partial ticks. Only meaningful while the aura is actually periodic; callers
/// should skip non-periodic auras (`periodic_interval_ms == 0`) themselves.
///
/// `duration` and `periodic_interval_ms` are both milliseconds. Returns the new periodic-timer
/// value ("ms until next tick" counting down — note `AuraContainer::tick_periodic` counts up
/// instead, so a caller wiring this in against that representation must invert appropriately).
pub fn update_periodic_timer(duration: i32, periodic_interval_ms: i32) -> i32 {
    if periodic_interval_ms <= 0 {
        return duration;
    }
    // Avoid modulo when we don't need it (duration already within one period).
    let mut newtick = if duration > periodic_interval_ms {
        duration % periodic_interval_ms
    } else {
        duration
    };
    // A duration that divides evenly resets to a full period rather than an instant tick.
    if newtick == 0 {
        newtick = periodic_interval_ms;
    }
    newtick
}

/// Dispatch a periodic tick based on aura type.
pub async fn dispatch_periodic_tick(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
    broadcast_mgr: &Arc<BroadcastManager>,
) -> Result<()> {
    match snapshot.aura_type {
        AURA_OBS_MOD_HEALTH => handle_obs_mod_health(target_guid, snapshot, world, broadcast_mgr),
        AURA_OBS_MOD_MANA => handle_obs_mod_mana(target_guid, snapshot, world),
        AURA_PERIODIC_DAMAGE => handle_periodic_damage(target_guid, snapshot, world, broadcast_mgr),
        AURA_PERIODIC_HEAL => handle_periodic_heal(target_guid, snapshot, world, broadcast_mgr),
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
        let died = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let current_health = player.stats.health;
                player.apply_damage(damage);
                let new_health = player.stats.health;

                tracing::debug!(
                    "Periodic damage: {} took {} damage from spell {}, health: {} -> {}",
                    player.name,
                    damage,
                    snapshot.spell_id,
                    current_health,
                    new_health
                );

                new_health == 0 && current_health > 0
            })
            .unwrap_or(false);

        if died {
            if let Err(e) = world.systems.death.on_killed(
                target_guid,
                Some(snapshot.caster_guid),
                Some(snapshot.spell_id),
                world,
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
            target_guid,
            damage,
            snapshot.caster_guid,
            timestamp,
        );

        if let Some((_actual_damage, is_dead)) = result {
            if is_dead {
                tracing::info!(
                    "Creature {:?} killed by periodic spell {}",
                    target_guid,
                    snapshot.spell_id
                );
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
    broadcast_mgr.broadcast_msg_nearby(target_guid, &msg, true);

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
    broadcast_mgr.broadcast_msg_nearby(target_guid, &msg, true);

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
        let died = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let current = player.stats.health;
                let new_health = current.saturating_sub(leech_amount);
                player.stats.health = new_health;
                player.stats.dirty = true;
                new_health == 0 && current > 0
            })
            .unwrap_or(false);

        if died {
            let _ = world.systems.death.on_killed(
                target_guid,
                Some(snapshot.caster_guid),
                Some(snapshot.spell_id),
                world,
            );
        }
    } else if target_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let result = world.managers.creature_mgr.apply_damage(
            target_guid,
            leech_amount,
            snapshot.caster_guid,
            timestamp,
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

/// Handle periodic trigger spell (`SPELL_AURA_PERIODIC_TRIGGER_SPELL`).
///
/// Examples: Lightning Shield charges triggering on melee, some trinket effects.
///
/// Approximation: `AuraTickSnapshot` does not carry the effect index the aura lives on, so the
/// triggered spell id is recovered by scanning the spell's three effect slots for the one whose
/// `effect_apply_aura_name` is `AURA_PERIODIC_TRIGGER_SPELL` (matching the code that assigned
/// this aura type) rather than reading `EffectTriggerSpell[m_effIndex]` directly. This is exact
/// for the overwhelming majority of spells, which only place one periodic-trigger effect per
/// spell; a spell with two such effects at different indices with different trigger spells would
/// pick the first match instead of the aura's own slot.
///
/// The custom per-spell-ID special cases (Firestone Passive weapon enchant, Brood
/// Affliction, Restoration, Frenzied Regeneration, Lightning Shield cleanup, etc.) and the
/// channel-target redirection for channeled spells are not modeled — those need item/weapon
/// state, motion/channel-target lookups, and per-spell-family branching not available from this
/// snapshot; only the generic "cast the configured trigger spell on the aura's target" path is
/// implemented.
async fn handle_periodic_trigger_spell(
    target_guid: ObjectGuid,
    snapshot: &AuraTickSnapshot,
    world: &World,
) -> Result<()> {
    let trigger_spell_id = world
        .managers
        .spell_mgr
        .get(snapshot.spell_id)
        .and_then(|entry| {
            (0..3)
                .find(|&i| entry.effect_apply_aura_name[i] == AURA_PERIODIC_TRIGGER_SPELL)
                .map(|i| entry.effect_trigger_spell[i])
        })
        .unwrap_or(0);

    if trigger_spell_id == 0 {
        tracing::debug!(
            "Periodic trigger spell {} has no configured trigger spell (custom EffectDummy case, not modeled)",
            snapshot.spell_id
        );
        return Ok(());
    }

    if world.managers.spell_mgr.get(trigger_spell_id).is_none() {
        tracing::warn!(
            "Periodic trigger spell {} references unknown trigger spell {}",
            snapshot.spell_id,
            trigger_spell_id
        );
        return Ok(());
    }

    world
        .systems
        .spells
        .cast_custom_spell(
            snapshot.caster_guid,
            trigger_spell_id,
            Some(target_guid),
            [None, None, None],
            world,
        )
        .await?;

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
        broadcast_mgr.broadcast_msg_nearby(target_guid, &msg, true);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::player::player::Player;
    use crate::World;
    use oxcore_shared::database::Databases;
    use oxcore_shared::protocol::ObjectGuid;
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

    fn add_player_with_health(world: &World, guid: ObjectGuid, health: u32, max_health: u32) {
        let player = Player::new(guid, format!("P{}", guid.counter()), 0, 0, 0, 60, 1, 1, 0);
        world.managers.player_mgr.add_player(player, guid.counter());
        world.systems.player.manager().with_player_mut(guid, |p| {
            p.stats.health = health;
            p.stats.max_health = max_health;
        });
    }

    fn health_of(world: &World, guid: ObjectGuid) -> u32 {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |p| p.stats.health)
            .unwrap_or(0)
    }

    fn snapshot(caster: ObjectGuid, aura_type: u32, current_value: i32) -> AuraTickSnapshot {
        AuraTickSnapshot {
            spell_id: 1000,
            caster_guid: caster,
            aura_type,
            current_value,
            misc_value: 0,
        }
    }

    // ── update_periodic_timer ────────────────────────────────────────────────

    #[test]
    fn update_periodic_timer_non_positive_interval_returns_duration() {
        assert_eq!(update_periodic_timer(5000, 0), 5000);
        assert_eq!(update_periodic_timer(5000, -1), 5000);
    }

    #[test]
    fn update_periodic_timer_wraps_into_one_period() {
        // 7000 ms into a 3000 ms period → 1000 ms until the next tick.
        assert_eq!(update_periodic_timer(7000, 3000), 1000);
    }

    #[test]
    fn update_periodic_timer_even_division_is_a_full_period_not_instant() {
        // A duration that divides evenly must not schedule an instant (0 ms) tick.
        assert_eq!(update_periodic_timer(6000, 3000), 3000);
        // Duration within a single period is returned as-is.
        assert_eq!(update_periodic_timer(2000, 3000), 2000);
    }

    // ── periodic tick handlers ───────────────────────────────────────────────

    #[tokio::test]
    async fn periodic_damage_reduces_target_health() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        add_player_with_health(&world, target, 1000, 1000);
        let bc = Arc::clone(&world.managers.broadcast_mgr);

        handle_periodic_damage(
            target,
            &snapshot(caster, AURA_PERIODIC_DAMAGE, 150),
            &world,
            &bc,
        )
        .unwrap();

        assert_eq!(health_of(&world, target), 850);
    }

    #[tokio::test]
    async fn periodic_damage_zero_value_is_a_noop() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        add_player_with_health(&world, target, 1000, 1000);
        let bc = Arc::clone(&world.managers.broadcast_mgr);

        handle_periodic_damage(
            target,
            &snapshot(caster, AURA_PERIODIC_DAMAGE, 0),
            &world,
            &bc,
        )
        .unwrap();

        assert_eq!(health_of(&world, target), 1000);
    }

    #[tokio::test]
    async fn periodic_heal_caps_at_max_health() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        add_player_with_health(&world, target, 500, 1000);
        let bc = Arc::clone(&world.managers.broadcast_mgr);

        // Partial heal.
        handle_periodic_heal(
            target,
            &snapshot(caster, AURA_PERIODIC_HEAL, 200),
            &world,
            &bc,
        )
        .unwrap();
        assert_eq!(health_of(&world, target), 700);

        // Overheal is clamped to max health.
        handle_periodic_heal(
            target,
            &snapshot(caster, AURA_PERIODIC_HEAL, 10_000),
            &world,
            &bc,
        )
        .unwrap();
        assert_eq!(health_of(&world, target), 1000);
    }

    #[tokio::test]
    async fn periodic_leech_damages_target_and_heals_caster() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        add_player_with_health(&world, target, 1000, 1000);
        add_player_with_health(&world, caster, 500, 1000);

        handle_periodic_leech(target, &snapshot(caster, AURA_PERIODIC_LEECH, 100), &world).unwrap();

        assert_eq!(health_of(&world, target), 900);
        assert_eq!(health_of(&world, caster), 600);
    }

    #[tokio::test]
    async fn periodic_leech_caster_heal_capped_at_max() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        add_player_with_health(&world, target, 1000, 1000);
        add_player_with_health(&world, caster, 990, 1000);

        handle_periodic_leech(target, &snapshot(caster, AURA_PERIODIC_LEECH, 100), &world).unwrap();

        // Target takes the full 100; caster heal is clamped to the 10 missing health.
        assert_eq!(health_of(&world, target), 900);
        assert_eq!(health_of(&world, caster), 1000);
    }

    #[tokio::test]
    async fn periodic_damage_percent_deals_pct_of_max_health() {
        let world = test_world();
        let caster = ObjectGuid::new_player(1);
        let target = ObjectGuid::new_player(2);
        add_player_with_health(&world, target, 1000, 1000);

        // Handler treats current_value/100 as the fraction of max health, so 10 = 10%
        // of 1000 max health = 100 damage.
        handle_periodic_damage_percent(
            target,
            &snapshot(caster, AURA_PERIODIC_DAMAGE_PERCENT, 10),
            &world,
        )
        .await
        .unwrap();

        assert_eq!(health_of(&world, target), 900);
    }
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
        world
            .systems
            .player
            .manager()
            .with_player(target_guid, |player| {
                (player.stats.max_health as f32 * pct) as u32
            })
            .unwrap_or(0)
    } else if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature(target_guid, |creature| {
                (creature.max_health as f32 * pct) as u32
            })
            .unwrap_or(0)
    } else {
        0
    };

    if damage > 0 {
        // Apply the damage
        if target_guid.is_player() {
            let died = world
                .systems
                .player
                .manager()
                .with_player_mut(target_guid, |player| {
                    let current = player.stats.health;
                    let new_health = current.saturating_sub(damage);
                    player.stats.health = new_health;
                    player.stats.dirty = true;
                    new_health == 0 && current > 0
                })
                .unwrap_or(false);

            if died {
                let _ = world.systems.death.on_killed(
                    target_guid,
                    Some(snapshot.caster_guid),
                    Some(snapshot.spell_id),
                    world,
                );
            }
        } else if target_guid.is_creature() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let result = world.managers.creature_mgr.apply_damage(
                target_guid,
                damage,
                snapshot.caster_guid,
                timestamp,
            );
            if let Some((_actual, is_dead)) = result {
                if is_dead {
                    tracing::info!(
                        "Creature {:?} killed by periodic damage percent spell {}",
                        target_guid,
                        snapshot.spell_id
                    );
                }
            }
        }
    }

    Ok(())
}
