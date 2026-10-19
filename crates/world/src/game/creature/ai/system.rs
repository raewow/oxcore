//! AI System - Main update loop and event processing
//!
//! This module coordinates AI updates for all creatures:
//! 1. Captures snapshots (brief lock)
//! 2. Picks the victim of creatures in combat (threat list, threat area)
//! 3. Runs pure decision functions (no locks)
//! 4. Executes actions (deterministic lock ordering)
//! 5. Runs the in-combat timers: call for help, combat pulse, unreachable
//!    target, creature spell lists

use super::assist;
use super::decision::decide;
use super::executor::{execute_actions, update_spells_list};
use super::snapshot::{AIInput, CreatureSnapshot, ThreatEntry};
use super::target::{select_hostile_target, HostileSelection};
use super::types::{AIAction, AIEvent, AIState, AIType};
use super::units::now_ms;
use crate::core::lua::bridge::{
    ai_input_to_lua_snapshot, invoke_callback, lua_actions_to_ai_actions, map_ai_event_to_callback,
    LuaCallback,
};
use crate::core::lua::scripts::CreatureScriptState;
use crate::game::common::creature_flags::CREATURE_FLAG_EXTRA_NO_UNREACHABLE_EVADE;
use crate::game::creature::movement::generators::ChaseMovementGenerator;
use crate::game::creature::movement::MovementGeneratorType;
use crate::World;
use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

/// A fighting creature away from home calls for help this often.
const CALL_FOR_HELP_INTERVAL_MS: u32 = 1000;
/// Calls for help only happen this far from home.
const CALL_FOR_HELP_MIN_HOME_DISTANCE: f32 = 10.0;
/// Zone combat and the hard leash are checked on this cadence.
const COMBAT_PULSE_INTERVAL_MS: u32 = 3000;
/// A creature evades after its chase target has been unreachable this long.
const UNREACHABLE_EVADE_MS: u32 = 24_000;
/// Longest step the AI clock takes after a stall.
const MAX_AI_DIFF_MS: u32 = 10_000;

/// Per-creature Lua script state (timers, phase, custom data).
/// Stored globally since CreatureScriptState is not part of the Creature struct.
fn lua_script_states() -> &'static DashMap<ObjectGuid, CreatureScriptState> {
    static STATES: OnceLock<DashMap<ObjectGuid, CreatureScriptState>> = OnceLock::new();
    STATES.get_or_init(DashMap::new)
}

/// Drop a creature's Lua script state (despawn, respawn).
pub fn forget_lua_state(creature_guid: ObjectGuid) {
    lua_script_states().remove(&creature_guid);
}

/// Time since the previous AI update, measured rather than assumed so timers
/// keep pace when the world tick runs late.
fn elapsed_since_last_update(world: &World) -> u32 {
    static LAST: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    let now = Instant::now();
    let mut last = LAST.get_or_init(|| Mutex::new(None)).lock();
    let diff = last
        .map(|previous| now.duration_since(previous).as_millis() as u32)
        .unwrap_or(world.update_interval.as_millis() as u32);
    *last = Some(now);
    diff.min(MAX_AI_DIFF_MS)
}

/// Process AI for all creatures that need updates
/// Called from the world update loop
pub fn update_creature_ai(world: &World) -> anyhow::Result<()> {
    let diff_ms = elapsed_since_last_update(world);
    // Creatures in combat, walking home, fleeing, with pending events or
    // assistance, and Lua-scripted creatures (whose OnUpdate runs out of combat).
    let creatures_to_update: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|entry| {
            let creature = entry.value();
            let guid = *entry.key();
            creature.combat.in_combat
                || creature.ai_state != AIState::Idle
                || creature.ai_state_data.pending_assistance.is_some()
                || world.systems.ai_event_queue.has_events(guid)
                || world.managers.lua_mgr.has_creature_ai(creature.entry)
                || (creature.ai_profile.ai_type == AIType::Event
                    && creature.is_alive()
                    && super::event_ai::has_events(creature.entry))
        })
        .map(|entry| *entry.key())
        .collect();

    for creature_guid in creatures_to_update {
        update_single_creature(world, creature_guid, diff_ms);
    }

    Ok(())
}

/// Process AI for a single creature
fn update_single_creature(world: &World, creature_guid: ObjectGuid, diff_ms: u32) {
    let now = now_ms();

    // Timers tick on the live creature so cooldowns started by this update's
    // actions are never overwritten afterwards.
    let ticked = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| {
            c.ai_state_data.tick(diff_ms);
            // A finished flee hands the creature back to its fight, or to idle.
            // Scared critters keep fleeing until their escape timer evades them.
            if c.ai_state == AIState::Fleeing
                && c.motion_master.active_generator() != MovementGeneratorType::Fleeing
                && !(c.ai_profile.ai_type == AIType::Critter && c.combat.in_combat)
            {
                c.ai_state = if c.combat.in_combat {
                    AIState::Combat
                } else {
                    AIState::Idle
                };
            }
        });
    if ticked.is_none() {
        return;
    }

    assist::process_pending_assistance(world, creature_guid, diff_ms);

    let Some(mut snapshot) = capture_snapshot(world, creature_guid) else {
        return;
    };

    let lua_mgr = &world.managers.lua_mgr;
    let has_lua = lua_mgr.is_initialized() && lua_mgr.has_creature_ai(snapshot.entry);
    if has_lua {
        snapshot.ai_type = AIType::Lua;
    }

    let mut events = world.systems.ai_event_queue.take_for(creature_guid);
    if let Some(target) = pet_defence_target(world, &snapshot) {
        events.push(AIEvent::Engage { target_guid: target });
    }
    if events.iter().any(|e| matches!(e, AIEvent::Died { .. })) {
        world
            .systems
            .linking
            .on_event(world, creature_guid, crate::game::coordination::LinkEvent::Die);
        crate::game::creature::formation::on_member_died(world, creature_guid);
    }

    let selection = if snapshot.is_alive && snapshot.ai_state == AIState::Combat {
        select_hostile_target(world, creature_guid, now)
    } else {
        HostileSelection {
            targets: resolve_threat_targets(world, &snapshot),
            ..Default::default()
        }
    };
    let has_victim = selection.victim.is_some();

    let input = AIInput {
        snapshot,
        events,
        diff_ms,
        nearby_targets: selection.targets,
        victim: selection.victim,
        evade_requested: selection.evade,
    };

    if has_lua {
        update_lua_creature(world, creature_guid, &input);
    } else {
        let result = decide(&input);
        if !result.actions.is_empty() {
            execute_actions(world, creature_guid, result.actions);
        }
        if input.snapshot.ai_type == AIType::Event {
            let has_victim = world
                .managers
                .creature_mgr
                .with_creature(creature_guid, |c| c.combat.attacking.is_some())
                .unwrap_or(false);
            super::event_ai::update(
                world,
                creature_guid,
                input.snapshot.entry,
                &input.events,
                diff_ms,
                has_victim,
            );
        }
    }

    if input.snapshot.is_alive {
        update_combat_timers(world, creature_guid, &input, has_victim, diff_ms);
    }
}

/// The in-combat work that runs after the decision: calling for help, the
/// three-second combat pulse, giving up on unreachable targets, and casting
/// from the creature spell list.
fn update_combat_timers(
    world: &World,
    creature_guid: ObjectGuid,
    input: &AIInput,
    has_victim: bool,
    diff_ms: u32,
) {
    #[derive(Default)]
    struct Due {
        call_for_help: Option<f32>,
        pulse: bool,
        leash_broken: bool,
        unreachable_evade: bool,
        zone_combat: bool,
    }

    let due = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| {
            let mut due = Due::default();
            if c.ai_state != AIState::Combat {
                return due;
            }
            let has_victim = has_victim && c.combat.attacking.is_some();
            let data = &mut c.ai_state_data;

            if has_victim {
                if data.call_for_help_timer <= diff_ms {
                    data.call_for_help_timer = CALL_FOR_HELP_INTERVAL_MS;
                    let home = c.home_position;
                    let (dx, dy, dz) = (
                        c.position.x - home.x,
                        c.position.y - home.y,
                        c.position.z - home.z,
                    );
                    if (dx * dx + dy * dy + dz * dz).sqrt() > CALL_FOR_HELP_MIN_HOME_DISTANCE {
                        due.call_for_help = Some(c.ai_profile.call_for_help_range);
                    }
                } else {
                    data.call_for_help_timer -= diff_ms;
                }
            }

            if data.combat_pulse_timer <= diff_ms {
                data.combat_pulse_timer = COMBAT_PULSE_INTERVAL_MS;
                due.pulse = true;
                let leash = c.ai_profile.leash_range;
                if leash > 0.0 {
                    if let Some(start) = data.combat_start {
                        let (dx, dy, dz) = (
                            c.position.x - start.x,
                            c.position.y - start.y,
                            c.position.z - start.z,
                        );
                        due.leash_broken = (dx * dx + dy * dy + dz * dz).sqrt() > leash;
                    }
                }
                due.zone_combat = data.combat_with_zone;
            } else {
                data.combat_pulse_timer -= diff_ms;
            }

            let chasing_unreachable = c.owner_guid.is_none()
                && !c.ai_profile.has_extra_flag(CREATURE_FLAG_EXTRA_NO_UNREACHABLE_EVADE)
                && c.motion_master.active_generator() == MovementGeneratorType::Chase
                && c
                    .motion_master
                    .get_generator_mut(MovementGeneratorType::Chase)
                    .and_then(|g| g.as_any_mut().downcast_mut::<ChaseMovementGenerator>())
                    .is_some_and(|chase| !chase.is_reachable());
            let data = &mut c.ai_state_data;
            if chasing_unreachable {
                data.unreachable_ms = data.unreachable_ms.saturating_add(diff_ms);
                due.unreachable_evade = data.unreachable_ms > UNREACHABLE_EVADE_MS;
            } else {
                data.unreachable_ms = 0;
            }
            due
        })
        .unwrap_or_default();

    if due.leash_broken || due.unreachable_evade {
        execute_actions(world, creature_guid, vec![AIAction::EnterEvadeMode]);
        return;
    }
    if let Some(range) = due.call_for_help {
        assist::call_for_help(world, creature_guid, range);
    }
    if due.pulse && due.zone_combat {
        assist::set_in_combat_with_zone(world, creature_guid, false);
    }

    let casts_spell_list = matches!(
        input.snapshot.ai_type,
        AIType::Basic | AIType::Guard | AIType::Event | AIType::Passive
    );
    if has_victim && casts_spell_list {
        update_spells_list(world, creature_guid, diff_ms);
    }
}

/// Process Lua AI for a single creature.
///
/// This handles the full Lua lifecycle:
/// 1. Get or create per-creature script state (timers, phase, custom data)
/// 2. Convert AIInput to LuaCreatureSnapshot
/// 3. Invoke event-based callbacks (OnEnterCombat, OnDeath, etc.)
/// 4. Invoke OnUpdate callback every tick
/// 5. Convert returned LuaActions to AIActions and execute
fn update_lua_creature(world: &World, creature_guid: ObjectGuid, input: &AIInput) {
    let lua_mgr = &world.managers.lua_mgr;
    let entry = input.snapshot.entry;

    let Some(lua_ai) = lua_mgr.get_creature_ai(entry) else {
        return;
    };

    // A creature that walked home or respawned starts its script over.
    let resets = input
        .events
        .iter()
        .any(|e| matches!(e, AIEvent::ReachedHome | AIEvent::Respawned));
    if resets {
        lua_script_states().remove(&creature_guid);
    }

    // Get or create per-creature script state
    let mut state = lua_script_states()
        .entry(creature_guid)
        .or_insert_with(CreatureScriptState::new)
        .clone();

    // Update timers and combat time
    state.update_timers(input.diff_ms);
    if input.snapshot.in_combat {
        state.add_combat_time(input.diff_ms);
    }
    if input
        .events
        .iter()
        .any(|e| matches!(e, AIEvent::CombatStarted { .. }))
    {
        state.reset_combat();
    }

    // Build Lua snapshot from AI input + script state
    let mut lua_snapshot = ai_input_to_lua_snapshot(input, &state);

    // Populate instance data if creature is in an instance with a script
    let map_id = input.snapshot.map_id;
    let instance_id = input.snapshot.instance_id;
    if instance_id > 0 && lua_mgr.has_instance_script(map_id) {
        let inst_state = lua_mgr.get_instance_state(map_id, instance_id);
        lua_snapshot.instance_data = inst_state.data.clone();
    }

    // Populate nearby creatures (within 100 yards, same map and same copy).
    lua_snapshot.nearby_creatures = nearby_creatures(world, &input.snapshot);

    let mut all_actions: Vec<AIAction> = Vec::new();

    // Run basic combat management only when alive (matches decide() early-out)
    if input.snapshot.is_alive {
        for event in &input.events {
            all_actions.extend(super::decision::process_basic_event(input, event));
        }
        all_actions.extend(super::decision::basic_combat_behavior(input));
    }

    let mut callbacks: Vec<LuaCallback> = input
        .events
        .iter()
        .filter_map(map_ai_event_to_callback)
        .collect();
    if resets {
        callbacks.push(LuaCallback::OnReset);
    }

    for callback in callbacks {
        let lua_actions =
            lua_mgr.with_lua(|lua| invoke_callback(&lua_ai, lua, &lua_snapshot, &callback));
        if !lua_actions.is_empty() {
            all_actions.extend(lua_actions_to_ai_actions(lua_actions, &mut state));
        }
    }

    if input.snapshot.is_alive {
        let lua_actions = lua_mgr.with_lua(|lua| lua_ai.on_update(lua, &lua_snapshot));
        if !lua_actions.is_empty() {
            all_actions.extend(lua_actions_to_ai_actions(lua_actions, &mut state));
        }
    }

    if !all_actions.is_empty() {
        execute_actions(world, creature_guid, all_actions);
    }

    lua_script_states().insert(creature_guid, state);
}

/// A pet without a victim defends its owner: a defensive pet attacks whoever
/// attacks the owner, an aggressive one also joins the owner's own attack.
fn pet_defence_target(world: &World, snapshot: &CreatureSnapshot) -> Option<ObjectGuid> {
    if snapshot.ai_type != AIType::Pet || snapshot.current_target.is_some() || !snapshot.is_alive {
        return None;
    }
    let react = snapshot.ai_state_data.react_state;
    if react == super::types::ReactState::Passive {
        return None;
    }
    let owner = snapshot.owner.filter(|o| o.is_player())?;
    world
        .managers
        .player_mgr
        .with_player(owner, |p| {
            if !p.combat.in_combat {
                return None;
            }
            let attacker = p.combat.attackers.iter().copied().find(|a| *a != snapshot.guid);
            match react {
                super::types::ReactState::Aggressive => p.combat.attack_target.or(attacker),
                _ => attacker,
            }
        })
        .flatten()
        .filter(|t| super::units::resolve_unit(world, *t).is_some_and(|u| u.alive))
}

/// Creatures within 100 yards on the same map *and* in the same dungeon copy.
/// Matching the instance as well as the map keeps creatures in different copies
/// of a dungeon from discovering each other at identical coordinates.
fn nearby_creatures(
    world: &World,
    snapshot: &CreatureSnapshot,
) -> Vec<crate::core::lua::snapshot::NearbyCreatureEntry> {
    use crate::core::lua::snapshot::NearbyCreatureEntry;
    const RANGE: f32 = 100.0;

    let candidates: Vec<ObjectGuid> = match world
        .managers
        .map_mgr
        .get_map(snapshot.map_id, snapshot.instance_id)
    {
        Some(map) => map
            .get_objects_in_range(snapshot.position, RANGE)
            .into_iter()
            .filter(|g| g.is_creature())
            .collect(),
        None => world
            .managers
            .creature_mgr
            .iter_creatures()
            .map(|e| *e.key())
            .collect(),
    };

    let mut nearby = Vec::new();
    for guid in candidates {
        if guid == snapshot.guid {
            continue;
        }
        let Some(entry) = world.managers.creature_mgr.with_creature(guid, |other| {
            if other.map_id != snapshot.map_id || other.instance_id != snapshot.instance_id {
                return None;
            }
            let distance = super::units::distance(&snapshot.position, &other.position);
            (distance <= RANGE).then(|| NearbyCreatureEntry {
                guid: other.guid,
                entry: other.entry,
                distance,
                is_alive: other.is_alive(),
            })
        }) else {
            continue;
        };
        nearby.extend(entry);
    }
    nearby
}

/// Snapshots of the threat-list targets of a creature that is not fighting.
fn resolve_threat_targets(
    world: &World,
    snapshot: &CreatureSnapshot,
) -> Vec<super::snapshot::TargetSnapshot> {
    snapshot
        .threat_list
        .iter()
        .filter_map(|entry| super::units::resolve_unit(world, entry.target))
        .map(|info| super::snapshot::TargetSnapshot {
            guid: info.guid,
            position: info.position,
            is_alive: info.alive,
            is_player: info.is_player,
            health_pct: info.health_pct,
            has_mana: info.has_mana,
        })
        .collect()
}

/// Capture read-only snapshot of creature state
fn capture_snapshot(world: &World, guid: ObjectGuid) -> Option<CreatureSnapshot> {
    world.managers.creature_mgr.with_creature(guid, |creature| {
        let health_pct = if creature.max_health > 0 {
            creature.current_health as f32 / creature.max_health as f32
        } else {
            0.0
        };

        let threat_list: Vec<ThreatEntry> = creature
            .threat_manager
            .get_threat_list()
            .into_iter()
            .map(|(target, threat)| ThreatEntry { target, threat })
            .collect();

        CreatureSnapshot {
            guid,
            entry: creature.entry,
            map_id: creature.map_id,
            instance_id: creature.instance_id,
            position: creature.position,
            home_position: creature.home_position,
            ai_state: creature.ai_state,
            ai_type: creature.ai_profile.ai_type,
            // The unit we are actually swinging at, not the top threat entry:
            // the Combat branch compares the two to decide when to retarget.
            current_target: creature.combat.attacking,
            threat_list,
            health_pct,
            current_health: creature.current_health,
            max_health: creature.max_health,
            in_combat: creature.combat.in_combat,
            is_alive: creature.is_alive(),
            attack_timer_ready: creature.is_attack_ready(),
            ai_state_data: creature.ai_state_data.clone(),
            combat_reach: creature.combat_reach,
            spells: creature.spells,
            current_mana: creature.current_mana,
            max_mana: creature.max_mana,
            level: creature.level,
            unit_class: 0,
            auras: creature
                .auras
                .all_auras()
                .map(|aura| {
                    (
                        aura.spell_id,
                        aura.remaining_duration_ms().unwrap_or(0),
                        aura.stack_count,
                    )
                })
                .collect(),
            profile: creature.ai_profile.clone(),
            static_flags1: creature.static_flags1,
            owner: creature.owner_guid,
        }
    })
}

/// Queue an AI event for a creature
/// Called by other systems (combat, damage, etc.) to trigger AI reactions
pub fn queue_event(world: &World, creature_guid: ObjectGuid, event: AIEvent) {
    world.systems.ai_event_queue.push(creature_guid, event);
}

/// Process a specific AI event for a creature immediately
/// Used when an event needs immediate processing (e.g., on damage)
pub fn process_ai_event(world: &World, creature_guid: ObjectGuid, event: AIEvent) {
    queue_event(world, creature_guid, event);
    let diff_ms = world.update_interval.as_millis() as u32;
    update_single_creature(world, creature_guid, diff_ms);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::{Creature, CreatureTemplate};
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::Position;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_world() -> World {
        let pool = || {
            PgPoolOptions::new()
                .connect_lazy("postgres://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        let databases = Arc::new(Databases {
            world: pool(),
            character: pool(),
            auth: pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn test_template(entry: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: "Test Creature".into(),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 0,
            model_id_1: 0,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 0,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
            gold_min: 0,
            gold_max: 0,
            loot_id: 0,
            pickpocket_loot_id: 0,
            skinning_loot_id: 0,
            ai: Default::default(),
        }
    }

    fn add_creature(
        world: &World,
        counter: u32,
        map_id: u32,
        instance_id: u32,
        position: Position,
    ) -> ObjectGuid {
        const ENTRY: u32 = 1;
        let guid = ObjectGuid::new_creature(ENTRY, counter);
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            ENTRY,
            counter,
            position,
            map_id,
            instance_id,
            &test_template(ENTRY),
            1,
            None,
        ));
        guid
    }

    #[tokio::test]
    async fn snapshot_keeps_the_creatures_instance_identity() {
        let world = test_world();
        let guid = add_creature(&world, 1, 409, 7, Position::new(1.0, 2.0, 3.0, 0.0));

        let snapshot = capture_snapshot(&world, guid).expect("creature snapshot");
        assert_eq!(snapshot.map_id, 409);
        assert_eq!(
            snapshot.instance_id, 7,
            "the snapshot must carry the creature's own copy, not instance zero"
        );
    }

    #[tokio::test]
    async fn nearby_creatures_never_cross_dungeon_copies() {
        let world = test_world();
        let pos = Position::new(0.0, 0.0, 0.0, 0.0);
        let boss = add_creature(&world, 1, 409, 7, pos);
        // Same coordinates in another copy of the same map must stay invisible.
        add_creature(&world, 2, 409, 8, pos);
        // Same copy, within range: the only creature the boss may discover.
        let add = add_creature(&world, 3, 409, 7, Position::new(1.0, 0.0, 0.0, 0.0));
        // A different map at the same coordinates must stay invisible too.
        add_creature(&world, 4, 530, 7, pos);

        let snapshot = capture_snapshot(&world, boss).expect("creature snapshot");
        let nearby = nearby_creatures(&world, &snapshot);

        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].guid, add);
    }
}
