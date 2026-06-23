//! Creature-to-Player Melee Attack System
//!
//! Executes creature auto-attacks against player targets.
//! Called from the AI executor when AIAction::MeleeAttack fires.

use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::core::common::packet::WorldPacketGuidExt;
use crate::game::broadcast_mgr::broadcast_around_creature;
use crate::game::common::update_fields::*;
use crate::game::creature::combat::{
    apply_hit_outcome, calculate_melee_damage, hit_outcome_to_hit_info,
    hit_outcome_to_victim_state, roll_melee_hit_outcome, MeleeHitOutcome,
};
use crate::World;
use oxcore_shared::messages::combat::{SmsgAttackStart, SmsgAttackStop, SmsgAttackerStateUpdate};
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Opcode, WorldPacket};

/// Perform a creature melee attack against a player target.
///
/// Called from AI executor when attack timer fires.
/// Validates range/alive status, rolls hit table, applies damage,
/// sends packets, and handles player death.
pub fn perform_creature_melee_attack(
    world: &World,
    creature_guid: ObjectGuid,
    target_guid: ObjectGuid,
) {
    // Only handle creature -> player attacks
    if !target_guid.is_player() {
        return;
    }

    // Check creature is alive and get attack data
    let creature_data = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            if !creature.is_alive() {
                return None;
            }
            // Check attack timer is ready
            if !creature.is_attack_ready() {
                return None;
            }
            Some((
                creature.level,
                creature.damage_min,
                creature.damage_max,
                creature.base_attack_time,
                creature.position,
                creature.combat_reach,
            ))
        })
        .flatten();

    let (attacker_level, damage_min, damage_max, base_attack_time, creature_pos, creature_reach) =
        match creature_data {
            Some(data) => data,
            None => return,
        };

    // Check target is alive and get their combat info
    let target_data = world
        .managers
        .player_mgr
        .with_player_mut(target_guid, |player| {
            if !player.is_alive() {
                return None;
            }
            Some((
                player.level,
                player.stats.armor,
                player.combat.can_parry,
                player.combat.can_block,
                player.movement.position,
            ))
        })
        .flatten();

    let (target_level, target_armor, can_parry, can_block, target_pos) = match target_data {
        Some(data) => data,
        None => return,
    };

    // Range check using MaNGOS formula with 2D distance
    use super::melee_range::{self, DEFAULT_COMBAT_REACH};
    if !melee_range::is_within_melee_range(
        &creature_pos,
        creature_reach,
        &target_pos,
        DEFAULT_COMBAT_REACH,
        false, // leeway not applied for creature attacks
    ) {
        return;
    }

    // Reset attack timer
    world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            creature.reset_attack_timer(base_attack_time);
        });

    // Roll hit outcome using the 8-outcome table
    let hit_outcome = roll_melee_hit_outcome(attacker_level, target_level, can_parry, can_block);

    // Calculate base damage with armor reduction, then apply outcome modifier
    let base_damage = calculate_melee_damage(attacker_level, damage_min, damage_max, target_armor);
    let damage = apply_hit_outcome(base_damage, &hit_outcome);

    // Apply damage to player
    let mut target_died = false;
    if damage > 0 {
        let damage_result = world
            .managers
            .player_mgr
            .with_player_mut(target_guid, |player| {
                let old_health = player.stats.health;
                let actual = damage.min(old_health);
                player.stats.health = old_health.saturating_sub(actual);
                (player.stats.health, actual)
            });

        if let Some((health, actual_damage)) = damage_result {
            reward_victim_rage_if_actual_damage(actual_damage, |damage| {
                let _ = world
                    .systems
                    .power
                    .on_damage_taken(target_guid, damage, world);
            });

            if health == 0 {
                target_died = true;
            }
        }
    }

    // Send SMSG_ATTACKERSTATEUPDATE
    let hit_info = hit_outcome_to_hit_info(&hit_outcome);
    let victim_state = hit_outcome_to_victim_state(&hit_outcome);
    let blocked = match &hit_outcome {
        MeleeHitOutcome::Block { blocked_amount } => *blocked_amount,
        _ => 0,
    };
    let attack_packet = SmsgAttackerStateUpdate {
        hit_info,
        attacker_guid: creature_guid,
        target_guid,
        total_damage: damage,
        damage_school: 0, // Physical
        absorbed: 0,
        resisted: 0,
        victim_state,
        blocked,
    };

    broadcast_around_creature(world, creature_guid, &attack_packet.to_world_packet());

    // Send player health update
    send_player_health_update(world, target_guid);

    // Handle player death via the death system (sends death packets, sets Corpse state, etc.)
    if target_died {
        handle_creature_kill_cleanup(world, target_guid, creature_guid);
        if let Err(e) = world
            .systems
            .death
            .on_killed(target_guid, Some(creature_guid), None, world)
        {
            tracing::error!("Failed to handle player death from melee attack: {}", e);
        }
    }
}

fn reward_victim_rage_if_actual_damage<F>(actual_damage: u32, mut reward: F)
where
    F: FnMut(u32),
{
    if actual_damage > 0 {
        reward(actual_damage);
    }
}

/// Send a partial health update for a player to nearby players
fn send_player_health_update(world: &World, player_guid: ObjectGuid) {
    let health_data = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            (player.stats.health, player.stats.max_health)
        });

    if let Some((current_health, max_health)) = health_data {
        let world_guid = WorldObjectGuid::new_player(player_guid.counter());

        let values_block = ValuesUpdateBlock::new(world_guid, ObjectType::Player)
            .set_field(UNIT_FIELD_HEALTH, current_health);

        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values_block));

        world.managers.broadcast_mgr.broadcast_nearby(
            player_guid,
            &msg.to_world_packet(),
            true, // include self so player sees their own health drop
        );
    }
}

/// Handle player death from creature kill
///
/// Basic implementation: sets death state, stops combat, removes from threat lists.
/// Full corpse/ghost/graveyard system is handled by the existing death system.
fn handle_creature_kill_cleanup(world: &World, player_guid: ObjectGuid, killer_guid: ObjectGuid) {
    // Remove player from all creature threat lists
    let creature_guids: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|creature| {
            creature.combat.attackers.contains(&player_guid)
                || creature.threat_manager.has_target(player_guid)
        })
        .map(|e| *e.key())
        .collect();

    for cg in creature_guids {
        world
            .managers
            .creature_mgr
            .with_creature_mut(cg, |creature| {
                creature.combat.remove_attacker(player_guid);
                creature.threat_manager.remove_target(player_guid);
            });
    }

    // Send attack stop from killer
    let stop_packet = SmsgAttackStop {
        attacker_guid: killer_guid,
        target_guid: player_guid,
        unk: 0,
    };
    broadcast_around_creature(world, killer_guid, &stop_packet.to_world_packet());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_actual_creature_damage_rewards_victim_rage() {
        let mut rewarded_damage = None;

        reward_victim_rage_if_actual_damage(42, |damage| {
            rewarded_damage = Some(damage);
        });

        assert_eq!(rewarded_damage, Some(42));
    }

    #[test]
    fn zero_actual_creature_damage_does_not_reward_victim_rage() {
        let mut called = false;

        reward_victim_rage_if_actual_damage(0, |_| {
            called = true;
        });

        assert!(!called);
    }
}
