//! AI Action Executor - Applies AI decisions to the game world
//!
//! This module executes the actions produced by AI decision functions.
//! Actions are applied in order with proper locking to maintain consistency.

use super::types::{AIAction, AIState, MovementType, ReactState};
use crate::game::broadcast_mgr::broadcast_around_creature;
use crate::game::creature::movement::MovementGeneratorType;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

/// Execute a batch of AI actions for a creature
/// Actions are applied in order with proper locking
/// A guid the creature's dungeon copy stored under `data_id` (doors, bosses).
fn instance_guid(world: &World, creature_guid: ObjectGuid, data_id: u32) -> Option<ObjectGuid> {
    let (map_id, instance_id) = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |c| (c.map_id, c.instance_id))?;
    world
        .managers
        .lua_mgr
        .get_instance_state(map_id, instance_id)
        .get_guid(data_id)
}

pub fn execute_actions(world: &World, creature_guid: ObjectGuid, actions: Vec<AIAction>) {
    for action in actions {
        execute_single_action(world, creature_guid, action);
    }
}

fn execute_single_action(world: &World, creature_guid: ObjectGuid, action: AIAction) {
    match action {
        AIAction::SetAttackTarget { target_guid } => switch_target(world, creature_guid, target_guid),

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
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let position = creature.position;
                    let combat_reach = creature.combat_reach;
                    let run_speed = creature.run_speed();
                    creature.motion_master.chase(
                        target_guid,
                        creature_guid,
                        position,
                        combat_reach,
                        run_speed,
                    );
                });
        }

        AIAction::MoveTo {
            position,
            movement_type,
        } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let current = creature.position;
                    let walk = movement_type == MovementType::Walk;
                    let speed = if walk {
                        creature.walk_speed()
                    } else {
                        creature.run_speed()
                    };
                    creature.motion_master.move_point(
                        0,
                        position,
                        u32::from(walk),
                        speed,
                        position.o,
                        creature_guid,
                        current,
                    );
                });
        }

        AIAction::ReturnToSpawn => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    if creature.motion_master.active_generator() != MovementGeneratorType::Home {
                        let position = creature.position;
                        let home = creature.home_position;
                        let run_speed = creature.run_speed();
                        creature
                            .motion_master
                            .return_home(home, creature_guid, position, run_speed);
                    }
                    creature.ai_state = AIState::Returning;
                });
        }

        AIAction::ReachedHome => reached_home(world, creature_guid),

        AIAction::EnterEvadeMode => enter_evade_mode(world, creature_guid),

        AIAction::LeaveCombat => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.ai_state = AIState::Idle;
                    creature.combat.leave_combat();
                    creature.threat_manager.clear();
                    creature.unit_flags &= !crate::game::common::unit_flags::IN_COMBAT;
                    creature.ai_state_data.reset_combat();
                    creature
                        .motion_master
                        .clear_type(MovementGeneratorType::Chase, creature_guid);
                    creature
                        .motion_master
                        .reinitialize_patrol_movement(creature_guid);
                });
            send_combat_exit_update(world, creature_guid);
        }

        AIAction::EnterCombat { target_guid } => attack_start(world, creature_guid, target_guid),

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
            world
                .systems
                .creature_movement
                .send_stop_packet(creature_guid, current_pos, world);
        }

        AIAction::FleeFrom {
            flee_from_guid,
            distance,
            duration_ms,
        } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    let position = creature.position;
                    let run_speed = creature.run_speed();
                    creature.ai_state = AIState::Fleeing;
                    creature.motion_master.fear(
                        flee_from_guid,
                        duration_ms,
                        distance,
                        creature_guid,
                        position,
                        run_speed,
                    );
                });
        }

        AIAction::FaceTarget { target_guid } => face_target(world, creature_guid, target_guid),

        AIAction::RandomMovement { wander_distance } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.wander_distance = wander_distance;
                    creature
                        .motion_master
                        .reinitialize_patrol_movement(creature_guid);
                });
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
                    creature.threat_manager.add_threat(target_guid, amount);
                    creature.combat.add_threat(target_guid, amount, 0);
                    if creature.ai_state == AIState::Combat {
                        creature.combat.attackers.insert(target_guid);
                    }
                });
            if amount == 0.0 {
                put_in_combat_with(world, target_guid, creature_guid);
            }
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
                    let current = creature.threat_manager.get_threat(target_guid);
                    let new_threat = if is_percent { current * amount } else { amount };
                    creature
                        .threat_manager
                        .modify_threat(target_guid, new_threat - current);
                    if let Some(entry) = creature
                        .combat
                        .threat_list
                        .iter_mut()
                        .find(|e| e.guid == target_guid)
                    {
                        entry.threat = new_threat.max(0.0);
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

        AIAction::SetMeleeAttack { enabled } => set_melee_attack(world, creature_guid, enabled),

        AIAction::SetCombatMovement { enabled } => {
            set_combat_movement(world, creature_guid, enabled)
        }

        AIAction::SetReactState { react_state } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.ai_state_data.react_state = react_state;
                });
        }

        AIAction::CallForHelp { radius } => {
            super::assist::call_for_help(world, creature_guid, radius);
        }

        AIAction::SummonGuards { guard_entry, count } => {
            let Some((map_id, instance_id, position)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id, c.position))
            else {
                return;
            };
            for _ in 0..count {
                crate::game::creature::summon::summon_creature(
                    world,
                    Some(creature_guid),
                    guard_entry,
                    map_id,
                    instance_id,
                    position,
                    crate::game::creature::summon::SummonDespawn::TimedOutOfCombat,
                    60_000,
                );
            }
        }

        AIAction::Say { text } => {
            send_creature_chat(world, creature_guid, 0x0B, &text); // CHAT_MSG_MONSTER_SAY
        }

        AIAction::Yell { text } => {
            send_creature_chat(world, creature_guid, 0x0C, &text); // CHAT_MSG_MONSTER_YELL
        }

        AIAction::PlayEmote { emote_id } => {
            send_creature_emote(world, creature_guid, emote_id);
        }

        AIAction::TextEmote { text } => {
            send_creature_chat(world, creature_guid, 0x0D, &text); // CHAT_MSG_MONSTER_EMOTE
        }

        AIAction::PlaySound {
            sound_id,
            zone_wide: _,
        } => {
            send_creature_sound(world, creature_guid, sound_id);
        }

        AIAction::InterruptSpell => {
            // Creature casts complete instantly (spell review F34); there is no
            // cast in progress to interrupt.
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
        }

        AIAction::SetFaction { faction_id } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.faction = faction_id;
                });
            send_unit_field(
                world,
                creature_guid,
                crate::game::common::update_fields::UNIT_FIELD_FACTIONTEMPLATE,
                faction_id,
            );
        }

        AIAction::SetImmune { physical, spell } => {
            use crate::game::common::unit_flags::{IMMUNE_TO_NPC, IMMUNE_TO_PLAYER};
            let flags = world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    if physical {
                        creature.unit_flags |= IMMUNE_TO_PLAYER;
                    } else {
                        creature.unit_flags &= !IMMUNE_TO_PLAYER;
                    }
                    if spell {
                        creature.unit_flags |= IMMUNE_TO_NPC;
                    } else {
                        creature.unit_flags &= !IMMUNE_TO_NPC;
                    }
                    creature.unit_flags
                });
            if let Some(flags) = flags {
                send_unit_field(
                    world,
                    creature_guid,
                    crate::game::common::update_fields::UNIT_FIELD_FLAGS,
                    flags,
                );
            }
        }

        AIAction::SetRoot { rooted } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    if rooted {
                        creature.move_spline.stop();
                        creature.motion_master.move_idle(creature_guid);
                        creature.ai_state_data.combat_movement_enabled = false;
                    } else {
                        creature.ai_state_data.combat_movement_enabled = creature.static_flags1
                            & crate::game::common::creature_flags::CREATURE_STATIC_FLAG_SESSILE
                            == 0;
                    }
                });
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
        }

        AIAction::Morph { display_id } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.display_id = display_id;
                });
            send_display_update(world, creature_guid, display_id);
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
        }

        AIAction::SpawnCreature {
            entry,
            position,
            summon_type,
            duration_ms,
        } => {
            let Some((map_id, instance_id)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id))
            else {
                return;
            };
            crate::game::creature::summon::summon_creature(
                world,
                Some(creature_guid),
                entry,
                map_id,
                instance_id,
                position,
                crate::game::creature::summon::SummonDespawn::from_script_code(summon_type),
                duration_ms,
            );
        }

        AIAction::DespawnCreature { guid } => {
            crate::game::creature::summon::despawn_creature(world, guid);
        }

        AIAction::DespawnCreaturesByEntry { entry } => {
            let Some((map_id, instance_id)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id))
            else {
                return;
            };
            let targets: Vec<ObjectGuid> = world
                .managers
                .creature_mgr
                .iter_creatures()
                .filter(|e| {
                    let c = e.value();
                    c.entry == entry && c.map_id == map_id && c.instance_id == instance_id
                })
                .map(|e| *e.key())
                .collect();
            for guid in targets {
                crate::game::creature::summon::despawn_creature(world, guid);
            }
        }

        AIAction::SpawnGameObject {
            entry,
            position,
            duration_secs,
        } => {
            let Some((map_id, instance_id)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id))
            else {
                return;
            };
            crate::game::creature::summon::summon_gameobject(
                world,
                entry,
                map_id,
                instance_id,
                position,
                duration_secs,
            );
        }

        AIAction::RespawnGameObject {
            guid,
            duration_secs,
        } => {
            crate::game::creature::summon::respawn_gameobject(world, guid, duration_secs);
        }

        AIAction::SetInstanceData { data_id, value } => {
            if let Some((map_id, instance_id)) = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| (c.map_id, c.instance_id))
            {
                world
                    .managers
                    .lua_mgr
                    .set_instance_data(map_id, instance_id, data_id, value);
                crate::game::instance::scripts::queue_save(world, map_id, instance_id);
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
            }
        }

        AIAction::OpenDoor { guid } => {
            crate::game::instance::scripts::set_door(world, guid, true);
        }

        AIAction::OpenDoorByData { data_id } => {
            if let Some(guid) = instance_guid(world, creature_guid, data_id) {
                crate::game::instance::scripts::set_door(world, guid, true);
            }
        }

        AIAction::CloseDoor { guid } => {
            crate::game::instance::scripts::set_door(world, guid, false);
        }

        AIAction::CloseDoorByData { data_id } => {
            if let Some(guid) = instance_guid(world, creature_guid, data_id) {
                crate::game::instance::scripts::set_door(world, guid, false);
            }
        }

        AIAction::RemoveAura { spell_id } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.remove_aura(spell_id);
                });
        }

        AIAction::SetUnitFlag { flag } => {
            let flags = world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.unit_flags |= flag;
                    creature.unit_flags
                });
            if let Some(flags) = flags {
                send_unit_field(
                    world,
                    creature_guid,
                    crate::game::common::update_fields::UNIT_FIELD_FLAGS,
                    flags,
                );
            }
        }

        AIAction::RemoveUnitFlag { flag } => {
            let flags = world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.unit_flags &= !flag;
                    creature.unit_flags
                });
            if let Some(flags) = flags {
                send_unit_field(
                    world,
                    creature_guid,
                    crate::game::common::update_fields::UNIT_FIELD_FLAGS,
                    flags,
                );
            }
        }

        AIAction::SetStandState { state } => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.stand_state = state;
                });
            send_unit_field(
                world,
                creature_guid,
                crate::game::common::update_fields::UNIT_FIELD_BYTES_1,
                state as u32,
            );
        }

        AIAction::SetDynFlag { flag } => {
            let flags = world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.dynamic_flags |= flag;
                    creature.dynamic_flags
                });
            if let Some(flags) = flags {
                send_unit_field(
                    world,
                    creature_guid,
                    crate::game::common::update_fields::UNIT_DYNAMIC_FLAGS,
                    flags,
                );
            }
        }

        AIAction::RemoveDynFlag { flag } => {
            let flags = world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.dynamic_flags &= !flag;
                    creature.dynamic_flags
                });
            if let Some(flags) = flags {
                send_unit_field(
                    world,
                    creature_guid,
                    crate::game::common::update_fields::UNIT_DYNAMIC_FLAGS,
                    flags,
                );
            }
        }

        AIAction::SetCombatWithZone => {
            super::assist::set_in_combat_with_zone(world, creature_guid, true);
        }

        AIAction::ScriptText { text_id } => script_text(world, creature_guid, text_id),

        AIAction::MoveAlongPath {
            waypoints,
            movement_type: _,
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
        }

        AIAction::ResetCritterEscape => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.ai_state_data.critter_escape_ms = super::decision::ESCAPE_TIMER_MS;
                });
        }

        AIAction::PetFollowOwner => {
            world
                .managers
                .creature_mgr
                .with_creature_mut(creature_guid, |creature| {
                    creature.ai_state = AIState::Idle;
                    creature.combat.leave_combat();
                    creature.threat_manager.clear();
                    creature.unit_flags &= !crate::game::common::unit_flags::IN_COMBAT;
                    creature.ai_state_data.reset_combat();
                    creature
                        .motion_master
                        .clear_type(MovementGeneratorType::Chase, creature_guid);
                    if let Some(owner) = creature.owner_guid {
                        let position = creature.position;
                        let speed = creature.run_speed();
                        creature.motion_master.move_follow(
                            owner,
                            2.0,
                            std::f32::consts::FRAC_PI_2,
                            creature_guid,
                            position,
                            speed,
                        );
                        creature.following_target = Some(owner);
                    }
                });
            send_combat_exit_update(world, creature_guid);
        }

        AIAction::None => {}
    }
}

/// Start attacking `target_guid`.
///
/// A passive creature never attacks. Attacking puts both sides in combat, adds
/// the target to the threat list, starts the chase unless combat movement is
/// off, and on the first attack of a fight records where the fight started,
/// starts the leash clock, calls nearby creatures of the same faction for
/// assistance and, for raid bosses, pulls the whole dungeon into combat.
fn attack_start(world: &World, creature_guid: ObjectGuid, target_guid: ObjectGuid) {
    let now = super::units::now_ms();
    let Some(outcome) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            if !creature.is_alive() || creature.ai_state_data.react_state == ReactState::Passive {
                return None;
            }
            let fresh = creature.ai_state != AIState::Combat;
            let position = creature.position;
            creature.ai_state = AIState::Combat;
            creature.combat.in_combat = true;
            creature.combat.attackers.insert(target_guid);
            creature.combat.attacking = Some(target_guid);
            creature.threat_manager.add_threat(target_guid, 0.0);
            creature.unit_flags |= crate::game::common::unit_flags::IN_COMBAT;
            if fresh {
                creature.ai_state_data.combat_start = Some(position);
                creature.ai_state_data.extend_leash(now);
                creature.ai_state_data.call_for_help_timer = 0;
                creature.ai_state_data.combat_pulse_timer = 0;
            }

            creature.move_spline.stop();
            if creature.ai_state_data.combat_movement_enabled {
                let combat_reach = creature.combat_reach;
                let run_speed = creature.run_speed();
                creature.motion_master.chase(
                    target_guid,
                    creature_guid,
                    position,
                    combat_reach,
                    run_speed,
                );
            }
            Some((
                fresh,
                position,
                creature.ai_state_data.melee_enabled,
                creature.ai_profile.static_flags2
                    & crate::game::common::creature_flags::CREATURE_STATIC_FLAG_2_FORCE_RAID_COMBAT
                    != 0,
            ))
        })
        .flatten()
    else {
        return;
    };
    let (fresh, position, melee, force_raid_combat) = outcome;

    world
        .systems
        .creature_movement
        .send_stop_packet(creature_guid, position, world);
    if melee {
        send_attack_start(world, creature_guid, target_guid);
    }
    send_combat_state_update(world, creature_guid, target_guid);
    put_in_combat_with(world, target_guid, creature_guid);

    if fresh {
        super::system::queue_event(
            world,
            creature_guid,
            super::types::AIEvent::CombatStarted {
                initial_aggressor: target_guid,
            },
        );
        crate::game::instance::scripts::queue_creature_combat_event(
            world,
            creature_guid,
            crate::game::instance::scripts::CreatureCombatEvent::EnterCombat,
        );
        world.systems.linking.on_event(
            world,
            creature_guid,
            crate::game::coordination::LinkEvent::Aggro { enemy: target_guid },
        );
        crate::game::creature::formation::on_member_attack_start(world, creature_guid, target_guid);
    }

    super::assist::call_assistance(world, creature_guid);
    if fresh && force_raid_combat {
        super::assist::set_in_combat_with_zone(world, creature_guid, true);
    }
}

/// Switch to a new victim in the middle of a fight.
fn switch_target(world: &World, creature_guid: ObjectGuid, target_guid: ObjectGuid) {
    let melee = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            creature.combat.attacking = Some(target_guid);
            if creature.ai_state == AIState::Combat
                && creature.ai_state_data.combat_movement_enabled
            {
                let position = creature.position;
                let combat_reach = creature.combat_reach;
                let run_speed = creature.run_speed();
                creature.motion_master.chase(
                    target_guid,
                    creature_guid,
                    position,
                    combat_reach,
                    run_speed,
                );
            }
            creature.ai_state_data.melee_enabled
        });
    if melee == Some(true) {
        send_attack_start(world, creature_guid, target_guid);
    }
    send_combat_state_update(world, creature_guid, target_guid);
}

/// Turn to face the victim. The new orientation is only broadcast when it
/// changed noticeably, so a creature standing in melee does not resend it
/// every update.
pub(crate) fn face_target(world: &World, creature_guid: ObjectGuid, target_guid: ObjectGuid) {
    const FACING_EPSILON: f32 = 0.1;
    let target_pos = if target_guid.is_player() {
        world.managers.player_mgr.get_player_position(target_guid)
    } else {
        world.managers.creature_mgr.get_position(target_guid)
    };
    let Some(target_pos) = target_pos else {
        return;
    };
    let changed = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            if creature.move_spline.is_active() {
                return None;
            }
            let position = creature.position;
            let angle = (target_pos.y - position.y).atan2(target_pos.x - position.x);
            let mut diff = (angle - position.o).abs() % std::f32::consts::TAU;
            if diff > std::f32::consts::PI {
                diff = std::f32::consts::TAU - diff;
            }
            creature.position.o = angle;
            (diff > FACING_EPSILON).then_some((position, angle))
        })
        .flatten();
    if let Some((position, angle)) = changed {
        world
            .systems
            .creature_movement
            .send_facing_packet(creature_guid, position, angle, world);
    }
}

/// Leave combat and walk home. Hits on the way show EVADE and do not pull the
/// creature back; health comes back through normal out-of-combat regeneration.
pub(crate) fn enter_evade_mode(world: &World, creature_guid: ObjectGuid) {
    // A player's pet does not walk home: it goes back to its owner.
    let player_pet = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |c| c.owner_guid.is_some_and(|o| o.is_player()))
        .unwrap_or(false);
    if player_pet {
        execute_single_action(world, creature_guid, AIAction::PetFollowOwner);
        return;
    }
    let Some(alive) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            let alive = creature.is_alive();
            creature.combat.leave_combat();
            creature.threat_manager.clear();
            creature.unit_flags &= !crate::game::common::unit_flags::IN_COMBAT;
            creature.ai_state_data.reset_combat();
            if !alive {
                return false;
            }
            let position = creature.position;
            creature.move_spline.stop();
            creature.motion_master.clear(creature_guid);
            let home = creature.home_position;
            let run_speed = creature.run_speed();
            creature
                .motion_master
                .return_home(home, creature_guid, position, run_speed);
            creature.ai_state = AIState::Evading;
            // An evading creature is untapped again.
            creature.loot_recipient = None;
            // Back to the template's spell list, melee and combat movement.
            let static_flags1 = creature.static_flags1;
            creature.ai_state_data.reset_combat_toggles(static_flags1);
            reset_spells_list(&mut creature.ai_state_data);
            true
        })
    else {
        return;
    };

    send_combat_exit_update(world, creature_guid);
    if !alive {
        return;
    }
    remove_auras_at_reset(world, creature_guid);
    super::system::queue_event(
        world,
        creature_guid,
        super::types::AIEvent::CombatEnded {
            reason: super::types::CombatEndReason::Evaded,
        },
    );
    crate::game::instance::scripts::queue_creature_combat_event(
        world,
        creature_guid,
        crate::game::instance::scripts::CreatureCombatEvent::Evade,
    );
    world
        .systems
        .linking
        .on_event(world, creature_guid, crate::game::coordination::LinkEvent::Evade);
    crate::game::creature::formation::on_member_evade(world, creature_guid);
}

/// Arrived home after an evade: idle again, idle movement resumes.
fn reached_home(world: &World, creature_guid: ObjectGuid) {
    world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            creature.ai_state = AIState::Idle;
            creature.combat.leave_combat();
            creature.threat_manager.clear();
            creature.unit_flags &= !crate::game::common::unit_flags::IN_COMBAT;
            creature
                .motion_master
                .clear_type(MovementGeneratorType::Chase, creature_guid);
            creature
                .motion_master
                .reinitialize_patrol_movement(creature_guid);
            // Formation members fall back in behind their leader on the next pass.
            if creature.owner_guid.is_none() {
                creature.following_target = None;
            }
        });
    send_combat_exit_update(world, creature_guid);
    super::system::queue_event(world, creature_guid, super::types::AIEvent::ReachedHome);
}

/// Restart every spell of the list on its initial delay.
fn reset_spells_list(data: &mut super::types::AIStateData) {
    let entries: Vec<_> = data.spells_list.iter().map(|s| s.inner.clone()).collect();
    data.set_spells_list(entries);
}

/// Remove the auras an evading creature drops: everything except positive
/// temporary buffs cast by players and auras flagged to survive an evade, or
/// only negative auras when the creature keeps its positive auras.
fn remove_auras_at_reset(world: &World, creature_guid: ObjectGuid) {
    const SPELL_CUSTOM_NOT_REMOVED_ON_EVADE: u32 = 0x400;
    let Some((auras, keep_positive)) = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |c| {
            (
                c.auras
                    .all_auras()
                    .map(|a| (a.spell_id, a.caster_guid, a.duration_ms.is_none()))
                    .collect::<Vec<_>>(),
                c.ai_profile.has_extra_flag(
                    crate::game::common::creature_flags::CREATURE_FLAG_EXTRA_KEEP_POSITIVE_AURAS_ON_EVADE,
                ),
            )
        })
    else {
        return;
    };
    let mut to_remove: Vec<u32> = auras
        .into_iter()
        .filter(|(spell_id, caster, permanent)| {
            let Some(spell) = world.managers.spell_mgr.get(*spell_id) else {
                return false;
            };
            let positive = spell.is_positive_spell();
            if keep_positive {
                return !positive;
            }
            let player_buff = caster.is_player() && !*permanent && positive;
            !player_buff && spell.custom & SPELL_CUSTOM_NOT_REMOVED_ON_EVADE == 0
        })
        .map(|(spell_id, _, _)| spell_id)
        .collect();
    to_remove.sort_unstable();
    to_remove.dedup();
    if to_remove.is_empty() {
        return;
    }
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        world
            .managers
            .creature_mgr
            .with_creature_mut(creature_guid, |c| {
                for spell_id in &to_remove {
                    c.remove_aura(*spell_id);
                }
            });
        return;
    };
    let world = world.clone();
    handle.spawn(async move {
        for spell_id in to_remove {
            if let Err(e) = world
                .systems
                .auras
                .remove_spell_auras(creature_guid, spell_id, &world)
                .await
            {
                tracing::debug!("Evade aura removal of {} failed: {}", spell_id, e);
            }
        }
    });
}

/// Turn melee on or off, telling clients the swing started or stopped.
pub(crate) fn set_melee_attack(world: &World, creature_guid: ObjectGuid, enabled: bool) {
    let victim = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            if creature.ai_state_data.melee_enabled == enabled {
                return None;
            }
            creature.ai_state_data.melee_enabled = enabled;
            creature.combat.attacking
        })
        .flatten();
    if let Some(victim) = victim {
        if enabled {
            send_attack_start(world, creature_guid, victim);
        } else {
            let stop = oxcore_shared::messages::combat::SmsgAttackStop {
                attacker_guid: creature_guid,
                target_guid: victim,
                unk: 0,
            };
            broadcast_around_creature(world, creature_guid, &stop);
        }
    }
}

/// Turn chasing on or off. Turning it off mid-fight stops the chase where the
/// creature stands; turning it on resumes chasing the victim.
pub(crate) fn set_combat_movement(world: &World, creature_guid: ObjectGuid, enabled: bool) {
    let stopped_at = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |creature| {
            if creature.ai_state_data.combat_movement_enabled == enabled {
                return None;
            }
            creature.ai_state_data.combat_movement_enabled = enabled;
            let victim = creature.combat.attacking?;
            let position = creature.position;
            if enabled {
                let combat_reach = creature.combat_reach;
                let run_speed = creature.run_speed();
                creature.motion_master.chase(
                    victim,
                    creature_guid,
                    position,
                    combat_reach,
                    run_speed,
                );
                None
            } else if creature.motion_master.active_generator() == MovementGeneratorType::Chase {
                creature
                    .motion_master
                    .clear_type(MovementGeneratorType::Chase, creature_guid);
                creature.motion_master.move_idle(creature_guid);
                creature.move_spline.stop();
                Some(position)
            } else {
                None
            }
        })
        .flatten();
    if let Some(position) = stopped_at {
        world
            .systems
            .creature_movement
            .send_stop_packet(creature_guid, position, world);
    }
}

/// Put `target` in combat with the creature that just engaged it.
pub(crate) fn put_in_combat_with(world: &World, target: ObjectGuid, creature_guid: ObjectGuid) {
    if target.is_player() {
        world
            .managers
            .player_mgr
            .with_player_mut(target, |p| p.combat.enter_combat(creature_guid));
    } else {
        let timestamp = super::units::now_ms();
        world.managers.creature_mgr.with_creature_mut(target, |c| {
            if c.is_alive() {
                c.combat.enter_combat(creature_guid, timestamp);
            }
        });
    }
}

/// Speak a broadcast text line: say, yell or emote, with its sound and emote.
pub(crate) fn script_text(world: &World, creature_guid: ObjectGuid, text_id: i32) {
    script_text_as(world, creature_guid, text_id, None);
}

/// Speak a broadcast text line, optionally overriding its chat type
/// (0 say, 1 yell, 2 emote, 3 boss emote, 6 zone yell, 7 zone emote).
pub(crate) fn script_text_as(
    world: &World,
    creature_guid: ObjectGuid,
    text_id: i32,
    chat_type_override: Option<u32>,
) {
    let text = u32::try_from(text_id)
        .ok()
        .and_then(|id| world.systems.gossip_manager.get_broadcast_text(id));
    let Some(text) = text else {
        tracing::debug!("Creature {:?} has no broadcast text {}", creature_guid, text_id);
        return;
    };
    let line = if text.male_text.is_empty() {
        &text.female_text
    } else {
        &text.male_text
    };
    // Broadcast chat types: 0 say, 1 yell, 2 emote, 3 boss emote, 6 zone yell.
    let chat_type = match chat_type_override.unwrap_or(text.chat_type as u32) {
        1 | 6 => 0x0C,
        2 | 3 | 7 => 0x0D,
        _ => 0x0B,
    };
    if !line.is_empty() {
        send_creature_chat(world, creature_guid, chat_type, line);
    }
    if text.sound_id != 0 {
        send_creature_sound(world, creature_guid, text.sound_id);
    }
    if text.emote_ids[0] != 0 {
        send_creature_emote(world, creature_guid, text.emote_ids[0]);
    }
}

/// Broadcast one unit field of the creature to the players around it.
pub(crate) fn send_unit_field(world: &World, creature_guid: ObjectGuid, field: u32, value: u32) {
    use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
    use oxcore_shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit).set_field(field, value),
    ));
    broadcast_around_creature(world, creature_guid, &update);
}


/// Send health update to nearby players
pub(crate) fn send_health_update(world: &World, creature_guid: ObjectGuid) {
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
        let flags = raw_flags;

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
pub(crate) fn send_combat_exit_update(world: &World, creature_guid: ObjectGuid) {
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
        let flags = raw_flags;

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
) -> bool {
    use oxcore_shared::messages::spells::{next_cast_sequence, SmsgSpellGo};
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
            return false;
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
            return false;
        }
    }

    // Build hit target list
    let hit_targets: Vec<ObjectGuid> = target_guid.into_iter().collect();

    // Broadcast SMSG_SPELL_GO to nearby players
    let msg = SmsgSpellGo {
        caster_guid: creature_guid,
        caster_guid_pack: creature_guid,
        spell_id,
        spell_visual_id: 0,
        cast_flags: if triggered { 0x0002 } else { 0x0000 },
        hit_targets,
        miss_targets: Vec::new(),
        target_guid,
        cast_item_guid: None,
        ammo_display_id: 0,
        ammo_inventory_type: 0,
        // No paired SPELL_START is sent for creature casts, so there is nothing to match --
        // a fresh identity is correct here.
        cast_sequence: next_cast_sequence(),
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
    true
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

/// Switch the creature to creature_spells list `entry` (0 clears it). Every
/// spell restarts on its initial delay.
pub fn set_spells_list_from_db(world: &World, creature_guid: ObjectGuid, entry: u32) {
    let list = (entry != 0)
        .then(|| world.managers.creature_mgr.spell_list(entry))
        .flatten();
    if entry != 0 && list.is_none() {
        tracing::warn!(
            "[AI] Creature {:?} asked for unknown creature_spells entry {}",
            creature_guid,
            entry
        );
        return;
    }
    world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| match &list {
            Some(list) => c.ai_state_data.set_spells_list(list.as_ref().clone()),
            None => c.ai_state_data.clear_spells_list(),
        });
}

/// Creature spell lists are worked through every 1.2 seconds.
const CREATURE_CASTING_DELAY: u32 = 1200;

/// Tick the casting delay and work through the spell list when it expires.
pub fn update_spells_list(world: &World, creature_guid: ObjectGuid, diff_ms: u32) {
    let cast_elapsed_ms = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| {
            let data = &mut c.ai_state_data;
            if data.casting_delay <= diff_ms {
                let desync = diff_ms - data.casting_delay;
                data.casting_delay = CREATURE_CASTING_DELAY.saturating_sub(desync);
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

/// Cast flags of creature spell lists and script casts.
pub mod cast_flags {
    pub const CF_INTERRUPT_PREVIOUS: u16 = 0x001;
    pub const CF_TRIGGERED: u16 = 0x002;
    pub const CF_FORCE_CAST: u16 = 0x004;
    /// Ranged casters: stop chasing and meleeing while this spell succeeds.
    pub const CF_MAIN_RANGED_SPELL: u16 = 0x008;
    /// Only when the target cannot be reached.
    pub const CF_TARGET_UNREACHABLE: u16 = 0x010;
    /// Only when the target lacks the spell's aura.
    pub const CF_AURA_NOT_PRESENT: u16 = 0x020;
    pub const CF_ONLY_IN_MELEE: u16 = 0x040;
    pub const CF_NOT_IN_MELEE: u16 = 0x080;
    /// Only to interrupt a target that is casting.
    pub const CF_TARGET_CASTING: u16 = 0x100;
}

/// Outcome of an attempted creature cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastOutcome {
    Ok,
    /// Fleeing or still casting: retry next update without a cooldown.
    Retry,
    /// The chance roll failed: the spell goes on cooldown anyway.
    TryAgain,
    /// Any other failure.
    Failed,
}

/// Decide whether a creature may cast `spell_id` at `target` now, then cast it.
pub fn try_to_cast(
    world: &World,
    creature_guid: ObjectGuid,
    target: Option<ObjectGuid>,
    spell_id: u32,
    flags: u16,
    chance: u8,
) -> CastOutcome {
    use cast_flags::*;
    const SPELL_CUSTOM_BEHIND_TARGET: u32 = 0x040;
    const SPELL_AURA_MOD_POSSESS: u32 = 2;
    const SPELL_AURA_MOD_CHARM: u32 = 6;
    const SPELL_EFFECT_APPLY_AURA: u32 = 6;
    const SPELL_EFFECT_SUMMON_GUARDIAN: u32 = 42;

    let Some(target) = target else {
        return CastOutcome::Failed;
    };
    let Some(spell) = world.managers.spell_mgr.get(spell_id) else {
        return CastOutcome::Failed;
    };
    let Some(me) = super::units::resolve_unit(world, creature_guid) else {
        return CastOutcome::Failed;
    };
    let Some(other) = super::units::resolve_unit(world, target) else {
        return CastOutcome::Failed;
    };
    let Some((fleeing, chasing_unreachable, threat_count)) =
        world.managers.creature_mgr.with_creature_mut(creature_guid, |c| {
            let generator = c.motion_master.active_generator();
            let unreachable = generator == MovementGeneratorType::Chase
                && c.motion_master
                    .get_generator_mut(MovementGeneratorType::Chase)
                    .and_then(|g| {
                        g.as_any_mut()
                            .downcast_mut::<crate::game::creature::movement::generators::ChaseMovementGenerator>()
                    })
                    .is_some_and(|chase| !chase.is_reachable());
            (
                generator == MovementGeneratorType::Fleeing,
                unreachable || !c.ai_state_data.combat_movement_enabled,
                c.threat_manager.get_threat_list().len(),
            )
        })
    else {
        return CastOutcome::Failed;
    };

    if flags & CF_AURA_NOT_PRESENT != 0 && target_has_aura(world, target, spell_id) {
        return CastOutcome::Failed;
    }
    if fleeing {
        return CastOutcome::Retry;
    }
    if flags & CF_TARGET_CASTING != 0 && !is_casting(world, target) {
        return CastOutcome::Failed;
    }
    let in_melee = super::units::in_melee_range(&me.position, me.combat_reach, &other);
    if flags & CF_ONLY_IN_MELEE != 0 && !in_melee {
        return CastOutcome::Failed;
    }
    if flags & CF_NOT_IN_MELEE != 0 && in_melee {
        return CastOutcome::Failed;
    }
    if flags & CF_TARGET_UNREACHABLE != 0 && (in_melee || !chasing_unreachable) {
        return CastOutcome::Failed;
    }
    if flags & CF_FORCE_CAST == 0 {
        if spell.custom & SPELL_CUSTOM_BEHIND_TARGET != 0
            && super::aggro::is_in_front(
                other.position.o,
                (other.position.x, other.position.y),
                (me.position.x, me.position.y),
            )
        {
            return CastOutcome::Failed;
        }
        let charm = (0..3).any(|i| {
            spell.effect[i] == SPELL_EFFECT_APPLY_AURA
                && matches!(
                    spell.effect_apply_aura_name[i],
                    SPELL_AURA_MOD_CHARM | SPELL_AURA_MOD_POSSESS
                )
        });
        if charm && threat_count == 1 {
            return CastOutcome::Failed;
        }
        let guardian_entry = (0..3)
            .find(|&i| spell.effect[i] == SPELL_EFFECT_SUMMON_GUARDIAN)
            .map(|i| spell.effect_misc_value[i].max(0) as u32);
        if let Some(entry) = guardian_entry {
            if crate::game::creature::summon::has_summon_with_entry(creature_guid, entry) {
                return CastOutcome::Failed;
            }
        }
    }

    // The checks a cast makes before it starts: range and line of sight.
    if target != creature_guid {
        let range = super::script_target::spell_max_range(world, &spell);
        if range > 0.0 && super::units::distance(&me.position, &other.position) > range + other.combat_reach
        {
            return CastOutcome::Failed;
        }
        if !world.managers.vmap_mgr.is_in_line_of_sight(
            me.map_id,
            me.instance_id,
            me.position,
            other.position,
        ) {
            return CastOutcome::Failed;
        }
    }

    if chance < 100 && rand::thread_rng().gen_range(0..100u32) >= chance as u32 {
        return CastOutcome::TryAgain;
    }
    if execute_creature_spell_cast(
        world,
        creature_guid,
        spell_id,
        Some(target),
        flags & CF_TRIGGERED != 0,
    ) {
        CastOutcome::Ok
    } else {
        CastOutcome::Failed
    }
}

fn target_has_aura(world: &World, target: ObjectGuid, spell_id: u32) -> bool {
    if target.is_player() {
        world
            .managers
            .player_mgr
            .with_player(target, |p| p.auras.container.has_aura(spell_id))
            .unwrap_or(false)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(target, |c| c.auras.has_aura(spell_id))
            .unwrap_or(false)
    }
}

/// A player in the middle of a cast or channel.
fn is_casting(world: &World, unit: ObjectGuid) -> bool {
    use crate::game::player::spells::state::CurrentSpellType;
    unit.is_player()
        && world
            .managers
            .player_mgr
            .with_player(unit, |p| {
                p.spells.current_spells[CurrentSpellType::Generic as usize].is_some()
                    || p.spells.current_spells[CurrentSpellType::Channeled as usize].is_some()
            })
            .unwrap_or(false)
}

/// Work through the spell list: each spell whose cooldown ran out tries to
/// cast at its target type. Only one untriggered spell succeeds per pass.
/// A ranged caster stops chasing and meleeing while its main spell succeeds
/// and resumes when it fails.
pub fn do_spells_list_casts(world: &World, creature_guid: ObjectGuid, diff_ms: u32) {
    use cast_flags::*;
    let Some(spells) = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |c| c.ai_state_data.spells_list.clone())
    else {
        return;
    };
    if spells.is_empty() {
        return;
    }
    let Some(source) = super::script_target::Source::of_creature(world, creature_guid) else {
        return;
    };

    let repeat_delay = |entry: &super::types::CreatureSpellsEntry| {
        if entry.delay_repeat_max > entry.delay_repeat_min {
            rand::thread_rng().gen_range(entry.delay_repeat_min..=entry.delay_repeat_max)
        } else {
            entry.delay_repeat_min
        }
    };

    let mut cooldowns: Vec<u32> = Vec::with_capacity(spells.len());
    let mut dont_cast = false;
    let mut main_ranged: Option<bool> = None;

    for spell in &spells {
        let entry = &spell.inner;
        if spell.cooldown > diff_ms {
            cooldowns.push(spell.cooldown - diff_ms);
            continue;
        }
        let mut cooldown = 0;
        if entry.cast_flags & (CF_TRIGGERED | CF_INTERRUPT_PREVIOUS) == 0 && dont_cast {
            cooldowns.push(cooldown);
            continue;
        }
        let spell_id = entry.spell_id as u32;
        let spell_entry = world.managers.spell_mgr.get(spell_id);
        // The provided target of a spell list is the creature itself.
        let target = super::script_target::get_target_by_type(
            world,
            &source,
            Some(creature_guid),
            entry.cast_target,
            entry.target_param1,
            entry.target_param2,
            spell_entry.as_deref(),
        );
        let ranged = entry.cast_flags & CF_MAIN_RANGED_SPELL != 0;
        match try_to_cast(
            world,
            creature_guid,
            target,
            spell_id,
            entry.cast_flags,
            entry.probability,
        ) {
            CastOutcome::Ok => {
                dont_cast = entry.cast_flags & CF_TRIGGERED == 0;
                cooldown = repeat_delay(entry);
                if ranged {
                    main_ranged = Some(false);
                }
                if entry.script_id != 0 {
                    tracing::debug!(
                        "[AI] Creature {:?} spell {} would start creature_spells_scripts {}",
                        creature_guid,
                        spell_id,
                        entry.script_id
                    );
                }
            }
            CastOutcome::Retry => {}
            CastOutcome::TryAgain => {
                cooldown = repeat_delay(entry);
                if ranged {
                    main_ranged = Some(true);
                }
            }
            CastOutcome::Failed => {
                if ranged {
                    main_ranged = Some(true);
                }
            }
        }
        cooldowns.push(cooldown);
    }

    world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| {
            for (entry, cooldown) in c.ai_state_data.spells_list.iter_mut().zip(cooldowns) {
                entry.cooldown = cooldown;
            }
        });

    // Ranged casters: success holds position, failure resumes chase and melee.
    if let Some(enable) = main_ranged {
        if !enable {
            execute_single_action(world, creature_guid, AIAction::StopMovement);
        }
        set_combat_movement(world, creature_guid, enable);
        set_melee_attack(world, creature_guid, enable);
    }
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
pub(crate) fn send_creature_chat(world: &World, creature_guid: ObjectGuid, chat_type: u8, text: &str) {
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
pub(crate) fn send_creature_emote(world: &World, creature_guid: ObjectGuid, emote_id: u32) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_EMOTE);
    packet.write_u32(emote_id);
    packet.write_u64(creature_guid.raw());

    crate::game::broadcast_mgr::broadcast_around_creature(world, creature_guid, &packet);
}

/// Send SMSG_PLAY_OBJECT_SOUND for creature sounds.
pub(crate) fn send_creature_sound(world: &World, creature_guid: ObjectGuid, sound_id: u32) {
    use oxcore_shared::protocol::{Opcode, WorldPacket};

    let mut packet = WorldPacket::new(Opcode::SMSG_PLAY_OBJECT_SOUND);
    packet.write_u32(sound_id);
    packet.write_u64(creature_guid.raw());

    crate::game::broadcast_mgr::broadcast_around_creature(world, creature_guid, &packet);
}

/// Send display ID update to nearby players (for Morph/Demorph).
pub(crate) fn send_display_update(world: &World, creature_guid: ObjectGuid, display_id: u32) {
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
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::ObjectGuid;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
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
            pickpocketed: false,
            ai_state: AIState::Idle,
            ai_state_data: crate::game::creature::ai::AIStateData::new(),
            ai_profile: Default::default(),
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
