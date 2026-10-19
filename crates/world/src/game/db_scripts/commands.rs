//! The database script commands. Each returns true when the script should stop:
//! a failed step returns its abort flag, a terminate command returns its verdict.

use super::{ScriptInfo, ScriptTable};
use crate::game::creature::ai::executor::{self, cast_flags, CastOutcome};
use crate::game::creature::ai::{self, units, AIAction, AIEvent, ReactState};
use crate::game::creature::movement::generators::Waypoint;
use crate::game::creature::summon::{self, SummonDespawn};
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::Rng;

pub const SCRIPT_COMMAND_TALK: u32 = 0;
pub const SCRIPT_COMMAND_EMOTE: u32 = 1;
pub const SCRIPT_COMMAND_FIELD_SET: u32 = 2;
pub const SCRIPT_COMMAND_MOVE_TO: u32 = 3;
pub const SCRIPT_COMMAND_MODIFY_FLAGS: u32 = 4;
pub const SCRIPT_COMMAND_INTERRUPT_CASTS: u32 = 5;
pub const SCRIPT_COMMAND_TELEPORT_TO: u32 = 6;
pub const SCRIPT_COMMAND_QUEST_EXPLORED: u32 = 7;
pub const SCRIPT_COMMAND_KILL_CREDIT: u32 = 8;
pub const SCRIPT_COMMAND_RESPAWN_GAMEOBJECT: u32 = 9;
pub const SCRIPT_COMMAND_TEMP_SUMMON_CREATURE: u32 = 10;
pub const SCRIPT_COMMAND_OPEN_DOOR: u32 = 11;
pub const SCRIPT_COMMAND_CLOSE_DOOR: u32 = 12;
pub const SCRIPT_COMMAND_REMOVE_AURA: u32 = 14;
pub const SCRIPT_COMMAND_CAST_SPELL: u32 = 15;
pub const SCRIPT_COMMAND_PLAY_SOUND: u32 = 16;
pub const SCRIPT_COMMAND_DESPAWN_CREATURE: u32 = 18;
pub const SCRIPT_COMMAND_MOVEMENT: u32 = 20;
pub const SCRIPT_COMMAND_SET_ACTIVEOBJECT: u32 = 21;
pub const SCRIPT_COMMAND_SET_FACTION: u32 = 22;
pub const SCRIPT_COMMAND_MORPH_TO_ENTRY_OR_MODEL: u32 = 23;
pub const SCRIPT_COMMAND_MOUNT_TO_ENTRY_OR_MODEL: u32 = 24;
pub const SCRIPT_COMMAND_ATTACK_START: u32 = 26;
pub const SCRIPT_COMMAND_STAND_STATE: u32 = 28;
pub const SCRIPT_COMMAND_MODIFY_THREAT: u32 = 29;
pub const SCRIPT_COMMAND_TERMINATE_SCRIPT: u32 = 31;
pub const SCRIPT_COMMAND_TERMINATE_CONDITION: u32 = 32;
pub const SCRIPT_COMMAND_ENTER_EVADE_MODE: u32 = 33;
pub const SCRIPT_COMMAND_SET_HOME_POSITION: u32 = 34;
pub const SCRIPT_COMMAND_TURN_TO: u32 = 35;
pub const SCRIPT_COMMAND_SET_INST_DATA: u32 = 37;
pub const SCRIPT_COMMAND_SET_INST_DATA64: u32 = 38;
pub const SCRIPT_COMMAND_START_SCRIPT: u32 = 39;
pub const SCRIPT_COMMAND_REMOVE_OBJECT: u32 = 41;
pub const SCRIPT_COMMAND_SET_MELEE_ATTACK: u32 = 42;
pub const SCRIPT_COMMAND_SET_COMBAT_MOVEMENT: u32 = 43;
pub const SCRIPT_COMMAND_SET_PHASE: u32 = 44;
pub const SCRIPT_COMMAND_SET_PHASE_RANDOM: u32 = 45;
pub const SCRIPT_COMMAND_SET_PHASE_RANGE: u32 = 46;
pub const SCRIPT_COMMAND_FLEE: u32 = 47;
pub const SCRIPT_COMMAND_DEAL_DAMAGE: u32 = 48;
pub const SCRIPT_COMMAND_ZONE_COMBAT_PULSE: u32 = 49;
pub const SCRIPT_COMMAND_CALL_FOR_HELP: u32 = 50;
pub const SCRIPT_COMMAND_INVINCIBILITY: u32 = 52;
pub const SCRIPT_COMMAND_GAME_EVENT: u32 = 53;
pub const SCRIPT_COMMAND_CREATURE_SPELLS: u32 = 55;
pub const SCRIPT_COMMAND_SET_REACT_STATE: u32 = 59;
pub const SCRIPT_COMMAND_START_WAYPOINTS: u32 = 60;
pub const SCRIPT_COMMAND_SET_DEFAULT_MOVEMENT: u32 = 67;
pub const SCRIPT_COMMAND_START_SCRIPT_FOR_ALL: u32 = 68;
pub const SCRIPT_COMMAND_RESPAWN_CREATURE: u32 = 71;
pub const SCRIPT_COMMAND_ASSIST_UNIT: u32 = 72;
pub const SCRIPT_COMMAND_COMBAT_STOP: u32 = 73;
pub const SCRIPT_COMMAND_ADD_THREAT: u32 = 75;
pub const SCRIPT_COMMAND_SUMMON_OBJECT: u32 = 76;
pub const SCRIPT_COMMAND_SET_GO_STATE: u32 = 80;
pub const SCRIPT_COMMAND_DESPAWN_GAMEOBJECT: u32 = 81;
pub const SCRIPT_COMMAND_QUEST_CREDIT: u32 = 83;
pub const SCRIPT_COMMAND_SEND_SCRIPT_EVENT: u32 = 85;

/// Motion types as scripts number them.
const IDLE_MOTION_TYPE: u32 = 0;
const RANDOM_MOTION_TYPE: u32 = 1;
const WAYPOINT_MOTION_TYPE: u32 = 2;
const CONFUSED_MOTION_TYPE: u32 = 5;
const CHASE_MOTION_TYPE: u32 = 6;
const HOME_MOTION_TYPE: u32 = 7;
pub const POINT_MOTION_TYPE: u32 = 9;
const FLEEING_MOTION_TYPE: u32 = 10;
const DISTRACT_MOTION_TYPE: u32 = 11;
const FOLLOW_MOTION_TYPE: u32 = 15;
const CHARGE_MOTION_TYPE: u32 = 18;

/// Movement flag of SCRIPT_COMMAND_MOVE_TO: switch to point movement (informs scripts).
const SF_MOVETO_POINT_MOVEGEN: u32 = 0x2;
/// Movement option: run instead of walk.
const MOVE_RUN_MODE: u32 = 0x004;
/// Summon flags of SCRIPT_COMMAND_TEMP_SUMMON_CREATURE.
const SF_SUMMONCREATURE_UNIQUE: i32 = 0x04;
const SF_SUMMONCREATURE_UNIQUE_TEMP: i32 = 0x08;
const SF_SUMMONCREATURE_NULL_AI: i32 = 0x10;
/// Flee for this long when fleeing without seeking help.
const FLEE_DELAY_MS: u32 = 7000;
/// "%s attempts to run away in fear!"
const CREATURE_FLEE_TEXT: i32 = 1150;

fn creature(guid: Option<ObjectGuid>) -> Option<ObjectGuid> {
    guid.filter(|g| g.is_creature())
}

fn unit(guid: Option<ObjectGuid>) -> Option<ObjectGuid> {
    guid.filter(|g| g.is_creature() || g.is_player())
}

fn player(guid: Option<ObjectGuid>) -> Option<ObjectGuid> {
    guid.filter(|g| g.is_player())
}

fn alive(world: &World, guid: ObjectGuid) -> bool {
    units::resolve_unit(world, guid).is_some_and(|u| u.alive)
}

fn act(world: &World, guid: ObjectGuid, action: AIAction) {
    executor::execute_actions(world, guid, vec![action]);
}

/// A game object by database guid in the source's copy, or the source or target itself.
fn gameobject(
    world: &World,
    db_guid: u32,
    source: Option<ObjectGuid>,
    target: Option<ObjectGuid>,
) -> Option<ObjectGuid> {
    if db_guid != 0 {
        let origin = source.or(target)?;
        let (map_id, instance_id) = super::source_of(world, origin).map(|s| (s.map_id, s.instance_id))?;
        return world
            .managers
            .gameobject_mgr
            .iter_gameobjects()
            .find(|e| {
                let go = e.value();
                go.spawn_id == db_guid && go.map_id == map_id && go.instance_id == instance_id
            })
            .map(|e| *e.key());
    }
    source
        .filter(|g| g.is_game_object())
        .or(target.filter(|g| g.is_game_object()))
}

/// Pick one of up to four ids by their chances (chances sum to at most 100).
fn choose_by_chance(ids: [u32; 4], chances: [i32; 4], skip_zero_chance: bool) -> u32 {
    let roll = rand::thread_rng().gen_range(1..=100u32);
    let mut sum = 0u32;
    for (id, chance) in ids.into_iter().zip(chances) {
        if (!skip_zero_chance && id == 0) || (skip_zero_chance && chance == 0) {
            continue;
        }
        let chance = chance.max(0) as u32;
        if roll > sum && roll <= sum + chance {
            return id;
        }
        sum += chance;
    }
    0
}

/// Run one script command. Returns true when the script should stop.
pub fn execute(world: &World, info: &ScriptInfo, source: Option<ObjectGuid>, target: Option<ObjectGuid>) -> bool {
    let fail = info.abort_on_failure();
    let d = &info.datalong;
    match info.command {
        SCRIPT_COMMAND_TALK => {
            let Some(speaker) = creature(source) else {
                return fail;
            };
            let count = info.dataint.iter().take_while(|id| **id != 0).count().max(1);
            let text_id = info.dataint[rand::thread_rng().gen_range(0..count)];
            let chat_type = (d[0] > 0).then_some(d[0]);
            executor::script_text_as(world, speaker, text_id, chat_type);
            false
        }
        SCRIPT_COMMAND_EMOTE => {
            let Some(emoter) = creature(source) else {
                return fail;
            };
            let count = d.iter().take_while(|id| **id != 0).count().max(1);
            let emote = d[rand::thread_rng().gen_range(0..count)];
            if info.dataint[0] != 0 {
                let Some(target) = unit(target) else {
                    return fail;
                };
                let in_combat = world
                    .managers
                    .creature_mgr
                    .with_creature(emoter, |c| c.combat.in_combat)
                    .unwrap_or(true);
                if in_combat {
                    return fail;
                }
                executor::face_target(world, emoter, target);
            }
            executor::send_creature_emote(world, emoter, emote);
            false
        }
        SCRIPT_COMMAND_FIELD_SET | SCRIPT_COMMAND_MODIFY_FLAGS => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            let mode = if info.command == SCRIPT_COMMAND_FIELD_SET { 3 } else { d[2] };
            if set_creature_field(world, guid, d[0], d[1], mode) {
                false
            } else {
                fail
            }
        }
        SCRIPT_COMMAND_MOVE_TO => move_to(world, info, source, target),
        SCRIPT_COMMAND_INTERRUPT_CASTS => {
            // Creature casts complete at once; there is nothing in progress to interrupt.
            false
        }
        SCRIPT_COMMAND_TELEPORT_TO => {
            let Some(player) = player(source).or(player(target)) else {
                if let Some(guid) = creature(source) {
                    teleport_creature(world, guid, Position::new(info.x, info.y, info.z, info.o));
                    return false;
                }
                return fail;
            };
            let dest = Position::new(info.x, info.y, info.z, info.o);
            let world_clone = world.clone();
            let (map, options) = (d[0], d[1]);
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = crate::game::player::teleport::teleport_to(
                        &world_clone,
                        player,
                        map,
                        dest,
                        options,
                    )
                    .await;
                });
            }
            false
        }
        SCRIPT_COMMAND_QUEST_EXPLORED => {
            let Some(player) = player(target).or(player(source)) else {
                return fail;
            };
            world.systems.quest.handle_area_explored(player, d[0]);
            false
        }
        SCRIPT_COMMAND_KILL_CREDIT => {
            let Some(player) = player(target).or(player(source)) else {
                return fail;
            };
            let credit_source = source.or(target).unwrap_or_default();
            world.systems.quest.handle_kill_credit(player, d[0], credit_source);
            false
        }
        SCRIPT_COMMAND_RESPAWN_GAMEOBJECT => {
            let Some(go) = gameobject(world, d[0], source, target) else {
                return fail;
            };
            summon::respawn_gameobject(world, go, d[1]);
            false
        }
        SCRIPT_COMMAND_TEMP_SUMMON_CREATURE => summon_creature(world, info, source, target),
        SCRIPT_COMMAND_OPEN_DOOR | SCRIPT_COMMAND_CLOSE_DOOR => {
            let Some(door) = gameobject(world, d[0], source, target) else {
                return fail;
            };
            crate::game::instance::scripts::set_door(world, door, info.command == SCRIPT_COMMAND_OPEN_DOOR);
            false
        }
        SCRIPT_COMMAND_REMOVE_AURA => {
            let Some(guid) = unit(source) else {
                return fail;
            };
            remove_auras(world, guid, d[0]);
            false
        }
        SCRIPT_COMMAND_CAST_SPELL => cast_spell(world, info, source, target),
        SCRIPT_COMMAND_PLAY_SOUND => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            executor::send_creature_sound(world, guid, d[0]);
            false
        }
        SCRIPT_COMMAND_DESPAWN_CREATURE => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            crate::game::creature::forced::force_despawn(world, guid);
            false
        }
        SCRIPT_COMMAND_MOVEMENT => movement(world, info, source, target),
        SCRIPT_COMMAND_SET_ACTIVEOBJECT => false,
        SCRIPT_COMMAND_SET_FACTION => {
            let Some(guid) = creature(source).or(creature(target)) else {
                return fail;
            };
            let faction = if d[0] != 0 {
                d[0]
            } else {
                world
                    .managers
                    .creature_mgr
                    .get_template(guid.entry())
                    .map(|t| t.faction)
                    .unwrap_or(0)
            };
            act(world, guid, AIAction::SetFaction { faction_id: faction });
            false
        }
        SCRIPT_COMMAND_MORPH_TO_ENTRY_OR_MODEL => {
            let Some(guid) = creature(source).or(creature(target)) else {
                return fail;
            };
            if !alive(world, guid) {
                return fail;
            }
            if d[0] == 0 {
                act(world, guid, AIAction::Demorph);
            } else {
                let display = if d[1] != 0 {
                    d[0]
                } else {
                    world
                        .managers
                        .creature_mgr
                        .get_template(d[0])
                        .map(|t| t.get_display_id())
                        .unwrap_or(0)
                };
                act(world, guid, AIAction::Morph { display_id: display });
            }
            false
        }
        SCRIPT_COMMAND_MOUNT_TO_ENTRY_OR_MODEL => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            let display = if d[1] != 0 || d[0] == 0 {
                d[0]
            } else {
                world
                    .managers
                    .creature_mgr
                    .get_template(d[0])
                    .map(|t| t.get_display_id())
                    .unwrap_or(0)
            };
            executor::send_unit_field(
                world,
                guid,
                crate::game::common::update_fields::UNIT_FIELD_MOUNTDISPLAYID,
                display,
            );
            false
        }
        SCRIPT_COMMAND_ATTACK_START => {
            let (Some(attacker), Some(target)) = (creature(source), unit(target)) else {
                return fail;
            };
            if !alive(world, attacker) || !valid_attack_target(world, attacker, target) {
                return fail;
            }
            act(world, attacker, AIAction::EnterCombat { target_guid: target });
            false
        }
        SCRIPT_COMMAND_STAND_STATE => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            if !alive(world, guid) {
                return fail;
            }
            act(world, guid, AIAction::SetStandState { state: d[0] as u8 });
            false
        }
        SCRIPT_COMMAND_MODIFY_THREAT => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            const ALL_ATTACKERS: u32 = 8;
            let percent = info.x as i32;
            let targets: Vec<ObjectGuid> = if d[0] == ALL_ATTACKERS {
                world
                    .managers
                    .creature_mgr
                    .with_creature(guid, |c| c.threat_manager.online_targets())
                    .unwrap_or_default()
            } else {
                super::source_of(world, guid)
                    .and_then(|origin| {
                        ai::script_target::get_target_by_type(world, &origin, target, d[0] as u8, 0, 0, None)
                    })
                    .into_iter()
                    .collect()
            };
            world.managers.creature_mgr.with_creature_mut(guid, |c| {
                for t in targets {
                    c.threat_manager.modify_threat_percent(t, percent);
                }
            });
            false
        }
        SCRIPT_COMMAND_TERMINATE_SCRIPT => {
            if d[0] == 0 {
                return true;
            }
            let Some(origin) = source.or(target).and_then(|g| super::source_of(world, g)) else {
                return fail;
            };
            let found = ai::script_target::get_target_by_type(
                world,
                &origin,
                None,
                ai::script_target::TARGET_T_NEAREST_CREATURE_WITH_ENTRY,
                d[0],
                d[1],
                None,
            )
            .is_some();
            match d[2] {
                0 => !found,
                _ => found,
            }
        }
        SCRIPT_COMMAND_TERMINATE_CONDITION => {
            let context = crate::game::conditions::ConditionContext {
                target: target.unwrap_or_default(),
                source: source.unwrap_or_default(),
            };
            let mut result = world.managers.condition_mgr.is_satisfied(d[0], context, world);
            if d[2] & 0x1 != 0 {
                result = !result;
            }
            result
        }
        SCRIPT_COMMAND_ENTER_EVADE_MODE => {
            let Some(guid) = creature(source).or(creature(target)) else {
                return fail;
            };
            if !alive(world, guid) {
                return fail;
            }
            executor::enter_evade_mode(world, guid);
            false
        }
        SCRIPT_COMMAND_SET_HOME_POSITION => {
            let Some(guid) = creature(source).or(creature(target)) else {
                return fail;
            };
            let spawn_home = world
                .managers
                .creature_mgr
                .get_spawn_data(guid)
                .map(|s| s.position);
            world.managers.creature_mgr.with_creature_mut(guid, |c| {
                c.home_position = match d[0] {
                    0 => Position::new(info.x, info.y, info.z, info.o),
                    1 => c.position,
                    _ => spawn_home.unwrap_or(c.home_position),
                };
            });
            false
        }
        SCRIPT_COMMAND_TURN_TO => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            if !alive(world, guid) {
                return fail;
            }
            if d[0] == 0 {
                let Some(target) = target else {
                    return fail;
                };
                executor::face_target(world, guid, target);
            } else {
                let position = world.managers.creature_mgr.with_creature_mut(guid, |c| {
                    c.position.o = info.o;
                    c.position
                });
                if let Some(position) = position {
                    world
                        .systems
                        .creature_movement
                        .send_facing_packet(guid, position, info.o, world);
                }
            }
            false
        }
        SCRIPT_COMMAND_SET_INST_DATA => {
            let Some((map_id, instance_id)) = source
                .or(target)
                .and_then(|g| super::source_of(world, g))
                .map(|s| (s.map_id, s.instance_id))
            else {
                return fail;
            };
            let old = world
                .managers
                .lua_mgr
                .get_instance_state(map_id, instance_id)
                .get_data(d[0]);
            let value = match d[2] {
                1 => old.saturating_add(d[1]),
                2 => old.saturating_sub(d[1]),
                _ => d[1],
            };
            world.managers.lua_mgr.set_instance_data(map_id, instance_id, d[0], value);
            crate::game::instance::scripts::queue_save(world, map_id, instance_id);
            false
        }
        SCRIPT_COMMAND_SET_INST_DATA64 => {
            let Some((map_id, instance_id)) = source
                .or(target)
                .and_then(|g| super::source_of(world, g))
                .map(|s| (s.map_id, s.instance_id))
            else {
                return fail;
            };
            let guid = match d[2] {
                1 => match source {
                    Some(g) => g,
                    None => return fail,
                },
                _ => ObjectGuid::from_raw(u64::from(d[1])),
            };
            let mut state = world.managers.lua_mgr.get_instance_state(map_id, instance_id);
            state.set_guid(d[0], guid);
            world.managers.lua_mgr.set_instance_state(map_id, instance_id, state);
            false
        }
        SCRIPT_COMMAND_START_SCRIPT => {
            let id = choose_by_chance(*d, info.dataint, false);
            if id == 0 {
                return fail;
            }
            super::start(world, ScriptTable::Generic, id, source, target);
            false
        }
        SCRIPT_COMMAND_REMOVE_OBJECT => {
            let Some(guid) = source else {
                return fail;
            };
            if guid.is_game_object() {
                summon::hide_gameobject(world, guid);
            } else if guid.is_creature() {
                summon::despawn_creature(world, guid);
            } else {
                return fail;
            }
            false
        }
        SCRIPT_COMMAND_SET_MELEE_ATTACK => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            executor::set_melee_attack(world, guid, d[0] != 0);
            false
        }
        SCRIPT_COMMAND_SET_COMBAT_MOVEMENT => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            executor::set_combat_movement(world, guid, d[0] != 0);
            false
        }
        SCRIPT_COMMAND_SET_PHASE | SCRIPT_COMMAND_SET_PHASE_RANDOM | SCRIPT_COMMAND_SET_PHASE_RANGE => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            let phase = match info.command {
                SCRIPT_COMMAND_SET_PHASE => {
                    let current = ai::event_ai::phase(guid).unwrap_or(0);
                    match d[1] {
                        1 => current.saturating_add(d[0]),
                        2 => current.saturating_sub(d[0]),
                        _ => d[0],
                    }
                }
                SCRIPT_COMMAND_SET_PHASE_RANDOM => {
                    let count = if d[3] != 0 {
                        4
                    } else if d[2] != 0 {
                        3
                    } else {
                        2
                    };
                    d[rand::thread_rng().gen_range(0..count)]
                }
                _ => rand::thread_rng().gen_range(d[0].min(d[1])..=d[0].max(d[1])),
            };
            if ai::event_ai::set_phase(guid, phase) {
                false
            } else {
                fail
            }
        }
        SCRIPT_COMMAND_FLEE => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            if flee(world, guid, d[0] != 0) {
                false
            } else {
                fail
            }
        }
        SCRIPT_COMMAND_DEAL_DAMAGE => {
            // The damage lands on the source itself.
            let Some(guid) = creature(source) else {
                return fail;
            };
            let Some((max_health, alive)) = world
                .managers
                .creature_mgr
                .with_creature(guid, |c| (c.max_health, c.is_alive()))
            else {
                return fail;
            };
            if !alive {
                return fail;
            }
            let damage = if d[1] != 0 {
                (max_health as f32 * d[0] as f32 / 100.0) as u32
            } else {
                d[0]
            };
            let (_, dead) = world
                .managers
                .creature_mgr
                .apply_damage(guid, damage, guid, units::now_ms())
                .unwrap_or((0, false));
            executor::send_health_update(world, guid);
            if dead {
                crate::game::creature::forced::force_kill(world, guid, None);
            }
            false
        }
        SCRIPT_COMMAND_ZONE_COMBAT_PULSE => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            if !alive(world, guid) {
                return fail;
            }
            ai::assist::set_in_combat_with_zone(world, guid, d[0] != 0);
            false
        }
        SCRIPT_COMMAND_CALL_FOR_HELP => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            if !alive(world, guid) {
                return fail;
            }
            ai::assist::call_for_help(world, guid, info.x);
            false
        }
        SCRIPT_COMMAND_INVINCIBILITY => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            world.managers.creature_mgr.with_creature_mut(guid, |c| {
                c.ai_state_data.invincibility_hp = if d[1] != 0 {
                    c.max_health * d[0] / 100
                } else {
                    d[0]
                };
            });
            false
        }
        SCRIPT_COMMAND_GAME_EVENT => {
            world.managers.game_event_mgr.set_forced(d[0] as u16, d[1] != 0);
            false
        }
        SCRIPT_COMMAND_CREATURE_SPELLS => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            let id = choose_by_chance(*d, info.dataint, true);
            executor::set_spells_list_from_db(world, guid, id);
            false
        }
        SCRIPT_COMMAND_SET_REACT_STATE => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            let react_state = match d[0] {
                0 => ReactState::Passive,
                1 => ReactState::Defensive,
                _ => ReactState::Aggressive,
            };
            act(world, guid, AIAction::SetReactState { react_state });
            false
        }
        SCRIPT_COMMAND_START_WAYPOINTS => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            if start_waypoints(world, guid, info) {
                false
            } else {
                fail
            }
        }
        SCRIPT_COMMAND_SET_DEFAULT_MOVEMENT => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            world.managers.creature_mgr.with_creature_mut(guid, |c| {
                match d[0] {
                    RANDOM_MOTION_TYPE => {
                        if c.wander_distance <= 0.0 {
                            c.wander_distance = if d[2] != 0 { d[2] as f32 } else { 5.0 };
                        }
                    }
                    _ => c.wander_distance = 0.0,
                }
                c.motion_master.reinitialize_patrol_movement(guid);
            });
            false
        }
        SCRIPT_COMMAND_START_SCRIPT_FOR_ALL => {
            let Some(origin) = source.or(target).and_then(|g| super::source_of(world, g)) else {
                return fail;
            };
            let Some(map) = world.managers.map_mgr.get_map(origin.map_id, origin.instance_id) else {
                return fail;
            };
            const OBJECT_CREATURE: u32 = 1;
            const OBJECT_GAMEOBJECT: u32 = 2;
            for guid in map.get_objects_in_range(origin.position, d[3] as f32) {
                let matches_type = match d[1] {
                    OBJECT_CREATURE => guid.is_creature(),
                    OBJECT_GAMEOBJECT => guid.is_game_object(),
                    _ => guid.is_player(),
                };
                if matches_type && (d[2] == 0 || guid.entry() == d[2]) {
                    super::start(world, ScriptTable::Generic, d[0], Some(guid), target);
                }
            }
            false
        }
        SCRIPT_COMMAND_RESPAWN_CREATURE => {
            let Some(guid) = creature(source).or(creature(target)) else {
                return fail;
            };
            if alive(world, guid) {
                if d[0] == 0 {
                    return fail;
                }
                crate::game::creature::forced::force_despawn(world, guid);
            }
            crate::game::creature::forced::force_respawn(world, guid);
            false
        }
        SCRIPT_COMMAND_ASSIST_UNIT => {
            let (Some(helper), Some(friend)) = (creature(source), unit(target)) else {
                return fail;
            };
            let enemy = units::resolve_unit(world, friend).and_then(|f| f.victim).or_else(|| {
                world
                    .managers
                    .creature_mgr
                    .with_creature(friend, |c| c.combat.attackers.iter().next().copied())
                    .flatten()
            });
            let Some(enemy) = enemy else {
                return fail;
            };
            act(world, helper, AIAction::EnterCombat { target_guid: enemy });
            false
        }
        SCRIPT_COMMAND_COMBAT_STOP => {
            let Some(guid) = creature(source) else {
                return fail;
            };
            act(world, guid, AIAction::LeaveCombat);
            false
        }
        SCRIPT_COMMAND_ADD_THREAT => {
            let (Some(guid), Some(target)) = (creature(source), unit(target)) else {
                return fail;
            };
            act(world, guid, AIAction::AddThreat { target_guid: target, amount: 0.0 });
            false
        }
        SCRIPT_COMMAND_SUMMON_OBJECT => {
            let Some(origin) = source.or(target).and_then(|g| super::source_of(world, g)) else {
                return fail;
            };
            summon::summon_gameobject(
                world,
                d[0],
                origin.map_id,
                origin.instance_id,
                Position::new(info.x, info.y, info.z, info.o),
                d[1],
            );
            false
        }
        SCRIPT_COMMAND_SET_GO_STATE => {
            let Some(go) = gameobject(world, 0, source, target) else {
                return fail;
            };
            crate::game::instance::scripts::set_door(world, go, d[0] == 0);
            false
        }
        SCRIPT_COMMAND_DESPAWN_GAMEOBJECT => {
            let Some(go) = gameobject(world, d[0], source, target) else {
                return fail;
            };
            summon::hide_gameobject(world, go);
            false
        }
        SCRIPT_COMMAND_QUEST_CREDIT => {
            let Some(player) = player(source).or(player(target)) else {
                return fail;
            };
            let Some(object) = source.filter(|g| !g.is_player()).or(target.filter(|g| !g.is_player())) else {
                return fail;
            };
            world.systems.quest.handle_kill_credit(player, object.entry(), object);
            false
        }
        SCRIPT_COMMAND_SEND_SCRIPT_EVENT => {
            let Some(receiver) = creature(target) else {
                return fail;
            };
            ai::queue_event(
                world,
                receiver,
                AIEvent::ScriptEvent {
                    event_id: d[0],
                    data: d[1],
                    invoker: source,
                },
            );
            false
        }
        other => {
            tracing::debug!("Script {} uses unsupported command {}", info.id, other);
            false
        }
    }
}

fn valid_attack_target(world: &World, attacker: ObjectGuid, target: ObjectGuid) -> bool {
    let Some(target_info) = units::resolve_unit(world, target) else {
        return false;
    };
    let Some(attacker) = world.managers.creature_mgr.with_creature(attacker, |c| units::Attacker {
        guid: attacker,
        map_id: c.map_id,
        instance_id: c.instance_id,
        faction: c.faction,
        unit_flags: c.unit_flags,
        player_controlled: c.owner_guid.is_some_and(|o| o.is_player()),
    }) else {
        return false;
    };
    units::is_valid_attack_target(world, &attacker, &target_info)
}

/// Set (mode 3), add (1), remove (2) or toggle (0) a creature unit field.
fn set_creature_field(world: &World, guid: ObjectGuid, field: u32, value: u32, mode: u32) -> bool {
    use crate::game::common::update_fields::{
        UNIT_DYNAMIC_FLAGS, UNIT_FIELD_BYTES_1, UNIT_FIELD_FACTIONTEMPLATE, UNIT_FIELD_FLAGS,
        UNIT_NPC_FLAGS,
    };
    let apply = |current: u32| match mode {
        1 => current | value,
        2 => current & !value,
        3 => value,
        _ => {
            if current & value == value {
                current & !value
            } else {
                current | value
            }
        }
    };
    let updated = world.managers.creature_mgr.with_creature_mut(guid, |c| {
        let slot = if field == UNIT_FIELD_FLAGS {
            &mut c.unit_flags
        } else if field == UNIT_NPC_FLAGS {
            &mut c.npc_flags
        } else if field == UNIT_DYNAMIC_FLAGS {
            &mut c.dynamic_flags
        } else if field == UNIT_FIELD_FACTIONTEMPLATE {
            &mut c.faction
        } else if field == UNIT_FIELD_BYTES_1 {
            let new = apply(c.stand_state as u32);
            c.stand_state = new as u8;
            return Some(new);
        } else {
            return None;
        };
        *slot = apply(*slot);
        Some(*slot)
    });
    match updated.flatten() {
        Some(new) => {
            executor::send_unit_field(world, guid, field, new);
            true
        }
        None => {
            tracing::debug!("Script field {} is not a creature field the server tracks", field);
            false
        }
    }
}

fn remove_auras(world: &World, guid: ObjectGuid, spell_id: u32) {
    let spells: Vec<u32> = if spell_id != 0 {
        vec![spell_id]
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.auras.all_auras().map(|a| a.spell_id).collect())
            .unwrap_or_default()
    };
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let world = world.clone();
    handle.spawn(async move {
        for spell in spells {
            let _ = world.systems.auras.remove_spell_auras(guid, spell, &world).await;
        }
    });
}

fn cast_spell(world: &World, info: &ScriptInfo, source: Option<ObjectGuid>, target: Option<ObjectGuid>) -> bool {
    let fail = info.abort_on_failure();
    let Some(caster) = source else {
        return fail;
    };
    let Some(target) = target else {
        return fail;
    };
    let spell_id = info.datalong[0];
    let flags = info.datalong[1] as u16;
    if caster.is_creature() && (target.is_creature() || target.is_player()) {
        return match executor::try_to_cast(world, caster, Some(target), spell_id, flags, 100) {
            CastOutcome::Ok => false,
            _ => fail,
        };
    }
    if caster.is_player() {
        let world = world.clone();
        let triggered = flags & cast_flags::CF_TRIGGERED != 0;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = world
                    .systems
                    .spells
                    .cast_spell(caster, spell_id, Some(target), triggered, &world)
                    .await;
            });
            return false;
        }
    }
    fail
}

fn summon_creature(world: &World, info: &ScriptInfo, source: Option<ObjectGuid>, target: Option<ObjectGuid>) -> bool {
    let fail = info.abort_on_failure();
    let d = &info.datalong;
    let entry = d[0];
    let Some(summoner) = source else {
        return fail;
    };
    if entry == 0 {
        return fail;
    }
    let Some(origin) = super::source_of(world, summoner) else {
        return fail;
    };
    let position = Position::new(info.x, info.y, info.z, info.o);
    let flags = info.dataint[0];
    let despawn = SummonDespawn::from_temp_summon_type(info.dataint[3].max(0) as u32);

    if flags & (SF_SUMMONCREATURE_UNIQUE | SF_SUMMONCREATURE_UNIQUE_TEMP) != 0 {
        let radius = if d[3] != 0 {
            d[3] as f32
        } else {
            (units::distance(&origin.position, &position) + 50.0) * 2.0
        };
        let limit = d[2].max(1) as usize;
        let count_dead = despawn.keeps_corpse();
        let existing = world
            .managers
            .creature_mgr
            .iter_creatures()
            .filter(|e| {
                let c = e.value();
                c.entry == entry
                    && c.map_id == origin.map_id
                    && c.instance_id == origin.instance_id
                    && units::distance(&c.position, &origin.position) <= radius
                    && (count_dead || c.is_alive())
                    && (flags & SF_SUMMONCREATURE_UNIQUE != 0 || summon::is_summon(*e.key()))
            })
            .count();
        if existing >= limit {
            return fail;
        }
    }

    let summoner_creature = summoner.is_creature().then_some(summoner);
    let Some(summoned) = summon::summon_creature(
        world,
        summoner_creature,
        entry,
        origin.map_id,
        origin.instance_id,
        position,
        despawn,
        d[1],
    ) else {
        return fail;
    };
    if flags & SF_SUMMONCREATURE_NULL_AI != 0 {
        world.managers.creature_mgr.with_creature_mut(summoned, |c| {
            c.ai_profile.ai_type = ai::AIType::Null;
        });
    }
    if info.dataint[2] >= 0 {
        if let Some(enemy) = ai::script_target::get_target_by_type(
            world,
            &origin,
            target,
            info.dataint[2] as u8,
            0,
            0,
            None,
        )
        .filter(|g| g.is_creature() || g.is_player())
        {
            act(world, summoned, AIAction::EnterCombat { target_guid: enemy });
        }
    }
    if info.dataint[1] > 0 {
        super::start(world, ScriptTable::Generic, info.dataint[1] as u32, Some(summoned), target);
    }
    false
}

fn move_to(world: &World, info: &ScriptInfo, source: Option<ObjectGuid>, target: Option<ObjectGuid>) -> bool {
    let fail = info.abort_on_failure();
    let Some(guid) = creature(source) else {
        return fail;
    };
    if !alive(world, guid) {
        return fail;
    }
    let target_position = target.and_then(|t| super::source_of(world, t)).map(|s| s.position);
    let mut destination = Position::new(info.x, info.y, info.z, info.o);
    match info.datalong[0] {
        1 => {
            let Some(t) = target_position else {
                return fail;
            };
            destination.x += t.x;
            destination.y += t.y;
            destination.z += t.z;
        }
        2 => {
            let Some(t) = target_position else {
                return fail;
            };
            let angle = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
            destination = Position::new(
                t.x + info.x * angle.cos(),
                t.y + info.x * angle.sin(),
                t.z,
                info.o,
            );
        }
        3 => {
            let angle = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
            let distance = rand::thread_rng().gen_range(0.0..=info.o.max(0.0));
            destination = Position::new(
                info.x + distance * angle.cos(),
                info.y + distance * angle.sin(),
                info.z,
                0.0,
            );
        }
        _ => {}
    }
    let point_id = if info.datalong[3] & SF_MOVETO_POINT_MOVEGEN != 0 {
        info.dataint[0].max(0) as u32
    } else {
        // Plain moves do not report their arrival to scripts.
        u32::MAX
    };
    let options = info.datalong[2];
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        let current = c.position;
        let run = options & MOVE_RUN_MODE != 0;
        let speed = if info.datalong[1] != 0 {
            let distance = units::distance(&current, &destination);
            distance / (info.datalong[1] as f32 / 1000.0)
        } else if run {
            c.run_speed()
        } else {
            c.walk_speed()
        };
        let orientation = if info.o > 0.0 && info.datalong[0] != 3 { info.o } else { -10.0 };
        c.motion_master.move_point(
            point_id,
            destination,
            u32::from(!run),
            speed,
            orientation,
            guid,
            current,
        );
    });
    false
}

fn movement(world: &World, info: &ScriptInfo, source: Option<ObjectGuid>, target: Option<ObjectGuid>) -> bool {
    let fail = info.abort_on_failure();
    let (guid, other) = match (creature(source), creature(target)) {
        (Some(s), _) => (s, unit(target)),
        (None, Some(t)) => (t, unit(source)),
        _ => return fail,
    };
    if !alive(world, guid) {
        return fail;
    }
    let d = &info.datalong;
    let victim = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| c.combat.attacking)
        .flatten();
    let ok = world
        .managers
        .creature_mgr
        .with_creature_mut(guid, |c| {
            let position = c.position;
            if d[3] != 0 {
                c.motion_master.clear(guid);
            }
            match d[0] {
                IDLE_MOTION_TYPE => {
                    c.move_spline.stop();
                    c.motion_master.move_idle(guid);
                }
                RANDOM_MOTION_TYPE => {
                    let home = c.home_position;
                    let speed = c.walk_speed();
                    c.motion_master
                        .move_random(d[1] != 0, home, guid, position, info.x, 0, speed);
                }
                WAYPOINT_MOTION_TYPE => {
                    c.motion_master.reinitialize_patrol_movement(guid);
                }
                CONFUSED_MOTION_TYPE => c.motion_master.move_confused(guid, position),
                CHASE_MOTION_TYPE => {
                    let chase = if d[1] != 0 { victim } else { other };
                    let Some(chase) = chase else {
                        return true;
                    };
                    let reach = c.combat_reach;
                    let speed = c.run_speed();
                    c.motion_master.chase(chase, guid, position, reach, speed);
                }
                HOME_MOTION_TYPE => {
                    let home = c.home_position;
                    let speed = c.run_speed();
                    c.motion_master.return_home(home, guid, position, speed);
                }
                FLEEING_MOTION_TYPE => {
                    let from = if d[1] != 0 { victim } else { other };
                    let Some(from) = from else {
                        return true;
                    };
                    let speed = c.run_speed();
                    c.motion_master.fear(from, d[2], 20.0, guid, position, speed);
                }
                DISTRACT_MOTION_TYPE => c.motion_master.move_distract(guid, d[2]),
                FOLLOW_MOTION_TYPE => {
                    let Some(leader) = other else {
                        return true;
                    };
                    let angle = if info.o < 0.0 {
                        rand::thread_rng().gen_range(0.0..std::f32::consts::TAU)
                    } else {
                        info.o
                    };
                    let speed = c.run_speed();
                    c.motion_master
                        .move_follow(leader, info.x, angle, guid, position, speed);
                    c.following_target = Some(leader);
                }
                CHARGE_MOTION_TYPE => {
                    let Some(at) = other else {
                        return true;
                    };
                    let speed = c.run_speed();
                    c.motion_master
                        .move_charge(at, d[2], d[1] != 0, false, guid, position, speed);
                }
                _ => return false,
            }
            true
        })
        .unwrap_or(false);
    if ok {
        false
    } else {
        tracing::debug!("Script {} uses unsupported motion type {}", info.id, d[0]);
        fail
    }
}

fn start_waypoints(world: &World, guid: ObjectGuid, info: &ScriptInfo) -> bool {
    let (spawn_id, entry) = match world
        .managers
        .creature_mgr
        .with_creature(guid, |c| (c.spawn_id, c.entry))
    {
        Some(ids) => ids,
        None => return false,
    };
    let spawn_id = if info.dataint[0] > 0 { info.dataint[0] as u32 } else { spawn_id };
    let entry = if info.dataint[1] > 0 { info.dataint[1] as u32 } else { entry };
    let Some(path) = world.managers.waypoint_mgr.get_waypoints(spawn_id, entry) else {
        return false;
    };
    let start = info.datalong[1];
    let waypoints: Vec<Waypoint> = path
        .iter()
        .skip_while(|w| start != 0 && w.point_id != start)
        .cloned()
        .collect();
    if waypoints.is_empty() {
        return false;
    }
    let repeat = info.datalong[3] != 0;
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        let position = c.position;
        let speed = c.walk_speed();
        c.motion_master.waypoint(waypoints, repeat, guid, position, speed);
    });
    true
}

/// Make a creature run away: from its victim alone, or towards the nearest
/// friend that can help.
pub fn flee(world: &World, guid: ObjectGuid, seek_assistance: bool) -> bool {
    const SPELL_AURA_PREVENTS_FLEEING: u32 = 92;
    let Some((victim, prevented, position)) = world.managers.creature_mgr.with_creature(guid, |c| {
        (
            c.combat.attacking,
            c.auras.has_aura_type(SPELL_AURA_PREVENTS_FLEEING) || c.cannot_react(),
            c.position,
        )
    }) else {
        return false;
    };
    let Some(victim) = victim else {
        return false;
    };
    if prevented {
        return false;
    }
    let helper = if seek_assistance {
        nearest_helper(world, guid, victim)
    } else {
        None
    };
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        let speed = c.run_speed();
        match helper {
            Some(helper_position) => {
                c.motion_master.move_seek_assistance(
                    helper_position,
                    guid,
                    position,
                    FLEE_DELAY_MS,
                    speed,
                );
                c.combat.attacking = None;
            }
            None => {
                c.ai_state = ai::AIState::Fleeing;
                c.motion_master.fear(victim, FLEE_DELAY_MS, 20.0, guid, position, speed);
            }
        }
    });
    executor::script_text_as(world, guid, CREATURE_FLEE_TEXT, Some(2));
    true
}

/// The nearest idle friend within 30 yards that would answer a call for help.
fn nearest_helper(world: &World, guid: ObjectGuid, victim: ObjectGuid) -> Option<Position> {
    const FLEE_ASSISTANCE_RADIUS: f32 = 30.0;
    let me = units::resolve_unit(world, guid)?;
    let enemy = units::resolve_unit(world, victim)?;
    let map = world.managers.map_mgr.get_map(me.map_id, me.instance_id)?;
    map.get_objects_in_range(me.position, FLEE_ASSISTANCE_RADIUS)
        .into_iter()
        .filter(|g| g.is_creature() && *g != guid)
        .filter_map(|g| units::resolve_unit(world, g))
        .filter(|h| {
            h.alive
                && !h.in_combat
                && !h.is_player_controlled()
                && h.faction == me.faction
                && units::reaction_to(world, h.faction, &enemy) < ReputationRank::Friendly
        })
        .min_by(|a, b| {
            units::distance(&a.position, &me.position)
                .total_cmp(&units::distance(&b.position, &me.position))
        })
        .map(|h| h.position)
}

/// Move a creature to a new position at once.
fn teleport_creature(world: &World, guid: ObjectGuid, destination: Position) {
    let Some((old, map_id, instance_id)) = world.managers.creature_mgr.with_creature_mut(guid, |c| {
        let old = c.position;
        c.move_spline.stop();
        c.position = destination;
        (old, c.map_id, c.instance_id)
    }) else {
        return;
    };
    if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
        map.remove_creature(guid, old);
        map.add_creature(guid, destination);
    }
    world
        .systems
        .creature_movement
        .send_stop_packet(guid, destination, world);
}
