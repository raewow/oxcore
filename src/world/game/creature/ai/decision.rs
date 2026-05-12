//! AI Decision Functions - Pure functions for AI decision making
//!
//! These functions are PURE - they have no side effects and hold no locks.
//! They take a snapshot of creature state and return actions to execute.
//!
//! This architecture prevents deadlocks by:
//! 1. Capturing snapshot (brief lock)
//! 2. Making decisions (no locks)
//! 3. Executing actions (deterministic lock ordering)

use super::snapshot::{AIDecisionResult, AIInput, CreatureSnapshot, TargetSnapshot};
use super::types::{AIAction, AIEvent, AIState, AIStateData, AIType, MovementType, ReactState};
use crate::shared::protocol::ObjectGuid;

/// Maximum chase distance before evade
const MAX_CHASE_DISTANCE: f32 = 45.0;
/// Distance at which to start returning
const EVADE_DISTANCE: f32 = 50.0;
/// Home position "close enough" threshold
const HOME_REACHED_DISTANCE: f32 = 3.0;
/// Default melee attack range (unused, kept for reference - now uses combat reach formula)
const _MELEE_RANGE: f32 = 5.0;
/// How often to update AI (milliseconds)
const AI_UPDATE_INTERVAL: u32 = 500;

/// Main decision entry point
/// Routes to the appropriate AI type's decision function
pub fn decide(input: &AIInput) -> AIDecisionResult {
    // If creature is dead, do nothing
    if !input.snapshot.is_alive {
        return AIDecisionResult::new();
    }

    // Update state data timers
    let mut state_data = input.snapshot.ai_state_data.clone();
    state_data.tick(input.diff_ms);

    // Process events and make decisions based on AI type
    let mut result = match input.snapshot.ai_type {
        AIType::Null => decide_null(input),
        AIType::Passive => decide_passive(input),
        AIType::Critter => decide_critter(input),
        AIType::Basic => decide_basic(input),
        AIType::Totem => decide_totem(input),
        AIType::Pet => decide_pet(input),
        AIType::Guard => decide_guard(input),
        AIType::Event => decide_event(input),
        AIType::Lua => decide_lua(input),
    };

    // Update state data in result
    result.updated_state_data = Some(state_data);
    result
}

/// Null AI - does nothing
fn decide_null(_input: &AIInput) -> AIDecisionResult {
    AIDecisionResult::new()
}

/// Passive AI - does not fight back, only returns to spawn if too far
fn decide_passive(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();

    // Check if too far from home
    if input.snapshot.distance_to_home() > EVADE_DISTANCE {
        result.actions.push(AIAction::ReturnToSpawn);
    }

    // Process events
    for event in &input.events {
        match event {
            AIEvent::ReachedHome => {
                result.actions.push(AIAction::LeaveCombat);
            }
            _ => {}
        }
    }

    result
}

/// Critter AI - flees when damaged, wanders randomly
fn decide_critter(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();

    for event in &input.events {
        match event {
            AIEvent::DamageTaken { attacker_guid, .. } => {
                // Flee from attacker
                if let Some(attacker) = input
                    .nearby_targets
                    .iter()
                    .find(|t| t.guid == *attacker_guid)
                {
                    result.actions.push(AIAction::FleeFrom {
                        flee_from_guid: *attacker_guid,
                        distance: 20.0,
                        duration_ms: 5000,
                    });
                }
            }
            AIEvent::UpdateTick { .. } => {
                // Random wandering when idle
                if input.snapshot.ai_state == AIState::Idle {
                    // TODO: Add random movement timer
                }
            }
            _ => {}
        }
    }

    result
}

/// Basic AI - standard combat AI with threat-based targeting
fn decide_basic(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();

    // Process events first
    for event in &input.events {
        let event_actions = process_basic_event(input, event);
        result.actions.extend(event_actions);
    }

    // State-based continuous behavior
    let state_actions = basic_combat_behavior(input);
    result.actions.extend(state_actions);

    result
}

/// State-based continuous combat behavior (chase, evade, return home, spell casting).
/// Used by both `decide_basic()` and Lua AI.
pub fn basic_combat_behavior(input: &AIInput) -> Vec<AIAction> {
    let mut actions = Vec::new();

    match input.snapshot.ai_state {
        AIState::Combat => {
            // Check evade distance
            if input.snapshot.distance_to_home() > EVADE_DISTANCE {
                actions.push(AIAction::EnterEvadeMode);
                return actions;
            }

            // Get top threat target
            if let Some(target_guid) = input.snapshot.get_top_threat_target() {
                // Check if target is valid
                if let Some(target) = input.nearby_targets.iter().find(|t| t.guid == target_guid) {
                    if target.is_alive {
                        // Set attack target if different
                        if input.snapshot.current_target != Some(target_guid) {
                            actions.push(AIAction::SetAttackTarget { target_guid });
                        }

                        // Check if in melee range using combat reach formula
                        use crate::world::game::combat::melee_range;
                        let melee_reach = melee_range::get_melee_reach(
                            input.snapshot.combat_reach,
                            melee_range::DEFAULT_COMBAT_REACH, // target (player) reach
                            false, // no leeway for AI
                        );
                        let distance = input.snapshot.distance_to_2d(&target.position);
                        // Try to cast a spell first (if any are available and off cooldown)
                        let spell_cast = try_select_spell(input, target_guid, distance);
                        if let Some(spell_action) = spell_cast {
                            actions.push(spell_action);
                        }

                        if distance <= melee_reach {
                            // In range — face target
                            // No StopMovement here: chase generator handles stopping
                            // Melee attacks are handled directly by combat_update.rs
                            // (matches vmangos where DoMeleeAttackIfReady is separate from AI decisions)
                            actions.push(AIAction::FaceTarget { target_guid });
                        }
                        // No else MoveToTarget — chase generator handles approach
                    } else {
                        // Target is dead, clear it
                        actions.push(AIAction::ClearAttackTarget);
                    }
                } else {
                    // Target not in nearby targets list (out of range or gone)
                    // Keep current target but try to find new one
                    if input.snapshot.target_count() == 0 {
                        actions.push(AIAction::EnterEvadeMode);
                    }
                }
            } else {
                // No targets, evade
                actions.push(AIAction::EnterEvadeMode);
            }
        }
        AIState::Evading => {
            // Immediately transition to returning
            actions.push(AIAction::ReturnToSpawn);
        }
        AIState::Returning => {
            // Check if reached home
            if input.snapshot.distance_to_home() <= HOME_REACHED_DISTANCE {
                actions.push(AIAction::LeaveCombat);
            } else {
                // Continue returning home
                actions.push(AIAction::MoveTo {
                    position: input.snapshot.home_position,
                    movement_type: MovementType::Run,
                });
            }
        }
        AIState::Idle => {
            // Check if we should be in combat (has targets)
            if input.snapshot.has_targets() {
                if let Some(target) = input.snapshot.get_top_threat_target() {
                    actions.push(AIAction::EnterCombat {
                        target_guid: target,
                    });
                }
            }
        }
        _ => {}
    }

    actions
}

/// Process a single event for Basic AI.
/// Also used by Lua AI for core combat management (enter combat, add threat).
pub fn process_basic_event(input: &AIInput, event: &AIEvent) -> Vec<AIAction> {
    let mut actions = Vec::new();

    match event {
        AIEvent::UnitInRange {
            unit_guid,
            is_hostile,
            ..
        } => {
            // Only aggro if we're idle and the unit is hostile
            if input.snapshot.ai_state == AIState::Idle && *is_hostile {
                actions.push(AIAction::EnterCombat {
                    target_guid: *unit_guid,
                });
                actions.push(AIAction::SetAttackTarget {
                    target_guid: *unit_guid,
                });
                actions.push(AIAction::AddThreat {
                    target_guid: *unit_guid,
                    amount: 1.0, // Initial threat from aggro
                });
            }
        }
        AIEvent::DamageTaken {
            attacker_guid,
            damage,
            ..
        } => {
            match input.snapshot.ai_state {
                AIState::Idle => {
                    // Enter combat and fight back
                    actions.push(AIAction::EnterCombat {
                        target_guid: *attacker_guid,
                    });
                    actions.push(AIAction::SetAttackTarget {
                        target_guid: *attacker_guid,
                    });
                    actions.push(AIAction::AddThreat {
                        target_guid: *attacker_guid,
                        amount: *damage as f32,
                    });
                }
                AIState::Combat => {
                    // Add threat to attacker
                    actions.push(AIAction::AddThreat {
                        target_guid: *attacker_guid,
                        amount: *damage as f32,
                    });
                }
                AIState::Returning | AIState::Evading => {
                    // Re-engage if attacked while returning
                    actions.push(AIAction::EnterCombat {
                        target_guid: *attacker_guid,
                    });
                    actions.push(AIAction::SetAttackTarget {
                        target_guid: *attacker_guid,
                    });
                    actions.push(AIAction::AddThreat {
                        target_guid: *attacker_guid,
                        amount: *damage as f32,
                    });
                }
                _ => {}
            }
        }
        AIEvent::TargetKilled { victim_guid } => {
            // Clear the killed target
            if input.snapshot.current_target == Some(*victim_guid) {
                actions.push(AIAction::ClearAttackTarget);
            }
            actions.push(AIAction::ModifyThreat {
                target_guid: *victim_guid,
                amount: 0.0,
                is_percent: false,
            });
        }
        AIEvent::ReachedHome => {
            actions.push(AIAction::LeaveCombat);
            actions.push(AIAction::ClearThreatList);
            actions.push(AIAction::ClearAttackTarget);
        }
        _ => {}
    }

    actions
}

/// Totem AI - stationary spell caster
fn decide_totem(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();

    // Totems don't move, just cast spells
    for event in &input.events {
        match event {
            AIEvent::UpdateTick { .. } => {
                // Check for spells to cast
                // TODO: Implement totem spell casting
            }
            _ => {}
        }
    }

    result
}

/// Pet AI - follows owner and assists
fn decide_pet(input: &AIInput) -> AIDecisionResult {
    // TODO: Implement pet AI
    AIDecisionResult::new()
}

/// Guard AI - like basic but calls for help
fn decide_guard(input: &AIInput) -> AIDecisionResult {
    // Start with basic AI behavior
    let mut result = decide_basic(input);

    // Add guard-specific behavior
    for event in &input.events {
        match event {
            AIEvent::DamageTaken { .. } => {
                // Call for help when damaged
                if input.snapshot.ai_state == AIState::Combat {
                    result.actions.push(AIAction::CallForHelp { radius: 30.0 });
                }
            }
            _ => {}
        }
    }

    result
}

/// Event AI - database-driven scripted behavior
fn decide_event(input: &AIInput) -> AIDecisionResult {
    // TODO: Implement event AI (creature_ai_scripts table)
    decide_basic(input)
}

/// Lua AI - script-driven behavior
fn decide_lua(input: &AIInput) -> AIDecisionResult {
    // TODO: Implement Lua script integration
    // For now, fall back to basic AI
    decide_basic(input)
}

/// Try to select a spell for the creature to cast at its target.
///
/// Iterates creature's spell list (spell1-4), picks the first spell that is:
/// - Non-zero (slot has a spell)
/// - Off cooldown (checked via AIStateData)
/// - Caster has enough mana
///
/// Returns a CastSpell action if a spell was selected, None otherwise.
fn try_select_spell(input: &AIInput, target_guid: ObjectGuid, _distance: f32) -> Option<AIAction> {
    let snapshot = &input.snapshot;

    for &spell_id in &snapshot.spells {
        if spell_id == 0 {
            continue;
        }

        // Check cooldown
        if !snapshot.ai_state_data.is_spell_ready(spell_id) {
            continue;
        }

        // Spell is available - return cast action
        // Mana check and range validation happen in the executor
        return Some(AIAction::CastSpell {
            spell_id,
            target_guid: Some(target_guid),
            target_position: None,
            triggered: false,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::{HighGuid, ObjectGuid, Position};
    use crate::world::game::creature::ai::snapshot::ThreatEntry;

    fn player_guid(id: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, id)
    }

    fn creature_guid(entry: u32, counter: u32) -> ObjectGuid {
        ObjectGuid::new_creature(entry, counter)
    }

    fn make_snapshot(ai_state: AIState) -> CreatureSnapshot {
        CreatureSnapshot {
            guid: creature_guid(38, 1),
            entry: 38,
            map_id: 0,
            instance_id: 0,
            position: Position::new(100.0, 100.0, 0.0, 0.0),
            home_position: Position::new(100.0, 100.0, 0.0, 0.0),
            ai_state,
            ai_type: AIType::Basic,
            current_target: None,
            threat_list: Vec::new(),
            health_pct: 1.0,
            current_health: 1000,
            max_health: 1000,
            in_combat: ai_state == AIState::Combat,
            is_alive: true,
            attack_timer_ready: false,
            ai_state_data: AIStateData::default(),
            combat_reach: 1.5,
            spells: [0; 4],
            current_mana: 0,
            max_mana: 0,
            level: 10,
            unit_class: 0,
            auras: Vec::new(),
        }
    }

    fn make_input(snapshot: CreatureSnapshot, events: Vec<AIEvent>, nearby_targets: Vec<TargetSnapshot>) -> AIInput {
        AIInput {
            snapshot,
            events,
            diff_ms: 50,
            nearby_targets,
        }
    }

    fn make_target(guid: ObjectGuid, pos: Position, is_alive: bool) -> TargetSnapshot {
        TargetSnapshot {
            guid,
            position: pos,
            is_alive,
            is_player: true,
            health_pct: 1.0,
            has_mana: false,
        }
    }

    fn has_action(actions: &[AIAction], check: impl Fn(&AIAction) -> bool) -> bool {
        actions.iter().any(check)
    }

    // ========== process_basic_event tests ==========

    #[test]
    fn test_unit_in_range_while_idle_enters_combat() {
        let target = player_guid(10);
        let input = make_input(make_snapshot(AIState::Idle), vec![], vec![]);
        let event = AIEvent::UnitInRange {
            unit_guid: target,
            distance: 15.0,
            is_hostile: true,
            is_player: true,
        };

        let actions = process_basic_event(&input, &event);

        assert!(has_action(&actions, |a| matches!(a, AIAction::EnterCombat { .. })));
        assert!(has_action(&actions, |a| matches!(a, AIAction::SetAttackTarget { target_guid } if *target_guid == target)));
        assert!(has_action(&actions, |a| matches!(a, AIAction::AddThreat { .. })));
    }

    #[test]
    fn test_unit_in_range_while_combat_ignored() {
        let target = player_guid(10);
        let input = make_input(make_snapshot(AIState::Combat), vec![], vec![]);
        let event = AIEvent::UnitInRange {
            unit_guid: target,
            distance: 15.0,
            is_hostile: true,
            is_player: true,
        };

        let actions = process_basic_event(&input, &event);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_unit_in_range_non_hostile_ignored() {
        let target = player_guid(10);
        let input = make_input(make_snapshot(AIState::Idle), vec![], vec![]);
        let event = AIEvent::UnitInRange {
            unit_guid: target,
            distance: 15.0,
            is_hostile: false,
            is_player: true,
        };

        let actions = process_basic_event(&input, &event);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_damage_taken_while_idle_enters_combat() {
        let attacker = player_guid(10);
        let input = make_input(make_snapshot(AIState::Idle), vec![], vec![]);
        let event = AIEvent::DamageTaken {
            attacker_guid: attacker,
            damage: 50,
            spell_id: None,
            school: 0,
        };

        let actions = process_basic_event(&input, &event);

        assert!(has_action(&actions, |a| matches!(a, AIAction::EnterCombat { .. })));
        assert!(has_action(&actions, |a| matches!(a, AIAction::SetAttackTarget { target_guid } if *target_guid == attacker)));
        assert!(has_action(&actions, |a| matches!(a, AIAction::AddThreat { amount, .. } if *amount == 50.0)));
    }

    #[test]
    fn test_damage_taken_while_combat_adds_threat() {
        let attacker = player_guid(10);
        let input = make_input(make_snapshot(AIState::Combat), vec![], vec![]);
        let event = AIEvent::DamageTaken {
            attacker_guid: attacker,
            damage: 100,
            spell_id: None,
            school: 0,
        };

        let actions = process_basic_event(&input, &event);

        assert!(!has_action(&actions, |a| matches!(a, AIAction::EnterCombat { .. })));
        assert!(has_action(&actions, |a| matches!(a, AIAction::AddThreat { amount, .. } if *amount == 100.0)));
    }

    #[test]
    fn test_damage_taken_while_returning_reengages() {
        let attacker = player_guid(10);
        let input = make_input(make_snapshot(AIState::Returning), vec![], vec![]);
        let event = AIEvent::DamageTaken {
            attacker_guid: attacker,
            damage: 30,
            spell_id: None,
            school: 0,
        };

        let actions = process_basic_event(&input, &event);
        assert!(has_action(&actions, |a| matches!(a, AIAction::EnterCombat { .. })));
    }

    #[test]
    fn test_reached_home_leaves_combat() {
        let input = make_input(make_snapshot(AIState::Returning), vec![], vec![]);
        let event = AIEvent::ReachedHome;

        let actions = process_basic_event(&input, &event);

        assert!(has_action(&actions, |a| matches!(a, AIAction::LeaveCombat)));
        assert!(has_action(&actions, |a| matches!(a, AIAction::ClearThreatList)));
        assert!(has_action(&actions, |a| matches!(a, AIAction::ClearAttackTarget)));
    }

    #[test]
    fn test_target_killed_clears_target() {
        let victim = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Combat);
        snapshot.current_target = Some(victim);
        let input = make_input(snapshot, vec![], vec![]);
        let event = AIEvent::TargetKilled { victim_guid: victim };

        let actions = process_basic_event(&input, &event);

        assert!(has_action(&actions, |a| matches!(a, AIAction::ClearAttackTarget)));
        assert!(has_action(&actions, |a| matches!(a, AIAction::ModifyThreat { target_guid, .. } if *target_guid == victim)));
    }

    // ========== basic_combat_behavior tests ==========

    #[test]
    fn test_combat_state_evades_when_too_far() {
        let mut snapshot = make_snapshot(AIState::Combat);
        // Place creature far from home
        snapshot.position = Position::new(200.0, 200.0, 0.0, 0.0);
        snapshot.threat_list.push(ThreatEntry { target: player_guid(10), threat: 10.0 });
        let input = make_input(snapshot, vec![], vec![]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::EnterEvadeMode)));
    }

    #[test]
    fn test_combat_state_faces_target_in_melee_range() {
        let target = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Combat);
        snapshot.current_target = Some(target);
        snapshot.threat_list.push(ThreatEntry { target, threat: 10.0 });
        // Target at same position (distance 0, within melee range)
        let target_snap = make_target(target, Position::new(100.0, 100.0, 0.0, 0.0), true);
        let input = make_input(snapshot, vec![], vec![target_snap]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::FaceTarget { target_guid } if *target_guid == target)));
    }

    #[test]
    fn test_combat_state_evades_with_no_targets() {
        let snapshot = make_snapshot(AIState::Combat);
        // Empty threat list
        let input = make_input(snapshot, vec![], vec![]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::EnterEvadeMode)));
    }

    #[test]
    fn test_combat_state_clears_dead_target() {
        let target = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Combat);
        snapshot.current_target = Some(target);
        snapshot.threat_list.push(ThreatEntry { target, threat: 10.0 });
        let target_snap = make_target(target, Position::new(100.0, 100.0, 0.0, 0.0), false); // dead
        let input = make_input(snapshot, vec![], vec![target_snap]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::ClearAttackTarget)));
    }

    #[test]
    fn test_evading_state_returns_to_spawn() {
        let snapshot = make_snapshot(AIState::Evading);
        let input = make_input(snapshot, vec![], vec![]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::ReturnToSpawn)));
    }

    #[test]
    fn test_returning_state_leaves_combat_at_home() {
        let snapshot = make_snapshot(AIState::Returning);
        // Position is already at home (same default position)
        let input = make_input(snapshot, vec![], vec![]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::LeaveCombat)));
    }

    #[test]
    fn test_idle_state_enters_combat_if_has_targets() {
        let target = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.threat_list.push(ThreatEntry { target, threat: 5.0 });
        let input = make_input(snapshot, vec![], vec![]);

        let actions = basic_combat_behavior(&input);
        assert!(has_action(&actions, |a| matches!(a, AIAction::EnterCombat { target_guid } if *target_guid == target)));
    }

    #[test]
    fn test_idle_state_no_action_without_targets() {
        let snapshot = make_snapshot(AIState::Idle);
        let input = make_input(snapshot, vec![], vec![]);

        let actions = basic_combat_behavior(&input);
        assert!(actions.is_empty());
    }

    // ========== decide_basic integration test ==========

    #[test]
    fn test_decide_basic_enters_combat_on_aggro() {
        let target = player_guid(10);
        let events = vec![AIEvent::UnitInRange {
            unit_guid: target,
            distance: 15.0,
            is_hostile: true,
            is_player: true,
        }];
        let input = make_input(make_snapshot(AIState::Idle), events, vec![]);

        let result = decide_basic(&input);

        assert!(has_action(&result.actions, |a| matches!(a, AIAction::EnterCombat { .. })));
        assert!(has_action(&result.actions, |a| matches!(a, AIAction::SetAttackTarget { .. })));
    }
}
