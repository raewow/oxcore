//! AI System - Main update loop and event processing
//!
//! This module coordinates AI updates for all creatures:
//! 1. Captures snapshots (brief lock)
//! 2. Runs pure decision functions (no locks)
//! 3. Executes actions (deterministic lock ordering)

use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use crate::world::core::lua::bridge::{ai_input_to_lua_snapshot, invoke_callback, lua_actions_to_ai_actions, map_ai_event_to_callback};
use crate::world::core::lua::scripts::CreatureScriptState;
use super::decision::decide;
use super::executor::execute_actions;
use super::snapshot::{AIInput, CreatureSnapshot, TargetSnapshot, ThreatEntry};
use super::types::{AIAction, AIEvent, AIState, AIType};
use dashmap::DashMap;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Per-creature Lua script state (timers, phase, custom data).
/// Stored globally since CreatureScriptState is not part of the Creature struct.
fn lua_script_states() -> &'static DashMap<ObjectGuid, CreatureScriptState> {
    static STATES: OnceLock<DashMap<ObjectGuid, CreatureScriptState>> = OnceLock::new();
    STATES.get_or_init(DashMap::new)
}

/// Process AI for all creatures that need updates
/// Called from the world update loop
pub fn update_creature_ai(world: &World) -> anyhow::Result<()> {
    // Get all creatures that need AI updates:
    // - Creatures in combat
    // - Creatures with pending events
    // - Creatures in special states (returning, fleeing)
    // - Lua-scripted creatures (so OnUpdate fires out-of-combat for patrol/event scripts)
    let creatures_to_update: Vec<ObjectGuid> = world.managers.creature_mgr
        .iter_creatures()
        .filter(|entry| {
            let creature = entry.value();
            let guid = *entry.key();
            creature.combat.in_combat
                || matches!(creature.ai_state, AIState::Returning | AIState::Fleeing | AIState::Evading)
                || world.systems.ai_event_queue.has_events(guid)
                // Lua-scripted creatures always tick so OnUpdate can run timers out-of-combat
                || world.managers.lua_mgr.has_creature_ai(creature.entry)
        })
        .map(|entry| *entry.key())
        .collect();

    for creature_guid in creatures_to_update {
        update_single_creature(world, creature_guid);
    }

    Ok(())
}

/// Process AI for a single creature
fn update_single_creature(world: &World, creature_guid: ObjectGuid) {
    // 1. Capture snapshot (brief lock)
    let Some(mut snapshot) = capture_snapshot(world, creature_guid) else {
        return;
    };

    // Check if this creature has a Lua script registered
    let lua_mgr = &world.managers.lua_mgr;
    let has_lua = lua_mgr.is_initialized() && lua_mgr.has_creature_ai(snapshot.entry);

    if has_lua {
        // Override AI type so the rest of the system knows
        snapshot.ai_type = AIType::Lua;
    }

    // 2. Get pending events
    let events = world.systems.ai_event_queue.take_for(creature_guid);

    // 3. Get nearby targets for decision making
    let nearby_targets = get_nearby_targets(world, &snapshot);

    // 4. Build AI input
    let diff_ms = world.update_interval.as_millis() as u32;
    let input = AIInput {
        snapshot,
        events,
        diff_ms,
        nearby_targets,
    };

    // 5. For Lua AI, bypass decide() and call Lua scripts directly
    if has_lua {
        update_lua_creature(world, creature_guid, &input);
        return;
    }

    // 6. Pure decision function for non-Lua AI (no locks held)
    let result = decide(&input);

    // 7. Execute actions
    if !result.actions.is_empty() {
        tracing::debug!(
            "[AI] Creature {:?} executing {} actions",
            creature_guid,
            result.actions.len()
        );
        execute_actions(world, creature_guid, result.actions);
    }

    // 8. Update AI state data if provided
    if let Some(state_data) = result.updated_state_data {
        world.managers.creature_mgr.with_creature_mut(creature_guid, |creature| {
            creature.ai_state_data = state_data;
        });
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

    // Build Lua snapshot from AI input + script state
    let mut lua_snapshot = ai_input_to_lua_snapshot(input, &state);

    // Populate instance data if creature is in an instance with a script
    let map_id = input.snapshot.map_id;
    let instance_id = input.snapshot.instance_id;
    if instance_id > 0 && lua_mgr.has_instance_script(map_id) {
        let inst_state = lua_mgr.get_instance_state(map_id, instance_id);
        lua_snapshot.instance_data = inst_state.data.clone();
    }

    // Populate nearby creatures (within 100 yards, same map)
    {
        use crate::world::core::lua::snapshot::NearbyCreatureEntry;
        let creature_pos = input.snapshot.position;
        let creature_map = input.snapshot.map_id;
        let max_dist = 100.0f32;
        let max_dist_sq = max_dist * max_dist;

        for entry in world.managers.creature_mgr.iter_creatures() {
            let other = entry.value();
            if other.guid == creature_guid || other.map_id != creature_map {
                continue;
            }
            let dx = creature_pos.x - other.position.x;
            let dy = creature_pos.y - other.position.y;
            let dz = creature_pos.z - other.position.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq <= max_dist_sq {
                lua_snapshot.nearby_creatures.push(NearbyCreatureEntry {
                    guid: other.guid,
                    entry: other.entry,
                    distance: dist_sq.sqrt(),
                    is_alive: other.is_alive(),
                });
            }
        }
    }

    let mut all_actions: Vec<AIAction> = Vec::new();

    if !input.events.is_empty() {
        tracing::info!(
            "[LuaAI] Creature {:?} (entry {}) processing {} events, ai_state={:?}",
            creature_guid, entry, input.events.len(), input.snapshot.ai_state
        );
    }

    // Run basic combat management only when alive (matches decide() early-out)
    if input.snapshot.is_alive {
        // Enter combat on aggro/damage, add threat, etc.
        for event in &input.events {
            let basic_actions = super::decision::process_basic_event(input, event);
            all_actions.extend(basic_actions);
        }

        // State-based continuous behavior (chase, evade, face target, return home)
        let state_actions = super::decision::basic_combat_behavior(input);
        all_actions.extend(state_actions);
    }

    // Process event-based Lua callbacks (OnEnterCombat, OnDeath, OnDamageTaken, etc.)
    for event in &input.events {
        if let Some(callback) = map_ai_event_to_callback(event) {
            tracing::info!(
                "[LuaAI] Creature {:?} (entry {}) invoking Lua callback for event",
                creature_guid, entry
            );
            let lua_actions = lua_mgr.with_lua(|lua| {
                invoke_callback(&lua_ai, lua, &lua_snapshot, &callback)
            });
            tracing::info!(
                "[LuaAI] Callback returned {} lua actions",
                lua_actions.len()
            );

            if !lua_actions.is_empty() {
                let ai_actions = lua_actions_to_ai_actions(lua_actions, &mut state);
                all_actions.extend(ai_actions);
            }
        }
    }

    // Always call OnUpdate when in combat or alive
    if input.snapshot.is_alive {
        let lua_actions = lua_mgr.with_lua(|lua| {
            lua_ai.on_update(lua, &lua_snapshot)
        });

        if !lua_actions.is_empty() {
            let ai_actions = lua_actions_to_ai_actions(lua_actions, &mut state);
            all_actions.extend(ai_actions);
        }
    }

    // Execute all collected actions
    if !all_actions.is_empty() {
        tracing::debug!(
            "[LuaAI] Creature {:?} (entry {}) executing {} actions",
            creature_guid,
            entry,
            all_actions.len()
        );
        execute_actions(world, creature_guid, all_actions);
    }

    // Save updated script state back
    lua_script_states().insert(creature_guid, state);
}

/// Capture read-only snapshot of creature state
fn capture_snapshot(world: &World, guid: ObjectGuid) -> Option<CreatureSnapshot> {
    world.managers.creature_mgr.with_creature_mut(guid, |creature| {
        let health_pct = if creature.max_health > 0 {
            creature.current_health as f32 / creature.max_health as f32
        } else {
            0.0
        };

        // Build threat list from ThreatManager (Phase 5)
        let threat_list: Vec<ThreatEntry> = creature.threat_manager
            .get_threat_list()
            .into_iter()
            .map(|(target, threat)| ThreatEntry { target, threat })
            .collect();

        // Determine AI type from creature template
        // Use cached creature_type (no template lookup needed - eliminates nested lock)
        let ai_type = AIType::from_creature_template(
            creature.creature_type as u32,
            0, // TODO: Get static_flags from template
        );

        CreatureSnapshot {
            guid,
            entry: creature.entry,
            map_id: creature.map_id,
            instance_id: 0, // TODO: get from instance system
            position: creature.position,
            home_position: creature.home_position,
            ai_state: creature.ai_state,
            ai_type,
            current_target: creature.threat_manager.get_victim(),
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
            unit_class: 0, // TODO: cache unit_class on Creature from template
            auras: creature.auras.clone(),
        }
    })
}

/// Get snapshots of nearby potential targets
fn get_nearby_targets(world: &World, snapshot: &CreatureSnapshot) -> Vec<TargetSnapshot> {
    let mut targets = Vec::new();

    // Get targets from threat list
    for entry in &snapshot.threat_list {
        // Try to get player position
        if let Some(pos) = world.managers.player_mgr.get_player_position(entry.target) {
            let (is_alive, health_pct, has_mana) = world.managers.player_mgr
                .with_player(entry.target, |p| {
                    let hp = if p.stats.max_health > 0 { p.stats.health as f32 / p.stats.max_health as f32 } else { 0.0 };
                    let mana = p.stats.max_mana > 0;
                    (p.is_alive(), hp, mana)
                })
                .unwrap_or((false, 0.0, false));

            targets.push(TargetSnapshot {
                guid: entry.target,
                position: pos,
                is_alive,
                is_player: true,
                health_pct,
                has_mana,
            });
        }
    }

    targets
}

/// Queue an AI event for a creature
/// Called by other systems (combat, damage, etc.) to trigger AI reactions
pub fn queue_event(world: &World, creature_guid: ObjectGuid, event: AIEvent) {
    world.systems.ai_event_queue.push(creature_guid, event);
}

/// Process a specific AI event for a creature immediately
/// Used when an event needs immediate processing (e.g., on damage)
pub fn process_ai_event(
    world: &World,
    creature_guid: ObjectGuid,
    event: AIEvent,
) {
    // Queue the event
    queue_event(world, creature_guid, event);

    // Process immediately
    update_single_creature(world, creature_guid);
}
