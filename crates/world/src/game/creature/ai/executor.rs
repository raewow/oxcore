//! AI Action Executor - Applies AI decisions to the game world
//!
//! This module executes the actions produced by AI decision functions.
//! Actions are applied in order with proper locking to maintain consistency.

use super::types::{AIAction, AIState, AIType};
use crate::game::broadcast_mgr::broadcast_around_creature;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;
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
            crate::game::combat::creature_attacks::perform_creature_melee_attack(
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
                    creature.unit_flags &= !crate::game::common::unit_flags::IN_COMBAT;
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
                        creature.unit_flags &= !crate::game::common::unit_flags::IN_COMBAT;

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
                    // Set the victim when combat starts. Without it
                    // the creature has no attack target to swing at, and UNIT_FIELD_TARGET
                    // stays empty for clients that see it enter combat.
                    if creature.combat.attacking.is_none() {
                        creature.combat.attacking = Some(target_guid);
                    }
                    creature.unit_flags |= crate::game::common::unit_flags::IN_COMBAT;

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
            let current_pos = world
                .managers
                .creature_mgr
                .get_position(creature_guid)
                .unwrap_or_default();

            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.ai_state = AIState::Fleeing;
                    creature.motion_master.fear(
                        flee_from_guid,
                        duration_ms,
                        distance,
                        creature_guid,
                        current_pos,
                        creature.run_speed(),
                    );
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
            use crate::game::creature::movement::generators::Waypoint;

            let wps: Vec<Waypoint> = waypoints
                .iter()
                .enumerate()
                .map(|(i, pos)| Waypoint {
                    point_id: i as u32,
                    position: *pos,
                    wait_time: 0,
                    wander_distance: 0.0,
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
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::game::common::update_fields::UNIT_FIELD_HEALTH;
    use oxcore_shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use oxcore_shared::messages::ToWorldPacket;

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
        let packet = msg;
        crate::game::broadcast_mgr::broadcast_around_creature(world, creature_guid, &packet);

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
    use oxcore_shared::messages::combat::SmsgAttackStart;
    use oxcore_shared::messages::ToWorldPacket;

    let packet = SmsgAttackStart {
        attacker_guid: creature_guid,
        target_guid,
    };

    // Broadcast to nearby players
    broadcast_around_creature(world, creature_guid, &packet);

    tracing::debug!(
        "[AI] Sent SMSG_ATTACKSTART for creature {:?} attacking {:?}",
        creature_guid,
        target_guid
    );
}

/// Send VALUES update when creature enters combat (UNIT_FIELD_FLAGS + UNIT_FIELD_TARGET)
fn send_combat_state_update(world: &World, creature_guid: ObjectGuid, target_guid: ObjectGuid) {
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::game::common::unit_flags;
    use crate::game::common::update_fields::{UNIT_FIELD_FLAGS, UNIT_FIELD_TARGET};
    use oxcore_shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use oxcore_shared::messages::ToWorldPacket;

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
        broadcast_around_creature(world, creature_guid, &update);

        tracing::debug!(
            "[AI] Combat state GUID check: creature_mgr_key={:?} (raw=0x{:016X}), world_guid=0x{:016X}, target_world_guid=0x{:016X}",
            creature_guid, creature_guid.raw(), world_guid.raw(), target_world_guid.raw()
        );
    }
}

/// Send VALUES update when creature leaves combat (clear IN_COMBAT flag + UNIT_FIELD_TARGET)
fn send_combat_exit_update(world: &World, creature_guid: ObjectGuid) {
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::game::common::unit_flags;
    use crate::game::common::update_fields::{UNIT_FIELD_FLAGS, UNIT_FIELD_TARGET};
    use oxcore_shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use oxcore_shared::messages::ToWorldPacket;

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
        broadcast_around_creature(world, creature_guid, &update);

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
    use oxcore_shared::messages::spells::SmsgSpellGo;
    use oxcore_shared::messages::ToWorldPacket;

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
        ammo_display_id: 0,
        ammo_inventory_type: 0,
    };
    broadcast_around_creature(world, creature_guid, &msg);

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

/// ScriptedAI::DoCastSpell - Cast a spell from scripted AI.
///
/// If the creature is currently casting a non-melee spell and this is not
/// a triggered cast, the cast is skipped. (Non-melee spell tracking is not
/// yet implemented for creatures.)
pub fn scripted_do_cast_spell(
    world: &World,
    creature_guid: ObjectGuid,
    target_guid: Option<ObjectGuid>,
    spell_id: u32,
    triggered: bool,
) {
    // ... non-melee spell check skipped — creature casting state not tracked yet
    execute_creature_spell_cast(world, creature_guid, spell_id, target_guid, triggered);
}

/// CreatureAI::DoCastSpellIfCan - Cast a spell if cooldown allows.
///
/// Checks cooldown via the creature's AI state data, then delegates to
/// execute_creature_spell_cast. Returns true if the cast was initiated.
/// The cast_flags parameter: bit 0 = CF_TRIGGERED.
/// Other TryToCast checks (LOS, range, movement, immunity) are not yet
/// implemented for creature casters.
pub fn do_cast_spell_if_can(
    world: &World,
    creature_guid: ObjectGuid,
    target_guid: Option<ObjectGuid>,
    spell_id: u32,
    cast_flags: u32,
) -> bool {
    if world.managers.spell_mgr.get(spell_id).is_none() {
        return false;
    }

    let ready = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |creature| {
            creature.ai_state_data.is_spell_ready(spell_id)
        })
        .unwrap_or(false);

    if !ready {
        return false;
    }

    let triggered = (cast_flags & 0x1) != 0;
    execute_creature_spell_cast(world, creature_guid, spell_id, target_guid, triggered);
    true
}

/// CreatureAI::SetSpellsList(uint32 entry) — load a creature_spells template by ID.
///
/// Clears the list when entry is 0. Otherwise logs a warning — loading from the
/// creature_spells DB table is not yet implemented (no Rust repository exists).
/// Callers that have a pre-loaded list can use `set_spells_list` on AIStateData directly.
pub fn set_spells_list_from_db(world: &World, creature_guid: ObjectGuid, entry: u32) {
    if entry == 0 {
        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |c| {
                c.ai_state_data.clear_spells_list();
            });
        return;
    }
    // ... CreatureSpellsList loading from DB not yet implemented
    tracing::warn!(
        "[AI] Creature {:?} tried to load creature_spells entry {} — not yet implemented",
        creature_guid,
        entry
    );
}

/// CreatureAI::UpdateSpellsList — ticks the casting delay and fires DoSpellsListCasts
/// when the delay expires. Matches the 1.2s research-based update interval.
pub fn update_spells_list(world: &World, creature_guid: ObjectGuid, diff_ms: u32) {
    const CREATURE_CASTING_DELAY: u32 = 1200;

    let cast_elapsed_ms = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| {
            let data = &mut c.ai_state_data;
            if data.casting_delay <= diff_ms {
                let desync = diff_ms - data.casting_delay;
                data.casting_delay = if desync < CREATURE_CASTING_DELAY {
                    CREATURE_CASTING_DELAY - desync
                } else {
                    0
                };
                Some(CREATURE_CASTING_DELAY.saturating_add(desync))
            } else {
                data.casting_delay -= diff_ms;
                None
            }
        })
        .flatten();

    if let Some(elapsed_ms) = cast_elapsed_ms {
        do_spells_list_casts(world, creature_guid, elapsed_ms);
    }
}

/// CreatureAI::DoSpellsListCasts — iterate the creature's spell list, decrement
/// cooldowns, and cast spells whose cooldown has expired.
///
/// Casting result handling is approximated: only SPELL_CAST_OK resets the cooldown;
/// other results (fleeing, in-progress, try-again, etc.) leave cooldown at 0 to
/// retry next tick. Target resolution via castTarget/GetTargetByType is not yet
/// implemented — uses the creature's current attack target.
pub fn do_spells_list_casts(world: &World, creature_guid: ObjectGuid, diff_ms: u32) {
    // Gather the spells list and current attack target
    let (spells, attack_target) = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |c| {
            (c.ai_state_data.spells_list.clone(), c.combat.attacking)
        })
        .unwrap_or_default();

    if spells.is_empty() {
        return;
    }

    let mut spells_to_update: Vec<(usize, u32)> = Vec::new();
    let mut dont_cast = false;

    for (idx, spell) in spells.iter().enumerate() {
        if spell.cooldown <= diff_ms {
            // Cooldown expired
            let cooldown = 0u32;

            // Skip non-triggered spells if already cast this tick or creature is casting
            let is_triggered = (spell.inner.cast_flags & 0x2) != 0;
            let interrupt_prev = (spell.inner.cast_flags & 0x1) != 0;
            if !(is_triggered || interrupt_prev) {
                if dont_cast {
                    spells_to_update.push((idx, spell.cooldown.saturating_sub(diff_ms)));
                    continue;
                }
                // ... IsNonMeleeSpellCasted check skipped — not tracked for creatures yet
            }

            let target = attack_target;
            let spell_id = spell.inner.spell_id as u32;

            // Attempt cast via execute_creature_spell_cast
            // ... TryToCast result handling approximated (only OK resets cooldown)
            let triggered = is_triggered;
            if world.managers.spell_mgr.get(spell_id).is_some() {
                execute_creature_spell_cast(world, creature_guid, spell_id, target, triggered);
                // Cast succeeded
                dont_cast = !triggered;
                let new_cooldown = if spell.inner.delay_repeat_max > spell.inner.delay_repeat_min {
                    rand::thread_rng()
                        .gen_range(spell.inner.delay_repeat_min..=spell.inner.delay_repeat_max)
                } else {
                    spell.inner.delay_repeat_min
                };
                spells_to_update.push((idx, new_cooldown));
            } else {
                // Unknown spell — keep cooldown at 0 for next tick
                spells_to_update.push((idx, 0));
            }
        } else {
            spells_to_update.push((idx, spell.cooldown.saturating_sub(diff_ms)));
        }
    }

    // Apply cooldown updates
    world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| {
            for (idx, cd) in spells_to_update {
                if let Some(entry) = c.ai_state_data.spells_list.get_mut(idx) {
                    entry.cooldown = cd;
                }
            }
        });
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
    use oxcore_shared::protocol::packet::WorldPacketGuidExt;
    use oxcore_shared::protocol::{Opcode, WorldPacket};

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
            .broadcast_msg_nearby(target_guid, &packet, true);
    } else {
        crate::game::broadcast_mgr::broadcast_around_creature(world, target_guid, &packet);
    }
}

/// Send SMSG_MESSAGECHAT for creature speech (Say, Yell, Emote text).
fn send_creature_chat(world: &World, creature_guid: ObjectGuid, chat_type: u8, text: &str) {
    use oxcore_shared::game::chat::ChatMsg;
    use oxcore_shared::messages::chat::SmsgMessageChat;
    use oxcore_shared::messages::ToWorldPacket;

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

    let msg = SmsgMessageChat {
        msgtype,
        language: oxcore_shared::game::chat::Language::Universal,
        sender_guid: creature_guid,
        sender_name: Some(&name),
        target_guid: None,
        channel_name: None,
        player_rank: None,
        message: text,
        chat_tag: oxcore_shared::game::chat::ChatTag::None,
    };

    broadcast_around_creature(world, creature_guid, &msg);
}

/// Send SMSG_EMOTE for creature emote animation.
fn send_creature_emote(world: &World, creature_guid: ObjectGuid, emote_id: u32) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_EMOTE);
    packet.write_u32(emote_id);
    packet.write_u64(creature_guid.raw());

    crate::game::broadcast_mgr::broadcast_around_creature(world, creature_guid, &packet);
}

/// Send SMSG_PLAY_OBJECT_SOUND for creature sounds.
fn send_creature_sound(world: &World, creature_guid: ObjectGuid, sound_id: u32) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_PLAY_OBJECT_SOUND);
    packet.write_u32(sound_id);
    packet.write_u64(creature_guid.raw());

    crate::game::broadcast_mgr::broadcast_around_creature(world, creature_guid, &packet);
}

/// Send display ID update to nearby players (for Morph/Demorph).
fn send_display_update(world: &World, creature_guid: ObjectGuid, display_id: u32) {
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::game::common::update_fields::UNIT_FIELD_DISPLAYID;
    use oxcore_shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use oxcore_shared::messages::ToWorldPacket;

    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());

    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_field(UNIT_FIELD_DISPLAYID, display_id),
    ));
    broadcast_around_creature(world, creature_guid, &update);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::dbc::structures::SpellEntry;
    use crate::game::creature::ai::types::AIState;
    use crate::game::creature::Creature;
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

    fn test_spell(id: u32) -> SpellEntry {
        SpellEntry {
            id,
            name: format!("TestSpell{id}"),
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
            spell_family_name: 0,
            spell_family_flags: 0,
            max_affected_targets: 0,
            dmg_class: 0,
            prevention_type: 0,
            custom: 0,
            internal: 0,
            allowed_target_mask: 0,
            script_id: 0,
            dmg_multiplier: [1.0; 3],
        }
    }

    fn test_creature(guid: ObjectGuid) -> Creature {
        Creature {
            guid,
            entry: guid.entry(),
            spawn_id: 1,
            position: oxcore_shared::protocol::Position::default(),
            home_position: oxcore_shared::protocol::Position::default(),
            map_id: 0,
            instance_id: 0,
            display_id: 0,
            native_display_id: 0,
            scale: 1.0,
            bounding_radius: 0.5,
            combat_reach: 1.5,
            level: 1,
            max_health: 100,
            current_health: 100,
            max_mana: 100,
            current_mana: 100,
            faction: 0,
            unit_flags: 0,
            dynamic_flags: 0,
            stand_state: 0,
            npc_flags: 0,
            armor: 0,
            damage_min: 0,
            damage_max: 0,
            attack_power: 0,
            name: String::from("Test Creature"),
            creature_type: 0,
            static_flags1: 0,
            spells: [0; 4],
            phase_mask: 1,
            in_world: true,
            combat: crate::game::creature::combat::CombatState::new(),
            threat_manager: crate::game::creature::combat::ThreatManager::new(guid),
            charm_guid: None,
            owner_guid: None,
            attack_timer: 0,
            base_attack_time: 2000,
            regen_timer: 0,
            death_state: crate::game::creature::death::DeathState::Alive,
            corpse_decay_timer: 0,
            respawn_time: 0,
            loot_recipient: None,
            has_loot: false,
            ai_state: AIState::Idle,
            ai_state_data: crate::game::creature::ai::AIStateData::new(),
            auras: crate::game::player::auras::AuraContainer::new(),
            motion_master: crate::game::creature::movement::MotionMaster::new(),
            move_spline: crate::game::creature::movement::MoveSpline::default(),
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
            movement_paused: false,
            movement_info: crate::core::common::movement::MovementInfo::new(),
            following_target: None,
            followers: Vec::new(),
        }
    }

    #[tokio::test]
    async fn scripted_do_cast_spell_skips_unknown_spell() {
        let world = test_world();
        let guid = ObjectGuid::new_creature(1, 1);
        let target = ObjectGuid::new_player(1);

        scripted_do_cast_spell(&world, guid, Some(target), 99999, false);
    }

    #[tokio::test]
    async fn scripted_do_cast_spell_with_known_spell() {
        let world = test_world();
        let spell = test_spell(133);
        world.managers.spell_mgr.add_spell(spell);

        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        scripted_do_cast_spell(&world, creature_guid, Some(creature_guid), 133, false);
    }

    #[tokio::test]
    async fn do_cast_spell_if_can_returns_true_when_ready() {
        let world = test_world();
        let spell = test_spell(200);
        world.managers.spell_mgr.add_spell(spell);

        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        let result = do_cast_spell_if_can(&world, creature_guid, Some(creature_guid), 200, 0);
        assert!(result, "cast should succeed when spell is ready");
    }

    #[tokio::test]
    async fn do_cast_spell_if_can_returns_false_on_cooldown() {
        let world = test_world();
        let mut spell = test_spell(201);
        spell.recovery_time = 5000;
        world.managers.spell_mgr.add_spell(spell);

        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        // First cast succeeds
        assert!(do_cast_spell_if_can(
            &world,
            creature_guid,
            Some(creature_guid),
            201,
            0
        ));

        // Spell is now on cooldown (set by execute_creature_spell_cast),
        // so second cast should fail immediately.
        let result = do_cast_spell_if_can(&world, creature_guid, Some(creature_guid), 201, 0);
        assert!(!result, "cast should fail when spell is on cooldown");
    }

    #[tokio::test]
    async fn do_cast_spell_if_can_returns_false_for_unknown_spell() {
        let world = test_world();
        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        let result = do_cast_spell_if_can(&world, creature_guid, Some(creature_guid), 99999, 0);
        assert!(!result, "cast should fail for unknown spell");
    }

    #[tokio::test]
    async fn do_cast_spell_if_can_with_triggered_flag() {
        let world = test_world();
        let mut spell = test_spell(202);
        spell.recovery_time = 5000;
        world.managers.spell_mgr.add_spell(spell);

        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        // Cast with triggered flag succeeds
        let result = do_cast_spell_if_can(
            &world,
            creature_guid,
            Some(creature_guid),
            202,
            0x1, // CF_TRIGGERED
        );
        assert!(result, "triggered cast should succeed when ready");
    }

    // ── CreatureAI spell list helpers ──────────────────────────────────────

    #[tokio::test]
    async fn update_spells_list_ticks_delay_and_fires_casts() {
        let world = test_world();
        let spell_entry = test_spell(300);
        world.managers.spell_mgr.add_spell(spell_entry);

        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        // Set a spell list with one entry
        let db_entry = super::super::types::CreatureSpellsEntry {
            spell_id: 300,
            probability: 100,
            cast_target: 0,
            target_param1: 0,
            target_param2: 0,
            cast_flags: 0,
            delay_initial_min: 0,
            delay_initial_max: 0,
            delay_repeat_min: 1000,
            delay_repeat_max: 1000,
            script_id: 0,
        };

        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |c| {
                c.ai_state_data.set_spells_list(vec![db_entry]);
            });

        // First update should tick delay and cast (initial delay is 0)
        update_spells_list(&world, creature_guid, 1200);

        // After casting, cooldown should be reset to delay_repeat_min
        let spell_cd = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| {
                c.ai_state_data.spells_list.first().map(|s| s.cooldown)
            })
            .flatten();
        assert_eq!(spell_cd, Some(1000), "cooldown should be reset after cast");
    }

    #[tokio::test]
    async fn update_spells_list_does_not_cast_before_delay_expires() {
        let world = test_world();
        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        // Set a high casting delay
        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |c| {
                c.ai_state_data.casting_delay = 5000;
            });

        // Update with a small diff — delay should decrease but not fire
        update_spells_list(&world, creature_guid, 100);

        let delay = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| c.ai_state_data.casting_delay)
            .unwrap_or(0);
        assert_eq!(delay, 4900, "casting delay should decrease by diff");
    }

    #[tokio::test]
    async fn update_spells_list_includes_desync_when_ticking_spell_cooldowns() {
        let world = test_world();
        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));
        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |c| {
                c.ai_state_data
                    .set_spells_list(vec![super::super::types::CreatureSpellsEntry {
                        spell_id: 999,
                        delay_initial_min: 2_000,
                        delay_initial_max: 2_000,
                        ..Default::default()
                    }]);
                c.ai_state_data.casting_delay = 100;
            });

        // The 300ms update exceeds the 100ms delay by 200ms, so the spell-list
        // cooldown sees the normal 1200ms interval plus the 200ms desync.
        update_spells_list(&world, creature_guid, 300);

        let cooldown = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| c.ai_state_data.spells_list[0].cooldown);
        assert_eq!(cooldown, Some(600));
    }

    #[tokio::test]
    async fn set_spells_list_from_db_clears_on_zero_entry() {
        let world = test_world();
        let creature_guid = ObjectGuid::new_creature(1, 1);
        world
            .managers
            .creature_mgr
            .add_creature(test_creature(creature_guid));

        // First set some spells
        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |c| {
                c.ai_state_data
                    .set_spells_list(vec![super::super::types::CreatureSpellsEntry {
                        spell_id: 1,
                        ..Default::default()
                    }]);
            });

        // Clear with entry=0
        set_spells_list_from_db(&world, creature_guid, 0);

        let len = world
            .managers
            .creature_mgr
            .with_creature(creature_guid, |c| c.ai_state_data.spells_list.len())
            .unwrap_or(0);
        assert_eq!(len, 0, "spells list should be cleared");
    }
}
