//! Attack Handler - Player attacks on creatures

use crate::shared::messages::combat::{SmsgAttackStart, SmsgAttackStop, SmsgAttackerStateUpdate};
use crate::shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::game::broadcast_mgr::broadcast_around_creature;
use crate::world::game::common::update_fields::*;
use crate::world::game::creature::combat::{
    apply_hit_outcome, calculate_melee_damage, hit_outcome_to_hit_info,
    hit_outcome_to_victim_state, roll_melee_hit_outcome, MeleeHitOutcome,
};
use crate::world::World;

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
    // MaNGOS: sends SMSG_ATTACKSTOP + SMSG_ATTACKSWING_DEADTARGET when target is dead
    if !world.managers.creature_mgr.is_alive(target_guid) {
        tracing::info!("[COMBAT] CMSG_ATTACKSWING on dead target {:?} from {:?} — sending DEADTARGET + ATTACKSTOP", target_guid, attacker_guid);
        // Tell client the target is dead (stops client auto-attack loop)
        let dead_packet = crate::shared::protocol::WorldPacket::new(
            crate::shared::protocol::Opcode::SMSG_ATTACKSWING_DEADTARGET,
        );
        world
            .managers
            .broadcast_mgr
            .send_to_player(attacker_guid, dead_packet);
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
        attacker_guid, target_guid, in_objects_created, creature_pos
    );

    // Start auto-attack via CombatSystem (sets is_auto_attacking, attack_target, timer=0)
    world
        .systems
        .combat
        .start_attack(attacker_guid, target_guid, &world.managers.player_mgr)
        .await?;

    // Enter combat state
    world
        .systems
        .combat
        .enter_combat(attacker_guid, target_guid, &world.managers.player_mgr);

    // Broadcast SMSG_ATTACKSTART to nearby players
    let packet = SmsgAttackStart {
        attacker_guid,
        target_guid,
    };
    world
        .managers
        .broadcast_mgr
        .broadcast_nearby(attacker_guid, &packet.to_world_packet(), true);

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
        let dead_packet = crate::shared::protocol::WorldPacket::new(
            crate::shared::protocol::Opcode::SMSG_ATTACKSWING_DEADTARGET,
        );
        world
            .managers
            .broadcast_mgr
            .send_to_player(attacker_guid, dead_packet);
        send_attack_stop(world, attacker_guid, target_guid, true);
        return Ok(true);
    }

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

        // Trigger AI event
        if actual_damage > 0 && !is_dead {
            crate::world::game::creature::ai::queue_event(
                world,
                target_guid,
                crate::world::game::creature::ai::AIEvent::DamageTaken {
                    attacker_guid,
                    damage: actual_damage,
                    spell_id: None,
                    school: 0,
                },
            );
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

            // 1. ATTACKSTOP from both directions (MaNGOS Kill → AttackStop + CombatStop)
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

            crate::world::game::creature::ai::queue_event(
                world,
                target_guid,
                crate::world::game::creature::ai::AIEvent::Died {
                    killer_guid: Some(attacker_guid),
                },
            );
        } else if actual_damage > 0 {
            send_health_update(world, target_guid)?;
        }
    }

    Ok(target_died)
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
    world.managers.broadcast_mgr.broadcast_nearby(
        attacker,
        &packet.to_world_packet(),
        true, // include self
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
        broadcast_around_creature(world, creature_guid, &update.to_world_packet());
    }

    Ok(())
}

/// Send death VALUES update on killing blow.
/// Matches vmangos Kill(): only sends health=0 + stand_state=7 + clear target/flags.
/// NOTE: vmangos does NOT set UNIT_DYNFLAG_DEAD on real death (only feign death).
/// UNIT_DYNFLAG_LOOTABLE is set separately after loot generation.
fn send_creature_killed_update(world: &World, creature_guid: ObjectGuid) -> anyhow::Result<()> {
    let (max_health, unit_flags) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| (c.max_health, c.unit_flags))
        .unwrap_or((1, 0));

    // Clear IN_COMBAT from unit flags (MaNGOS ClearInCombat)
    let cleared_flags = unit_flags & !crate::world::game::common::unit_flags::IN_COMBAT;

    tracing::debug!(
        "[COMBAT] send_creature_killed_update {:?}: HEALTH=0, MAXHEALTH={}, FLAGS=0x{:08X}, BYTES1=7, NPC_FLAGS=0",
        creature_guid, max_health, cleared_flags
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
    broadcast_around_creature(world, creature_guid, &update.to_world_packet());
    Ok(())
}

/// Send SMSG_ATTACKSTOP - broadcast to nearby players so all clients update combat state.
/// vmangos SendMeleeAttackStop always sends unk=0.
fn send_attack_stop(world: &World, attacker: ObjectGuid, target: ObjectGuid, _target_dead: bool) {
    let packet = SmsgAttackStop {
        attacker_guid: attacker,
        target_guid: target,
        unk: 0,
    };

    world.managers.broadcast_mgr.broadcast_nearby(
        attacker,
        &packet.to_world_packet(),
        true, // include self
    );
}

/// Send SMSG_ATTACKSTOP directly to the attacker player only (not broadcast).
/// Used in HandleAttackSwingOpcode for dead targets — matches vmangos SendAttackStop().
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
        .send_to_player(attacker, packet.to_world_packet());
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

    broadcast_around_creature(world, creature_guid, &packet.to_world_packet());
}
