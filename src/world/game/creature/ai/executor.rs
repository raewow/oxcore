//! AI Action Executor - Applies AI decisions to the game world
//!
//! This module executes the actions produced by AI decision functions.
//! Actions are applied in order with proper locking to maintain consistency.

use super::types::{AIAction, AIState, AIType};
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::broadcast_around_creature;
use crate::world::World;
use std::time::{SystemTime, UNIX_EPOCH};

/// Execute a batch of AI actions for a creature
/// Actions are applied in order with proper locking
pub fn execute_actions(world: &World, creature_guid: ObjectGuid, actions: Vec<AIAction>) {
    for action in actions {
        execute_single_action(world, creature_guid, action);
    }
}

fn execute_single_action(world: &World, creature_guid: ObjectGuid, action: AIAction) {
    match action {
        AIAction::SetAttackTarget { target_guid } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.combat.attacking = Some(target_guid);
                });
        }

        AIAction::ClearAttackTarget => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.combat.attacking = None;
                });
        }

        AIAction::MeleeAttack { target_guid } => {
            crate::world::game::combat::creature_attacks::perform_creature_melee_attack(
                world,
                creature_guid,
                target_guid,
            );
        }

        AIAction::MoveToTarget {
            target_guid,
            min_distance: _,
        } => {
            // Phase 8: Start chasing the target
            let current_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let combat_reach = creature.combat_reach;
                    let run_speed = creature.run_speed();
                    creature.motion_master.chase(
                        target_guid,
                        creature_guid,
                        current_pos,
                        combat_reach,
                        run_speed,
                    );
                });

            tracing::debug!(
                "[AI] Creature {:?} started chasing {:?}",
                creature_guid,
                target_guid
            );
        }

        AIAction::MoveTo {
            position,
            movement_type,
        } => {
            // Movement comes in Phase 8
            tracing::debug!(
                "[AI] Creature {:?} wants to move to {:?} ({:?})",
                creature_guid,
                position,
                movement_type
            );
        }

        AIAction::ReturnToSpawn => {
            // Phase 8: Start returning home
            let current_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let home = creature.home_position;
                    let run_speed = creature.run_speed();
                    creature
                        .motion_master
                        .return_home(home, creature_guid, current_pos, run_speed);
                    creature.ai_state = AIState::Returning;
                });

            tracing::debug!("[AI] Creature {:?} returning to spawn", creature_guid);
        }

        AIAction::EnterEvadeMode => {
            // Phase 8: Stop movement and return home on evade
            let current_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.motion_master.stop(creature_guid);
                    creature.move_spline.stop();
                    let home = creature.home_position;
                    let run_speed = creature.run_speed();
                    creature
                        .motion_master
                        .return_home(home, creature_guid, current_pos, run_speed);
                    creature.ai_state = AIState::Evading;
                    creature.combat.leave_combat();
                    creature.threat_manager.clear();
                    // Heal to full on evade
                    creature.current_health = creature.max_health;
                    creature.current_mana = creature.max_mana;
                    // Clear combat flags
                    creature.unit_flags &= !crate::world::game::common::unit_flags::IN_COMBAT;
                });

            // Send health update to nearby players
            send_health_update(world, creature_guid);

            // Send VALUES update clearing combat flags and target
            send_combat_exit_update(world, creature_guid);

            tracing::debug!("[AI] Creature {:?} entering evade mode", creature_guid);
        }

        AIAction::LeaveCombat => {
            // Restore wander movement if creature had wander_distance
            let restore_wander =
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(creature_guid, |creature| {
                        creature.ai_state = AIState::Idle;
                        creature.combat.leave_combat();
                        creature.threat_manager.clear(); // Clear threat when leaving combat (Phase 5)
                                                         // Clear combat flags
                        creature.unit_flags &= !crate::world::game::common::unit_flags::IN_COMBAT;

                        // Return info needed to restore wander
                        (
                            creature.wander_distance,
                            creature.home_position,
                            creature.position,
                        )
                    });

            if let Some((wander_dist, home_pos, current_pos)) = restore_wander {
                if wander_dist > 0.0 {
                    world
                        .managers
                        .creature_mgr
                        .with_creature_mut(creature_guid, |creature| {
                            let walk_speed = creature.walk_speed();
                            creature.motion_master.random_wander(
                                home_pos,
                                wander_dist,
                                creature_guid,
                                current_pos,
                                walk_speed,
                            );
                        });
                }
            }

            // Send VALUES update clearing combat flags and target
            send_combat_exit_update(world, creature_guid);

            tracing::debug!("[AI] Creature {:?} leaving combat", creature_guid);
        }

        AIAction::EnterCombat { target_guid } => {
            // Get current position for stop packet and chase init
            let current_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            let was_already_in_combat = world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let was_combat = creature.ai_state == AIState::Combat;
                    creature.ai_state = AIState::Combat;
                    creature.combat.in_combat = true;
                    creature.combat.attackers.insert(target_guid);
                    creature.unit_flags |= crate::world::game::common::unit_flags::IN_COMBAT;

                    // Stop any active movement (wander spline, etc.) immediately
                    creature.move_spline.stop();
                    // Clear all generators (removes Random/Waypoint) before starting chase
                    creature.motion_master.clear(creature_guid);
                    // Start chase right away instead of waiting for next AI tick
                    let combat_reach = creature.combat_reach;
                    let run_speed = creature.run_speed();
                    creature.motion_master.chase(
                        target_guid,
                        creature_guid,
                        current_pos,
                        combat_reach,
                        run_speed,
                    );
                    was_combat
                })
                .unwrap_or(true);

            // Send stop packet so client stops showing old wander movement
            world
                .systems
                .creature_movement
                .send_stop_packet(creature_guid, current_pos, world);

            // Send SMSG_ATTACKSTART to notify nearby players
            send_attack_start(world, creature_guid, target_guid);

            // Send VALUES update with IN_COMBAT flag and UNIT_FIELD_TARGET
            send_combat_state_update(world, creature_guid, target_guid);

            // Queue CombatStarted event so Lua OnEnterCombat callback fires next tick
            if !was_already_in_combat {
                super::system::queue_event(
                    world,
                    creature_guid,
                    super::types::AIEvent::CombatStarted {
                        initial_aggressor: target_guid,
                    },
                );
            }

            tracing::debug!(
                "[AI] Creature {:?} entering combat with {:?}",
                creature_guid,
                target_guid
            );
        }

        AIAction::StopMovement => {
            let current_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.motion_master.stop(creature_guid);
                    creature.move_spline.stop();
                });

            // Send stop packet to client so it also stops the creature
            world
                .systems
                .creature_movement
                .send_stop_packet(creature_guid, current_pos, world);
            tracing::debug!("[AI] Creature {:?} stopping movement", creature_guid);
        }

        AIAction::FleeFrom {
            flee_from_guid,
            distance,
            duration_ms,
        } => {
            // Movement comes in Phase 8
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.ai_state = AIState::Fleeing;
                });
            tracing::debug!(
                "[AI] Creature {:?} fleeing from {:?} for {} yards, {} ms",
                creature_guid,
                flee_from_guid,
                distance,
                duration_ms
            );
        }

        AIAction::FaceTarget { target_guid } => {
            // Get creature and target positions to calculate facing angle
            let creature_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            let target_pos = if target_guid.is_player() {
                world.managers.player_mgr.get_player_position(target_guid)
            } else {
                world.managers.creature_mgr.get_position(target_guid)
            };

            if let Some(target_pos) = target_pos {
                let angle = (target_pos.y - creature_pos.y).atan2(target_pos.x - creature_pos.x);

                // Update creature orientation
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(creature_guid, |creature| {
                        creature.position.o = angle;
                    });

                // Send facing packet to nearby players
                world.systems.creature_movement.send_facing_packet(
                    creature_guid,
                    creature_pos,
                    angle,
                    world,
                );
            }
        }

        AIAction::RandomMovement { wander_distance } => {
            // Movement comes in Phase 8
            tracing::debug!(
                "[AI] Creature {:?} random movement (max {} yards)",
                creature_guid,
                wander_distance
            );
        }

        AIAction::CastSpell {
            spell_id,
            target_guid,
            target_position: _,
            triggered,
        } => {
            execute_creature_spell_cast(world, creature_guid, spell_id, target_guid, triggered);
        }

        AIAction::AddThreat {
            target_guid,
            amount,
        } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    // Use ThreatManager for Phase 5 threat handling
                    creature.threat_manager.add_threat(target_guid, amount);

                    // Also update legacy combat threat for backward compatibility
                    creature.combat.add_threat(target_guid, amount, 0);
                });
        }

        AIAction::ModifyThreat {
            target_guid,
            amount,
            is_percent,
        } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    // Get current threat from ThreatManager
                    let current_threat = creature.threat_manager.get_threat(target_guid);
                    let new_threat = if is_percent {
                        current_threat * amount
                    } else {
                        amount
                    };

                    // Calculate delta and add it
                    let delta = new_threat - current_threat;
                    if delta != 0.0 {
                        creature.threat_manager.add_threat(target_guid, delta);
                    }

                    // Also update legacy combat threat
                    if let Some(entry) = creature
                        .combat
                        .threat_list
                        .iter_mut()
                        .find(|e| e.guid == target_guid)
                    {
                        if is_percent {
                            entry.threat *= amount;
                        } else {
                            entry.threat = amount;
                        }
                    }
                });
        }

        AIAction::ClearThreatList => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.threat_manager.clear();
                    creature.combat.threat_list.clear();
                });
        }

        AIAction::SetMeleeAttack { enabled } => {
            tracing::debug!(
                "[AI] Creature {:?} melee attack {}",
                creature_guid,
                if enabled { "enabled" } else { "disabled" }
            );
        }

        AIAction::SetCombatMovement { enabled } => {
            tracing::debug!(
                "[AI] Creature {:?} combat movement {}",
                creature_guid,
                if enabled { "enabled" } else { "disabled" }
            );
        }

        AIAction::SetReactState { react_state } => {
            tracing::debug!(
                "[AI] Creature {:?} react state set to {:?}",
                creature_guid,
                react_state
            );
        }

        AIAction::CallForHelp { radius } => {
            tracing::debug!(
                "[AI] Creature {:?} calling for help ({} yards)",
                creature_guid,
                radius
            );
            // TODO: Implement call for help - notify nearby creatures
        }

        AIAction::SummonGuards { guard_entry, count } => {
            tracing::debug!(
                "[AI] Creature {:?} summoning {} guards (entry {})",
                creature_guid,
                count,
                guard_entry
            );
            // TODO: Implement guard summoning
        }

        AIAction::Say { text } => {
            send_creature_chat(world, creature_guid, 0x0B, &text); // CHAT_MSG_MONSTER_SAY
            tracing::debug!("[AI] Creature {:?} says: {}", creature_guid, text);
        }

        AIAction::Yell { text } => {
            send_creature_chat(world, creature_guid, 0x0C, &text); // CHAT_MSG_MONSTER_YELL
            tracing::debug!("[AI] Creature {:?} yells: {}", creature_guid, text);
        }

        AIAction::PlayEmote { emote_id } => {
            send_creature_emote(world, creature_guid, emote_id);
            tracing::debug!("[AI] Creature {:?} plays emote {}", creature_guid, emote_id);
        }

        AIAction::TextEmote { text } => {
            send_creature_chat(world, creature_guid, 0x0D, &text); // CHAT_MSG_MONSTER_EMOTE
            tracing::debug!("[AI] Creature {:?} text emote: {}", creature_guid, text);
        }

        AIAction::PlaySound {
            sound_id,
            zone_wide: _,
        } => {
            send_creature_sound(world, creature_guid, sound_id);
            tracing::debug!("[AI] Creature {:?} plays sound {}", creature_guid, sound_id);
        }

        AIAction::InterruptSpell => {
            // TODO: Clear casting state when creature cast-time spells are implemented
            tracing::debug!("[AI] Creature {:?} interrupted spell", creature_guid);
        }

        AIAction::KillSelf => {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            world.managers.creature_mgr.apply_damage(
                creature_guid,
                u32::MAX,
                creature_guid,
                timestamp,
            );
            tracing::debug!("[AI] Creature {:?} killed self", creature_guid);
        }

        AIAction::SetFaction { faction_id } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.faction = faction_id;
                });
            tracing::debug!(
                "[AI] Creature {:?} faction set to {}",
                creature_guid,
                faction_id
            );
        }

        AIAction::SetImmune { physical, spell } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    if physical {
                        creature.unit_flags |= 0x00000100; // UNIT_FLAG_IMMUNE_TO_PC (physical approx)
                    } else {
                        creature.unit_flags &= !0x00000100;
                    }
                    if spell {
                        creature.unit_flags |= 0x00000200; // UNIT_FLAG_IMMUNE_TO_NPC (spell approx)
                    } else {
                        creature.unit_flags &= !0x00000200;
                    }
                });
            tracing::debug!(
                "[AI] Creature {:?} immune set: phys={}, spell={}",
                creature_guid,
                physical,
                spell
            );
        }

        AIAction::SetRoot { rooted } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    if rooted {
                        creature.move_spline.stop();
                    }
                });
            tracing::debug!("[AI] Creature {:?} root={}", creature_guid, rooted);
        }

        AIAction::SetHealthPercent { percent } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.current_health =
                        ((creature.max_health as f32) * percent.clamp(0.0, 1.0)) as u32;
                });
            send_health_update(world, creature_guid);
            tracing::debug!(
                "[AI] Creature {:?} health set to {}%",
                creature_guid,
                (percent * 100.0) as u32
            );
        }

        AIAction::Morph { display_id } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.display_id = display_id;
                });
            send_display_update(world, creature_guid, display_id);
            tracing::debug!(
                "[AI] Creature {:?} morphed to display {}",
                creature_guid,
                display_id
            );
        }

        AIAction::Demorph => {
            let native_display =
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(creature_guid, |creature| {
                        creature.display_id = creature.native_display_id;
                        creature.native_display_id
                    });
            if let Some(display_id) = native_display {
                send_display_update(world, creature_guid, display_id);
            }
            tracing::debug!("[AI] Creature {:?} demorphed", creature_guid);
        }

        AIAction::SpawnCreature {
            entry,
            position,
            summon_type: _,
            duration_ms: _,
        } => {
            // Spawn creature at position - uses creature manager
            tracing::debug!(
                "[AI] Creature {:?} spawning creature entry {} at ({}, {}, {})",
                creature_guid,
                entry,
                position.x,
                position.y,
                position.z
            );
            // TODO: Full spawn implementation - create creature from template, add to world
            // For now this is a stub that logs the intent
        }

        AIAction::DespawnCreature { guid } => {
            tracing::debug!("[AI] Despawning creature {:?}", guid);
            // TODO: Remove creature from world
        }

        AIAction::DespawnCreaturesByEntry { entry } => {
            tracing::debug!("[AI] Despawning all creatures with entry {}", entry);
            // TODO: Find and remove creatures by entry
        }

        AIAction::SpawnGameObject {
            entry,
            position,
            duration_secs,
        } => {
            tracing::debug!(
                "[AI] Spawning gameobject entry {} at ({}, {}, {}) for {}s",
                entry,
                position.x,
                position.y,
                position.z,
                duration_secs
            );
            // TODO: Full gameobject spawn implementation
        }

        AIAction::RespawnGameObject {
            guid,
            duration_secs,
        } => {
            tracing::debug!(
                "[AI] Respawning gameobject {:?} for {}s",
                guid,
                duration_secs
            );
            // TODO: Gameobject respawn implementation
        }

        AIAction::SetInstanceData { data_id, value } => {
            // Get creature's map/instance info to key the instance state
            if let Some((map_id, instance_id)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id))
            {
                world
                    .managers
                    .lua_mgr
                    .set_instance_data(map_id, instance_id, data_id, value);
                tracing::debug!(
                    "[AI] Set instance data: {} = {} (map={}, inst={})",
                    data_id,
                    value,
                    map_id,
                    instance_id
                );
            }
        }

        AIAction::SetInstanceGuid { data_id, guid } => {
            if let Some((map_id, instance_id)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id))
            {
                let mut state = world
                    .managers
                    .lua_mgr
                    .get_instance_state(map_id, instance_id);
                state.set_guid(data_id, guid);
                world
                    .managers
                    .lua_mgr
                    .set_instance_state(map_id, instance_id, state);
                tracing::debug!(
                    "[AI] Set instance GUID: {} = {:?} (map={}, inst={})",
                    data_id,
                    guid,
                    map_id,
                    instance_id
                );
            }
        }

        AIAction::OpenDoor { guid } => {
            tracing::debug!("[AI] Open door {:?}", guid);
            // TODO: Set gameobject state to open
        }

        AIAction::OpenDoorByData { data_id } => {
            tracing::debug!("[AI] Open door by data {}", data_id);
            // TODO: Look up GUID from instance data, then open
        }

        AIAction::CloseDoor { guid } => {
            tracing::debug!("[AI] Close door {:?}", guid);
            // TODO: Set gameobject state to closed
        }

        AIAction::CloseDoorByData { data_id } => {
            tracing::debug!("[AI] Close door by data {}", data_id);
            // TODO: Look up GUID from instance data, then close
        }

        // ==================== Phase 5: New Actions ====================
        AIAction::RemoveAura { spell_id } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.remove_aura(spell_id);
                });
            tracing::debug!("[AI] Remove aura {} from {:?}", spell_id, creature_guid);
        }

        AIAction::SetUnitFlag { flag } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.unit_flags |= flag;
                });
            tracing::debug!("[AI] Set unit flag 0x{:X} on {:?}", flag, creature_guid);
        }

        AIAction::RemoveUnitFlag { flag } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.unit_flags &= !flag;
                });
            tracing::debug!(
                "[AI] Remove unit flag 0x{:X} from {:?}",
                flag,
                creature_guid
            );
        }

        AIAction::SetStandState { state } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.stand_state = state;
                });
            tracing::debug!("[AI] SetStandState {} on {:?}", state, creature_guid);
        }

        AIAction::SetDynFlag { flag } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.dynamic_flags |= flag;
                });
            tracing::debug!("[AI] SetDynFlag 0x{:X} on {:?}", flag, creature_guid);
        }

        AIAction::RemoveDynFlag { flag } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.dynamic_flags &= !flag;
                });
            tracing::debug!("[AI] RemoveDynFlag 0x{:X} from {:?}", flag, creature_guid);
        }

        AIAction::SetCombatWithZone => {
            // Add all players on the same map to the creature's threat list
            if let Some(map_id) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| c.map_id)
            {
                // Collect player GUIDs on same map
                let mut player_guids = Vec::new();
                world.managers.player_mgr.for_each_player(|guid, player| {
                    if player.map_id == map_id && player.is_alive() {
                        player_guids.push(guid);
                    }
                });

                for player_guid in &player_guids {
                    // Add minimal threat so they're on the list
                    world
                        .managers
                        .creature_mgr
                        .with_creature_mut(creature_guid, |creature| {
                            creature.threat_manager.add_threat(*player_guid, 1.0);
                            if !creature.combat.in_combat {
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64;
                                creature.combat.enter_combat(*player_guid, now);
                            }
                        });
                }
                tracing::debug!(
                    "[AI] Set combat with zone: {} players engaged on map {}",
                    player_guids.len(),
                    map_id
                );
            }
        }

        AIAction::ScriptText { text_id } => {
            // TODO: Look up text from script_texts DB table by text_id
            // For now, log it
            tracing::debug!("[AI] Script text {} for {:?}", text_id, creature_guid);
        }

        // ==================== Phase 6: Path Movement ====================
        AIAction::MoveAlongPath {
            waypoints,
            movement_type,
            repeating,
        } => {
            use crate::world::game::creature::movement::generators::Waypoint;

            let wps: Vec<Waypoint> = waypoints
                .iter()
                .enumerate()
                .map(|(i, pos)| Waypoint {
                    point_id: i as u32,
                    position: *pos,
                    wait_time: 0,
                    script_id: 0,
                    orientation: None,
                })
                .collect();

            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let walk_speed = creature.walk_speed();
                    creature.motion_master.waypoint(
                        wps,
                        repeating,
                        creature_guid,
                        creature.position,
                        walk_speed,
                    );
                });
            tracing::debug!(
                "[AI] Move along path ({} waypoints, repeating={})",
                waypoints.len(),
                repeating
            );
        }

        AIAction::None => {
            // Explicit no-op
        }
    }
}

/// Send health update to nearby players
fn send_health_update(world: &World, creature_guid: ObjectGuid) {
    use crate::shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use crate::shared::messages::ToWorldPacket;
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::world::game::common::update_fields::UNIT_FIELD_HEALTH;

    if let Some((current_health, max_health)) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| (c.current_health, c.max_health))
    {
        // Build update block for health using the correct API
        let entry = creature_guid.entry();
        let world_guid = WorldObjectGuid::new_creature(entry, creature_guid.counter());

        let values_block = ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_field(UNIT_FIELD_HEALTH, current_health);

        let msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values_block));

        // Broadcast to nearby players (creatures don't have PlayerBroadcasters)
        let packet = msg.to_world_packet();
        broadcast_around_creature(world, creature_guid, &packet);

        tracing::debug!(
            "[AI] Sent health update for {:?}: {}/{}",
            creature_guid,
            current_health,
            max_health
        );
    }
}

/// Send SMSG_ATTACKSTART when creature enters combat
fn send_attack_start(world: &World, creature_guid: ObjectGuid, target_guid: ObjectGuid) {
    use crate::shared::messages::combat::SmsgAttackStart;
    use crate::shared::messages::ToWorldPacket;

    let packet = SmsgAttackStart {
        attacker_guid: creature_guid,
        target_guid,
    };

    // Broadcast to nearby players
    broadcast_around_creature(world, creature_guid, &packet.to_world_packet());

    tracing::debug!(
        "[AI] Sent SMSG_ATTACKSTART for creature {:?} attacking {:?}",
        creature_guid,
        target_guid
    );
}

/// Send VALUES update when creature enters combat (UNIT_FIELD_FLAGS + UNIT_FIELD_TARGET)
fn send_combat_state_update(world: &World, creature_guid: ObjectGuid, target_guid: ObjectGuid) {
    use crate::shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use crate::shared::messages::ToWorldPacket;
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::world::game::common::unit_flags;
    use crate::world::game::common::update_fields::{UNIT_FIELD_FLAGS, UNIT_FIELD_TARGET};

    if let Some(raw_flags) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| c.unit_flags)
    {
        // Strip blocking flags (same as build_create_msg in manager.rs)
        let mut flags = raw_flags;
        flags &= !(unit_flags::NOT_SELECTABLE
            | unit_flags::SPAWNING
            | unit_flags::NOT_ATTACKABLE
            | unit_flags::UNATTACKABLE
            | unit_flags::NOT_ATTACKABLE_1
            | unit_flags::IMMUNE_TO_PLAYER);

        let world_guid =
            WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
        let target_world_guid = WorldObjectGuid::from_raw(target_guid.raw());

        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
                .set_field(UNIT_FIELD_FLAGS, flags)
                .set_guid_field(UNIT_FIELD_TARGET, target_world_guid),
        ));
        broadcast_around_creature(world, creature_guid, &update.to_world_packet());

        tracing::debug!(
            "[AI] Combat state GUID check: creature_mgr_key={:?} (raw=0x{:016X}), world_guid=0x{:016X}, target_world_guid=0x{:016X}",
            creature_guid, creature_guid.raw(), world_guid.raw(), target_world_guid.raw()
        );
    }
}

/// Send VALUES update when creature leaves combat (clear IN_COMBAT flag + UNIT_FIELD_TARGET)
fn send_combat_exit_update(world: &World, creature_guid: ObjectGuid) {
    use crate::shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use crate::shared::messages::ToWorldPacket;
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::world::game::common::unit_flags;
    use crate::world::game::common::update_fields::{UNIT_FIELD_FLAGS, UNIT_FIELD_TARGET};

    if let Some(raw_flags) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| c.unit_flags)
    {
        // Strip blocking flags (same as build_create_msg in manager.rs)
        let mut flags = raw_flags;
        flags &= !(unit_flags::NOT_SELECTABLE
            | unit_flags::SPAWNING
            | unit_flags::NOT_ATTACKABLE
            | unit_flags::UNATTACKABLE
            | unit_flags::NOT_ATTACKABLE_1
            | unit_flags::IMMUNE_TO_PLAYER);

        let world_guid =
            WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());

        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
                .set_field(UNIT_FIELD_FLAGS, flags)
                .set_guid_field(UNIT_FIELD_TARGET, WorldObjectGuid::empty()),
        ));
        broadcast_around_creature(world, creature_guid, &update.to_world_packet());

        tracing::debug!(
            "[AI] Sent combat exit update for {:?}: flags=0x{:08X}",
            creature_guid,
            flags
        );
    }
}

/// Execute a creature spell cast (instant only for now).
///
/// Sends SMSG_SPELL_GO to nearby players, then applies spell effects via the
/// spell system's effect dispatcher. Creature cast-time spells will be added later.
pub fn execute_creature_spell_cast(
    world: &World,
    creature_guid: ObjectGuid,
    spell_id: u32,
    target_guid: Option<ObjectGuid>,
    triggered: bool,
) {
    use crate::shared::messages::spells::SmsgSpellGo;
    use crate::shared::messages::ToWorldPacket;

    // Validate spell exists
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(s) => s,
        None => {
            tracing::warn!(
                "[AI] Creature {:?} tried to cast unknown spell {}",
                creature_guid,
                spell_id
            );
            return;
        }
    };

    // Check mana cost
    if spell_entry.mana_cost > 0 {
        let has_mana = world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |creature| {
                if creature.current_mana >= spell_entry.mana_cost {
                    creature.current_mana -= spell_entry.mana_cost;
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if !has_mana {
            tracing::debug!(
                "[AI] Creature {:?} not enough mana for spell {}",
                creature_guid,
                spell_id
            );
            return;
        }
    }

    // Build hit target list
    let hit_targets: Vec<ObjectGuid> = target_guid.into_iter().collect();

    // Broadcast SMSG_SPELL_GO to nearby players
    let msg = SmsgSpellGo {
        caster_guid: creature_guid,
        caster_guid_pack: creature_guid,
        spell_id,
        cast_flags: if triggered { 0x0002 } else { 0x0000 },
        hit_targets,
        miss_targets: Vec::new(),
        target_guid,
        cast_item_guid: None,
    };
    broadcast_around_creature(world, creature_guid, &msg.to_world_packet());

    // Apply spell effects - handle common damage/heal effects inline
    for effect_idx in 0..3u8 {
        let effect_type = spell_entry.effect[effect_idx as usize];
        let base_value = spell_entry.effect_base_points[effect_idx as usize];

        match effect_type {
            2 => {
                // SPELL_EFFECT_SCHOOL_DAMAGE
                if let Some(target) = target_guid {
                    let damage = base_value.max(0) as u32;
                    let school = spell_entry.school as u8;
                    apply_creature_spell_damage(
                        world,
                        creature_guid,
                        target,
                        damage,
                        spell_id,
                        school,
                    );
                }
            }
            10 => {
                // SPELL_EFFECT_HEAL
                if let Some(target) = target_guid {
                    let heal_amount = base_value.max(0) as u32;
                    apply_creature_spell_heal(world, creature_guid, target, heal_amount, spell_id);
                }
            }
            _ => {
                // Other effects not yet handled for creature casters
                if effect_type != 0 {
                    tracing::debug!(
                        "[AI] Creature {:?} spell {} effect {} type {} not yet handled",
                        creature_guid,
                        spell_id,
                        effect_idx,
                        effect_type
                    );
                }
            }
        }
    }

    // Set cooldown in AI state
    if !triggered {
        let cooldown_ms = spell_entry
            .recovery_time
            .max(spell_entry.category_recovery_time);
        if cooldown_ms > 0 {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature
                        .ai_state_data
                        .set_spell_cooldown(spell_id, cooldown_ms);
                });
        }
    }

    tracing::debug!(
        "[AI] Creature {:?} cast spell {} on {:?} (triggered={})",
        creature_guid,
        spell_id,
        target_guid,
        triggered
    );
}

/// Apply spell damage from a creature to a target.
fn apply_creature_spell_damage(
    world: &World,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    damage: u32,
    spell_id: u32,
    school: u8,
) {
    if target_guid.is_player() {
        let died = world
            .systems
            .player
            .manager()
            .with_player_mut(target_guid, |player| {
                let current_health = player.stats.health;
                let new_health = current_health.saturating_sub(damage);
                player.stats.health = new_health;
                player.stats.dirty = true;
                tracing::debug!(
                    "Creature spell damage: {} took {} damage, health: {} -> {}",
                    player.name,
                    damage,
                    current_health,
                    new_health
                );
                new_health == 0 && current_health > 0
            })
            .unwrap_or(false);

        send_creature_spell_damage_log(world, caster_guid, target_guid, spell_id, damage, school);

        if died {
            if let Err(e) =
                world
                    .systems
                    .death
                    .on_killed(target_guid, Some(caster_guid), Some(spell_id), world)
            {
                tracing::error!("Failed to handle player death from creature spell: {}", e);
            }
        }
    } else if target_guid.is_creature() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        world
            .managers
            .creature_mgr
            .apply_damage(target_guid, damage, caster_guid, timestamp);
    }
}

/// Apply spell healing from a creature to a target.
fn apply_creature_spell_heal(
    world: &World,
    _caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    heal_amount: u32,
    _spell_id: u32,
) {
    if target_guid.is_creature() {
        world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |creature| {
                creature.current_health =
                    (creature.current_health + heal_amount).min(creature.max_health);
            });
        send_health_update(world, target_guid);
    }
}

/// Send SMSG_SPELLNONMELEEDAMAGELOG for creature spell damage.
fn send_creature_spell_damage_log(
    world: &World,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    damage: u32,
    school: u8,
) {
    use crate::shared::protocol::packet::WorldPacketGuidExt;
    use crate::shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_SPELLNONMELEEDAMAGELOG);
    packet.write_packed_guid(target_guid);
    packet.write_packed_guid(caster_guid);
    packet.write_u32(spell_id);
    packet.write_u32(damage); // damage
    packet.write_u8(school); // school
    packet.write_u32(0); // absorbed
    packet.write_u32(0); // resisted
    packet.write_u8(0); // periodic log (0 = no)
    packet.write_u8(0); // unused
    packet.write_u32(0); // blocked
    packet.write_u32(0); // hit_info flags
    packet.write_u8(0); // extend flag

    if target_guid.is_player() {
        world
            .managers
            .broadcast_mgr
            .broadcast_nearby(target_guid, &packet, true);
    } else {
        broadcast_around_creature(world, target_guid, &packet);
    }
}

/// Send SMSG_MESSAGECHAT for creature speech (Say, Yell, Emote text).
fn send_creature_chat(world: &World, creature_guid: ObjectGuid, chat_type: u8, text: &str) {
    use crate::shared::game::chat::ChatMsg;
    use crate::shared::messages::chat::SmsgMessageChat;
    use crate::shared::messages::ToWorldPacket;

    let name = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| c.name.clone())
        .unwrap_or_default();

    let msgtype = match chat_type {
        0x0B => ChatMsg::MonsterSay,
        0x0C => ChatMsg::MonsterYell,
        0x0D => ChatMsg::MonsterEmote,
        _ => ChatMsg::MonsterSay,
    };

    let packet = SmsgMessageChat {
        msgtype,
        language: crate::shared::game::chat::Language::Universal,
        sender_guid: creature_guid,
        sender_name: Some(&name),
        target_guid: None,
        channel_name: None,
        player_rank: None,
        message: text,
        chat_tag: crate::shared::game::chat::ChatTag::None,
    }
    .to_world_packet();

    broadcast_around_creature(world, creature_guid, &packet);
}

/// Send SMSG_EMOTE for creature emote animation.
fn send_creature_emote(world: &World, creature_guid: ObjectGuid, emote_id: u32) {
    use crate::shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_EMOTE);
    packet.write_u32(emote_id);
    packet.write_u64(creature_guid.raw());

    broadcast_around_creature(world, creature_guid, &packet);
}

/// Send SMSG_PLAY_OBJECT_SOUND for creature sounds.
fn send_creature_sound(world: &World, creature_guid: ObjectGuid, sound_id: u32) {
    use crate::shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_PLAY_OBJECT_SOUND);
    packet.write_u32(sound_id);
    packet.write_u64(creature_guid.raw());

    broadcast_around_creature(world, creature_guid, &packet);
}

/// Send display ID update to nearby players (for Morph/Demorph).
fn send_display_update(world: &World, creature_guid: ObjectGuid, display_id: u32) {
    use crate::shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use crate::shared::messages::ToWorldPacket;
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::world::game::common::update_fields::UNIT_FIELD_DISPLAYID;

    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());

    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_field(UNIT_FIELD_DISPLAYID, display_id),
    ));
    broadcast_around_creature(world, creature_guid, &update.to_world_packet());
}
