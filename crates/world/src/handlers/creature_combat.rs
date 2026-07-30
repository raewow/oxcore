//! Attack Handler - Player attacks on creatures

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
use oxcore_shared::protocol::{ObjectGuid, Opcode, Protocol, WorldPacket};

pub fn read_attack_swing_target(
    protocol: Protocol,
    packet: &mut WorldPacket,
) -> Option<ObjectGuid> {
    packet.read_guid_for(protocol)
}

/// Handle player attack swing (CMSG_ATTACKSWING)
/// Initializes auto-attack state and sends SMSG_ATTACKSTART.
/// Actual damage is dealt by execute_pending_attack_vs_creature() in the update loop.
pub async fn handle_attack_swing(
    world: &World,
    attacker_guid: ObjectGuid,
    target_guid: ObjectGuid,
) -> anyhow::Result<()> {
    // Validate target is a creature (unit but not player)
    if !target_guid.is_unit() || target_guid.is_player() {
        tracing::debug!("Attack target {:?} is not a creature", target_guid);
        return Ok(());
    }

    // Check creature exists and is alive
    // Sends SMSG_ATTACKSTOP + SMSG_ATTACKSWING_DEADTARGET when target is dead
    if !world.managers.creature_mgr.is_alive(target_guid) {
        tracing::info!(
            "[COMBAT] CMSG_ATTACKSWING on dead target {:?} from {:?} — sending DEADTARGET + ATTACKSTOP",
            target_guid,
            attacker_guid
        );
        // Tell client the target is dead (stops client auto-attack loop)
        let dead_packet = oxcore_shared::protocol::WorldPacket::new(
            oxcore_shared::protocol::Opcode::SMSG_ATTACKSWING_DEADTARGET,
        );
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(attacker_guid, dead_packet);
        send_attack_stop_to_player(world, attacker_guid, target_guid, true);
        // Also clear server-side auto-attack state
        world
            .systems
            .combat
            .stop_attack(attacker_guid, &world.managers.player_mgr)
            .await?;
        return Ok(());
    }

    // Diagnostic: check if creature is in player's objects_created
    let in_objects_created = world
        .managers
        .player_mgr
        .has_object_created(attacker_guid, target_guid);
    let creature_pos = world.managers.creature_mgr.get_position(target_guid);
    tracing::warn!(
        "[COMBAT] ATTACKSWING: player {:?} attacking creature {:?}, in_objects_created={}, creature_pos={:?}",
        attacker_guid,
        target_guid,
        in_objects_created,
        creature_pos
    );

    // Start auto-attack via CombatSystem (sets is_auto_attacking, attack_target, timer=0)
    world
        .systems
        .combat
        .start_attack(attacker_guid, target_guid, &world.managers.player_mgr)
        .await?;

    // Broadcast SMSG_ATTACKSTART to nearby players
    let packet = SmsgAttackStart {
        attacker_guid,
        target_guid,
    };
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_nearby(attacker_guid, &packet, true);

    Ok(())
}

/// Execute a pending auto-attack swing against a creature target.
/// Called from the player update loop when a swing timer fires.
/// Returns true if the target died.
pub async fn execute_pending_attack_vs_creature(
    world: &World,
    attacker_guid: ObjectGuid,
    target_guid: ObjectGuid,
) -> anyhow::Result<bool> {
    // Validate target is still alive
    if !world.managers.creature_mgr.is_alive(target_guid) {
        // Send SMSG_ATTACKSWING_DEADTARGET to stop client auto-attack loop
        let dead_packet = oxcore_shared::protocol::WorldPacket::new(
            oxcore_shared::protocol::Opcode::SMSG_ATTACKSWING_DEADTARGET,
        );
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(attacker_guid, dead_packet);
        send_attack_stop(world, attacker_guid, target_guid, true);
        return Ok(true);
    }

    // Starting auto-attack only arms the swing timer. Combat begins when the
    // first swing is actually attempted, after the range check in the update loop.
    world
        .systems
        .combat
        .enter_combat(attacker_guid, target_guid, &world.managers.player_mgr);

    // Get attacker weapon damage from combat state (with level-based fallback)
    let (attacker_level, weapon_min, weapon_max) = world
        .managers
        .player_mgr
        .with_player_mut(attacker_guid, |player| {
            let base_min = player.combat.main_hand_min_dmg as u32;
            let base_max = player.combat.main_hand_max_dmg as u32;
            // Fallback to level-based defaults if weapon damage is trivially low
            let min = if base_min <= 1 {
                5 + (player.level as u32 * 2)
            } else {
                base_min
            };
            let max = if base_max <= 2 {
                10 + (player.level as u32 * 3)
            } else {
                base_max
            };
            (player.level, min, max)
        })
        .unwrap_or((1, 10, 20));

    // Get target armor and level
    let (target_armor, target_level) = world
        .managers
        .creature_mgr
        .with_creature_mut(target_guid, |creature| (creature.armor, creature.level))
        .unwrap_or((0, 1));

    // Roll hit outcome (creatures don't parry or block player attacks)
    let hit_outcome = roll_melee_hit_outcome(attacker_level, target_level, false, false);

    // Calculate base damage with armor reduction, then apply hit outcome modifier
    let base_damage = calculate_melee_damage(attacker_level, weapon_min, weapon_max, target_armor);
    let damage = apply_hit_outcome(base_damage, &hit_outcome);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Apply damage
    let mut target_died = false;
    let health_before = world.managers.creature_mgr.get_health(target_guid);
    if let Some((actual_damage, is_dead)) =
        world
            .managers
            .creature_mgr
            .apply_damage(target_guid, damage, attacker_guid, timestamp)
    {
        let health_after = world.managers.creature_mgr.get_health(target_guid);
        tracing::debug!(
            "[COMBAT] {:?} hit {:?}: damage={}, actual={}, is_dead={}, health {:?} -> {:?}",
            attacker_guid,
            target_guid,
            damage,
            actual_damage,
            is_dead,
            health_before,
            health_after
        );

        // Trigger AI event. A swing that misses (or is dodged/parried/fully absorbed)
        // still engages the creature — both route through the victim's AI.
        if !is_dead {
            let event = if actual_damage > 0 {
                crate::game::creature::ai::AIEvent::DamageTaken {
                    attacker_guid,
                    damage: actual_damage,
                    spell_id: None,
                    school: 0,
                }
            } else {
                crate::game::creature::ai::AIEvent::AttackedBy { attacker_guid }
            };
            crate::game::creature::ai::queue_event(world, target_guid, event);
        }

        // Send damage packet
        send_attacker_state_update(
            world,
            attacker_guid,
            target_guid,
            actual_damage,
            &hit_outcome,
            is_dead,
        )?;

        reward_attacker_rage_if_actual_damage(actual_damage, |damage| {
            world
                .systems
                .power
                .on_damage_dealt(attacker_guid, damage, world)
        })?;

        // Send health or death update
        if is_dead {
            target_died = true;
            tracing::info!(
                "[COMBAT] Creature {:?} killed by {:?}, calling handle_death",
                target_guid,
                attacker_guid
            );

            // Death packet order: stop movement BEFORE death state change.
            // The 1.12.1 client cancels the active spline when it sees the dead
            // stand state (BYTES_1=7), snapping the creature to its base position
            // from CREATE_OBJECT2. By sending the stop packet first, we ensure the
            // client places the creature at its actual death location before
            // processing the death animation.

            // 1. ATTACKSTOP from both directions
            send_attack_stop(world, attacker_guid, target_guid, true);
            send_creature_attack_stop(world, target_guid, attacker_guid, true);

            // 2. Mark dead on server (snaps position to spline location, stops movement)
            let death_info = world
                .managers
                .creature_mgr
                .handle_death(target_guid, Some(attacker_guid));

            // 3. Send movement stop packet BEFORE death VALUES update
            // This tells the client to stop the spline at the death position
            if let Some(ref info) = death_info {
                world
                    .systems
                    .creature_movement
                    .send_stop_packet(info.guid, info.position, world);
            }

            // 4. Death VALUES update (health=0, stand state Dead)
            send_creature_killed_update(world, target_guid)?;

            crate::game::creature::ai::queue_event(
                world,
                target_guid,
                crate::game::creature::ai::AIEvent::Died {
                    killer_guid: Some(attacker_guid),
                },
            );
        } else if actual_damage > 0 {
            send_health_update(world, target_guid)?;
        }
    }

    // Caster-side melee-swing procs (weapon enchants, Windfury, flurry triggers, etc.).
    // The proc eligibility gate filters non-connecting outcomes for default procs; a swing
    // always counts as a DEAL_MELEE_SWING event with the rolled hit outcome.
    {
        use crate::game::player::auras::proc::proc_flags;
        let _ = world
            .systems
            .auras
            .check_procs(
                attacker_guid,
                proc_flags::DEAL_MELEE_SWING | proc_flags::MAIN_HAND_WEAPON_SWING,
                melee_swing_proc_ex(&hit_outcome),
                None, // melee swing — no triggering spell
                damage,
                world,
            )
            .await;
    }

    Ok(target_died)
}

fn reward_attacker_rage_if_actual_damage<F>(actual_damage: u32, mut reward: F) -> anyhow::Result<()>
where
    F: FnMut(u32) -> anyhow::Result<()>,
{
    if actual_damage > 0 {
        reward(actual_damage)?;
    }
    Ok(())
}

/// Map a melee swing outcome to the proc extra-flag (ProcFlagsEx) describing it.
fn melee_swing_proc_ex(outcome: &MeleeHitOutcome) -> u32 {
    use crate::game::player::auras::proc::proc_flags_ex;
    match outcome {
        MeleeHitOutcome::Crit => proc_flags_ex::CRITICAL_HIT,
        MeleeHitOutcome::Miss => proc_flags_ex::MISS,
        MeleeHitOutcome::Dodge => proc_flags_ex::DODGE,
        MeleeHitOutcome::Parry => proc_flags_ex::PARRY,
        MeleeHitOutcome::Block { .. } => proc_flags_ex::BLOCK,
        // Hit, Glancing and Crushing all connect as a normal hit for proc purposes.
        _ => proc_flags_ex::NORMAL_HIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::auras::proc::proc_flags_ex;
    use oxcore_shared::protocol::bitbuf::BitWriter;

    #[test]
    fn swing_outcome_maps_to_proc_ex() {
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Crit),
            proc_flags_ex::CRITICAL_HIT
        );
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Hit),
            proc_flags_ex::NORMAL_HIT
        );
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Miss),
            proc_flags_ex::MISS
        );
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Dodge),
            proc_flags_ex::DODGE
        );
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Parry),
            proc_flags_ex::PARRY
        );
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Block { blocked_amount: 5 }),
            proc_flags_ex::BLOCK
        );
        // Glancing/Crushing connect as normal hits for proc purposes.
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Glancing { reduction: 0.3 }),
            proc_flags_ex::NORMAL_HIT
        );
        assert_eq!(
            melee_swing_proc_ex(&MeleeHitOutcome::Crushing),
            proc_flags_ex::NORMAL_HIT
        );
    }

    #[test]
    fn modern_attack_swing_reads_packed_guid128() {
        let expected = ObjectGuid::new_creature(299, 464);
        let (high, low) = expected.to_guid128(1);
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(high, low);
        let mut packet = WorldPacket::new(Opcode::CMSG_ATTACKSWING);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(read_attack_swing_target(Protocol::Modern, &mut packet).unwrap(), expected);
    }

    #[test]
    fn positive_actual_melee_damage_rewards_attacker_rage() {
        let mut rewarded_damage = None;

        reward_attacker_rage_if_actual_damage(37, |damage| {
            rewarded_damage = Some(damage);
            Ok(())
        })
        .unwrap();

        assert_eq!(rewarded_damage, Some(37));
    }

    #[test]
    fn zero_actual_melee_damage_does_not_reward_attacker_rage() {
        let mut called = false;

        reward_attacker_rage_if_actual_damage(0, |_| {
            called = true;
            Ok(())
        })
        .unwrap();

        assert!(!called);
    }
}

/// Handle attack stop (CMSG_ATTACKSTOP)
pub async fn handle_attack_stop(world: &World, attacker_guid: ObjectGuid) -> anyhow::Result<()> {
    // Get the current attack target from the player's combat state
    if let Some(target_guid) = world
        .systems
        .combat
        .get_attack_target(attacker_guid, &world.managers.player_mgr)
    {
        let target_dead = !world.managers.creature_mgr.is_alive(target_guid);
        send_attack_stop(world, attacker_guid, target_guid, target_dead);
    }

    // Stop the attack in the combat system
    world
        .systems
        .combat
        .stop_attack(attacker_guid, &world.managers.player_mgr)
        .await?;

    Ok(())
}

/// Send SMSG_ATTACKERSTATEUPDATE
fn send_attacker_state_update(
    world: &World,
    attacker: ObjectGuid,
    target: ObjectGuid,
    damage: u32,
    outcome: &MeleeHitOutcome,
    is_dead: bool,
) -> anyhow::Result<()> {
    let hit_info = hit_outcome_to_hit_info(outcome);
    let victim_state = hit_outcome_to_victim_state(outcome);
    let blocked = match outcome {
        MeleeHitOutcome::Block { blocked_amount } => *blocked_amount,
        _ => 0,
    };

    let packet = SmsgAttackerStateUpdate {
        hit_info,
        attacker_guid: attacker,
        target_guid: target,
        total_damage: damage,
        damage_school: 0, // Physical
        absorbed: 0,
        resisted: 0,
        victim_state,
        blocked,
    };

    // Broadcast to nearby players
    world.managers.broadcast_mgr.broadcast_msg_nearby(
        attacker, &packet, true, // include self
    );

    Ok(())
}

/// Send health update for creature
fn send_health_update(world: &World, creature_guid: ObjectGuid) -> anyhow::Result<()> {
    if let Some((current, max)) = world.managers.creature_mgr.get_health(creature_guid) {
        tracing::debug!(
            "[COMBAT] send_health_update {:?}: health={}/{}",
            creature_guid,
            current,
            max
        );
        let world_guid =
            WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
                .set_field(UNIT_FIELD_HEALTH, current)
                .set_field(UNIT_FIELD_MAXHEALTH, max),
        ));
        broadcast_around_creature(world, creature_guid, &update);
    }

    Ok(())
}

/// Send death VALUES update on killing blow.
/// Only sends health=0 + stand_state=7 + clear target/flags.
/// Does NOT set UNIT_DYNFLAG_DEAD on real death (only feign death).
/// UNIT_DYNFLAG_LOOTABLE is set separately after loot generation.
fn send_creature_killed_update(world: &World, creature_guid: ObjectGuid) -> anyhow::Result<()> {
    let (max_health, unit_flags) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| (c.max_health, c.unit_flags))
        .unwrap_or((1, 0));

    // Clear IN_COMBAT from unit flags
    let cleared_flags = unit_flags & !crate::game::common::unit_flags::IN_COMBAT;

    tracing::debug!(
        "[COMBAT] send_creature_killed_update {:?}: HEALTH=0, MAXHEALTH={}, FLAGS=0x{:08X}, BYTES1=7, NPC_FLAGS=0",
        creature_guid,
        max_health,
        cleared_flags
    );
    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
    let empty_guid = WorldObjectGuid::from_raw(0);
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_guid_field(UNIT_FIELD_TARGET, empty_guid) // Clear target
            .set_field(UNIT_FIELD_HEALTH, 0u32)
            .set_field(UNIT_FIELD_MAXHEALTH, max_health)
            .set_field(UNIT_FIELD_FLAGS, cleared_flags) // Clear IN_COMBAT
            .set_field(UNIT_DYNAMIC_FLAGS, 0u32) // Clear dynamic flags (no DYNFLAG_DEAD — that's feign death only)
            .set_field(UNIT_FIELD_BYTES_1, 7u32) // Stand state Dead = 7 (UNIT_STAND_STATE_DEAD)
            .set_field(UNIT_NPC_FLAGS, 0u32), // Clear NPC interaction flags
    ));
    broadcast_around_creature(world, creature_guid, &update);
    Ok(())
}

/// Send SMSG_ATTACKSTOP - broadcast to nearby players so all clients update combat state.
/// Always sends unk=0.
fn send_attack_stop(world: &World, attacker: ObjectGuid, target: ObjectGuid, _target_dead: bool) {
    let packet = SmsgAttackStop {
        attacker_guid: attacker,
        target_guid: target,
        unk: 0,
    };

    world.managers.broadcast_mgr.broadcast_msg_nearby(
        attacker, &packet, true, // include self
    );
}

/// Send SMSG_ATTACKSTOP directly to the attacker player only (not broadcast).
/// Used to send attack stop for dead targets.
fn send_attack_stop_to_player(
    world: &World,
    attacker: ObjectGuid,
    target: ObjectGuid,
    _target_dead: bool,
) {
    let packet = SmsgAttackStop {
        attacker_guid: attacker,
        target_guid: target,
        unk: 0,
    };

    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(attacker, packet);
}

/// Send attack stop from a creature (uses creature position for broadcast)
fn send_creature_attack_stop(
    world: &World,
    creature_guid: ObjectGuid,
    target: ObjectGuid,
    _target_dead: bool,
) {
    let packet = SmsgAttackStop {
        attacker_guid: creature_guid,
        target_guid: target,
        unk: 0,
    };

    broadcast_around_creature(world, creature_guid, &packet);
}
