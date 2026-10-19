//! AI Decision Functions - Pure functions for AI decision making
//!
//! These functions are PURE - they have no side effects and hold no locks.
//! They take a snapshot of creature state and return actions to execute.
//!
//! This architecture prevents deadlocks by:
//! 1. Capturing snapshot (brief lock)
//! 2. Making decisions (no locks)
//! 3. Executing actions (deterministic lock ordering)
//!
//! The victim is picked before the decision runs (see `target`), so the
//! functions here only react to it: attack it, switch to it, or evade when
//! victim selection says the fight is lost.

use super::snapshot::{AIDecisionResult, AIInput};
use super::types::{AIAction, AIEvent, AIState, AIType, ReactState};
use oxcore_shared::protocol::ObjectGuid;

/// Home position "close enough" threshold
const HOME_REACHED_DISTANCE: f32 = 3.0;
/// Critters flee this long when scared, and evade this long after the last scare.
pub const ESCAPE_TIMER_MS: u32 = 30000;
/// How far a scared critter runs.
const CRITTER_FLEE_DISTANCE: f32 = 20.0;

/// Main decision entry point
/// Routes to the appropriate AI type's decision function
pub fn decide(input: &AIInput) -> AIDecisionResult {
    // If creature is dead, do nothing
    if !input.snapshot.is_alive {
        return AIDecisionResult::new();
    }

    match input.snapshot.ai_type {
        AIType::Null => decide_null(input),
        AIType::Critter => decide_critter(input),
        AIType::Totem => decide_totem(input),
        AIType::Pet => decide_pet(input),
        AIType::Passive | AIType::Basic | AIType::Guard | AIType::Event | AIType::Lua => {
            decide_basic(input)
        }
    }
}

/// Null AI: never starts or answers a fight, but still evades properly when
/// something put it in combat.
fn decide_null(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();
    result.actions.extend(return_home_behavior(input));
    if input.snapshot.ai_state == AIState::Combat && input.evade_requested {
        result.actions.push(AIAction::EnterEvadeMode);
    }
    result
}

/// Critter AI: never attacks. Damage or a hostile non-damage spell makes it run
/// from the source for thirty seconds; thirty seconds after the last scare it
/// evades.
fn decide_critter(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();
    let mut scared = false;

    for event in &input.events {
        let source = match event {
            AIEvent::DamageTaken { attacker_guid, .. } => Some(*attacker_guid),
            AIEvent::SpellHit {
                caster_guid,
                spell_is_positive,
                spell_is_direct_damage,
                ..
            } if !spell_is_positive && !spell_is_direct_damage => Some(*caster_guid),
            _ => None,
        };
        if let Some(source) = source {
            scared = true;
            if input.snapshot.ai_state != AIState::Fleeing {
                result.actions.push(AIAction::FleeFrom {
                    flee_from_guid: source,
                    distance: CRITTER_FLEE_DISTANCE,
                    duration_ms: ESCAPE_TIMER_MS,
                });
            }
        }
    }

    if scared {
        result.actions.push(AIAction::ResetCritterEscape);
    } else if input.snapshot.in_combat && input.snapshot.ai_state_data.critter_escape_ms == 0 {
        result.actions.push(AIAction::EnterEvadeMode);
    }
    result.actions.extend(return_home_behavior(input));
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

/// Walking home after an evade: arrive, or keep going.
fn return_home_behavior(input: &AIInput) -> Vec<AIAction> {
    match input.snapshot.ai_state {
        AIState::Evading => vec![AIAction::ReturnToSpawn],
        AIState::Returning if input.snapshot.distance_to_home() <= HOME_REACHED_DISTANCE => {
            vec![AIAction::ReachedHome]
        }
        _ => Vec::new(),
    }
}

/// State-based continuous combat behavior (chase, evade, return home).
/// Used by both `decide_basic()` and Lua AI.
pub fn basic_combat_behavior(input: &AIInput) -> Vec<AIAction> {
    let mut actions = Vec::new();

    match input.snapshot.ai_state {
        AIState::Combat => {
            if input.evade_requested {
                actions.push(AIAction::EnterEvadeMode);
                return actions;
            }
            if let Some(victim) = &input.victim {
                if input.snapshot.current_target != Some(victim.guid) {
                    actions.push(AIAction::SetAttackTarget {
                        target_guid: victim.guid,
                    });
                }
                actions.push(AIAction::FaceTarget {
                    target_guid: victim.guid,
                });
            }
        }
        AIState::Evading | AIState::Returning => actions.extend(return_home_behavior(input)),
        AIState::Idle => {
            // Threat put on an idle creature (by a spell, a script) starts the fight.
            if let Some(target) = input.snapshot.threat_list.first() {
                actions.push(AIAction::EnterCombat {
                    target_guid: target.target,
                });
            }
        }
        _ => {}
    }

    actions
}

/// Engage an attacker the way an attacked creature does: start attacking it when
/// it has no victim yet, otherwise just put it on the threat list. A creature
/// walking home after an evade ignores attacks.
fn engage_attacker(input: &AIInput, attacker_guid: ObjectGuid) -> Vec<AIAction> {
    match input.snapshot.ai_state {
        AIState::Evading | AIState::Returning | AIState::Dead => Vec::new(),
        AIState::Combat if input.snapshot.current_target.is_some() => vec![AIAction::AddThreat {
            target_guid: attacker_guid,
            amount: 0.0,
        }],
        _ => attack_start(input, attacker_guid),
    }
}

/// Start attacking `target_guid`, unless the creature is passive.
fn attack_start(input: &AIInput, target_guid: ObjectGuid) -> Vec<AIAction> {
    if input.snapshot.ai_state_data.react_state == ReactState::Passive {
        return Vec::new();
    }
    vec![AIAction::EnterCombat { target_guid }]
}

/// Process a single event for Basic AI.
/// Also used by Lua AI for core combat management (enter combat, add threat).
pub fn process_basic_event(input: &AIInput, event: &AIEvent) -> Vec<AIAction> {
    match event {
        // The aggro scan has already applied every proximity rule.
        AIEvent::UnitInRange {
            unit_guid,
            is_hostile: true,
            ..
        } => match input.snapshot.ai_state {
            AIState::Idle => attack_start(input, *unit_guid),
            // Joining the fight without switching victims (dungeons).
            AIState::Combat => vec![AIAction::AddThreat {
                target_guid: *unit_guid,
                amount: 0.0,
            }],
            _ => Vec::new(),
        },
        // A hit, a miss or a hostile spell: the damage itself already counted as threat.
        AIEvent::AttackedBy { attacker_guid } | AIEvent::DamageTaken { attacker_guid, .. } => {
            engage_attacker(input, *attacker_guid)
        }
        AIEvent::Engage { target_guid } | AIEvent::AssistanceRequested { target_guid, .. } => {
            engage_attacker(input, *target_guid)
        }
        _ => Vec::new(),
    }
}

/// Totem AI: stationary, never melees, never chases.
fn decide_totem(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();
    if input.snapshot.ai_state == AIState::Combat && input.evade_requested {
        result.actions.push(AIAction::EnterEvadeMode);
    }
    result
}

/// Pet AI: follows the owner (set up by the pet system) and defends itself
/// and its owner; it never acquires targets by proximity.
fn decide_pet(input: &AIInput) -> AIDecisionResult {
    let mut result = AIDecisionResult::new();
    for event in &input.events {
        match event {
            AIEvent::AttackedBy { attacker_guid } | AIEvent::DamageTaken { attacker_guid, .. }
                if input.snapshot.current_target.is_none()
                    && input.snapshot.ai_state_data.react_state != ReactState::Passive =>
            {
                result.actions.push(AIAction::EnterCombat {
                    target_guid: *attacker_guid,
                });
            }
            AIEvent::Engage { target_guid }
                if input.snapshot.ai_state_data.react_state != ReactState::Passive =>
            {
                result.actions.push(AIAction::SetAttackTarget {
                    target_guid: *target_guid,
                });
                result.actions.push(AIAction::EnterCombat {
                    target_guid: *target_guid,
                });
            }
            _ => {}
        }
    }
    if input.snapshot.ai_state == AIState::Combat {
        if let Some(victim) = &input.victim {
            if input.snapshot.current_target != Some(victim.guid) {
                result.actions.push(AIAction::SetAttackTarget {
                    target_guid: victim.guid,
                });
            }
        } else if input.evade_requested {
            // A pet without a victim goes back to following its owner.
            result.actions.push(AIAction::PetFollowOwner);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::ai::snapshot::{CreatureSnapshot, TargetSnapshot, ThreatEntry};
    use crate::game::creature::ai::types::{AIStateData, CreatureAiProfile};
    use oxcore_shared::protocol::{HighGuid, ObjectGuid, Position};

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
            ai_state_data: AIStateData::new(),
            combat_reach: 1.5,
            spells: [0; 4],
            current_mana: 0,
            max_mana: 0,
            level: 10,
            unit_class: 0,
            auras: Vec::new(),
            profile: CreatureAiProfile::default(),
            static_flags1: 0,
            owner: None,
        }
    }

    fn make_input(snapshot: CreatureSnapshot, events: Vec<AIEvent>) -> AIInput {
        AIInput {
            snapshot,
            events,
            diff_ms: 50,
            nearby_targets: Vec::new(),
            victim: None,
            evade_requested: false,
        }
    }

    fn target(guid: ObjectGuid) -> TargetSnapshot {
        TargetSnapshot {
            guid,
            position: Position::new(100.0, 100.0, 0.0, 0.0),
            is_alive: true,
            is_player: true,
            health_pct: 1.0,
            has_mana: false,
        }
    }

    fn has_action(actions: &[AIAction], check: impl Fn(&AIAction) -> bool) -> bool {
        actions.iter().any(check)
    }

    #[test]
    fn proximity_aggro_while_idle_attacks() {
        let target = player_guid(10);
        let input = make_input(make_snapshot(AIState::Idle), vec![]);
        let event = AIEvent::UnitInRange {
            unit_guid: target,
            distance: 15.0,
            is_hostile: true,
            is_player: true,
        };
        let actions = process_basic_event(&input, &event);
        assert!(has_action(
            &actions,
            |a| matches!(a, AIAction::EnterCombat { target_guid } if *target_guid == target)
        ));
    }

    #[test]
    fn proximity_aggro_in_combat_only_joins_the_threat_list() {
        let target = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Combat);
        snapshot.current_target = Some(player_guid(1));
        let input = make_input(snapshot, vec![]);
        let event = AIEvent::UnitInRange {
            unit_guid: target,
            distance: 15.0,
            is_hostile: true,
            is_player: true,
        };
        let actions = process_basic_event(&input, &event);
        assert!(matches!(
            actions.as_slice(),
            [AIAction::AddThreat { target_guid, amount }] if *target_guid == target && *amount == 0.0
        ));
    }

    #[test]
    fn passive_creatures_never_start_a_fight() {
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.ai_state_data.react_state = ReactState::Passive;
        let input = make_input(snapshot, vec![]);
        let event = AIEvent::AttackedBy {
            attacker_guid: player_guid(10),
        };
        assert!(process_basic_event(&input, &event).is_empty());
    }

    #[test]
    fn damage_does_not_add_threat_a_second_time() {
        let attacker = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Combat);
        snapshot.current_target = Some(attacker);
        let input = make_input(snapshot, vec![]);
        let event = AIEvent::DamageTaken {
            attacker_guid: attacker,
            damage: 100,
            spell_id: None,
            school: 0,
        };
        let actions = process_basic_event(&input, &event);
        assert!(!has_action(&actions, |a| matches!(
            a,
            AIAction::AddThreat { amount, .. } if *amount > 0.0
        )));
    }

    #[test]
    fn damage_while_idle_starts_the_fight() {
        let attacker = player_guid(10);
        let input = make_input(make_snapshot(AIState::Idle), vec![]);
        let event = AIEvent::DamageTaken {
            attacker_guid: attacker,
            damage: 50,
            spell_id: None,
            school: 0,
        };
        let actions = process_basic_event(&input, &event);
        assert!(has_action(
            &actions,
            |a| matches!(a, AIAction::EnterCombat { target_guid } if *target_guid == attacker)
        ));
    }

    #[test]
    fn evading_creatures_ignore_attacks() {
        for state in [AIState::Evading, AIState::Returning] {
            let input = make_input(make_snapshot(state), vec![]);
            let event = AIEvent::DamageTaken {
                attacker_guid: player_guid(10),
                damage: 30,
                spell_id: None,
                school: 0,
            };
            assert!(process_basic_event(&input, &event).is_empty());
        }
    }

    #[test]
    fn combat_switches_to_the_selected_victim() {
        let victim = player_guid(2);
        let mut snapshot = make_snapshot(AIState::Combat);
        snapshot.current_target = Some(player_guid(1));
        let mut input = make_input(snapshot, vec![]);
        input.victim = Some(target(victim));
        let actions = basic_combat_behavior(&input);
        assert!(has_action(
            &actions,
            |a| matches!(a, AIAction::SetAttackTarget { target_guid } if *target_guid == victim)
        ));
    }

    #[test]
    fn evade_request_evades() {
        let mut input = make_input(make_snapshot(AIState::Combat), vec![]);
        input.evade_requested = true;
        assert!(has_action(&basic_combat_behavior(&input), |a| matches!(
            a,
            AIAction::EnterEvadeMode
        )));
    }

    #[test]
    fn no_victim_without_evade_request_holds() {
        let input = make_input(make_snapshot(AIState::Combat), vec![]);
        assert!(basic_combat_behavior(&input).is_empty());
    }

    #[test]
    fn evading_walks_home_and_arrives() {
        let input = make_input(make_snapshot(AIState::Evading), vec![]);
        assert!(has_action(&basic_combat_behavior(&input), |a| matches!(
            a,
            AIAction::ReturnToSpawn
        )));
        let input = make_input(make_snapshot(AIState::Returning), vec![]);
        assert!(has_action(&basic_combat_behavior(&input), |a| matches!(
            a,
            AIAction::ReachedHome
        )));
    }

    #[test]
    fn idle_with_threat_starts_the_fight() {
        let target = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.threat_list.push(ThreatEntry {
            target,
            threat: 5.0,
        });
        let input = make_input(snapshot, vec![]);
        assert!(has_action(
            &basic_combat_behavior(&input),
            |a| matches!(a, AIAction::EnterCombat { target_guid } if *target_guid == target)
        ));
    }

    #[test]
    fn critter_flees_from_damage_for_thirty_seconds() {
        let attacker = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.ai_type = AIType::Critter;
        let input = make_input(
            snapshot,
            vec![AIEvent::DamageTaken {
                attacker_guid: attacker,
                damage: 1,
                spell_id: None,
                school: 0,
            }],
        );
        let result = decide(&input);
        assert!(has_action(&result.actions, |a| matches!(
            a,
            AIAction::FleeFrom { flee_from_guid, duration_ms: ESCAPE_TIMER_MS, .. } if *flee_from_guid == attacker
        )));
    }

    #[test]
    fn critter_ignores_a_missed_swing() {
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.ai_type = AIType::Critter;
        let input = make_input(
            snapshot,
            vec![AIEvent::AttackedBy {
                attacker_guid: player_guid(10),
            }],
        );
        assert!(decide(&input).actions.is_empty());
    }

    #[test]
    fn critter_spell_hit_flees_from_hostile_non_damage_spell() {
        let caster = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.ai_type = AIType::Critter;
        let input = make_input(
            snapshot,
            vec![AIEvent::SpellHit {
                caster_guid: caster,
                spell_id: 123,
                spell_is_positive: false,
                spell_is_direct_damage: false,
            }],
        );
        assert!(has_action(&decide(&input).actions, |a| matches!(
            a,
            AIAction::FleeFrom { flee_from_guid, .. } if *flee_from_guid == caster
        )));
    }

    #[test]
    fn critter_spell_hit_ignores_positive_and_direct_damage_spells() {
        let caster = player_guid(10);
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.ai_type = AIType::Critter;
        let input = make_input(
            snapshot,
            vec![
                AIEvent::SpellHit {
                    caster_guid: caster,
                    spell_id: 123,
                    spell_is_positive: true,
                    spell_is_direct_damage: false,
                },
                AIEvent::SpellHit {
                    caster_guid: caster,
                    spell_id: 456,
                    spell_is_positive: false,
                    spell_is_direct_damage: true,
                },
            ],
        );
        assert!(decide(&input).actions.is_empty());
    }

    #[test]
    fn critter_evades_after_the_escape_timer() {
        let mut snapshot = make_snapshot(AIState::Fleeing);
        snapshot.ai_type = AIType::Critter;
        snapshot.in_combat = true;
        snapshot.ai_state_data.critter_escape_ms = 0;
        let input = make_input(snapshot, vec![]);
        assert!(has_action(&decide(&input).actions, |a| matches!(
            a,
            AIAction::EnterEvadeMode
        )));
    }

    #[test]
    fn null_ai_never_answers_attacks() {
        let mut snapshot = make_snapshot(AIState::Idle);
        snapshot.ai_type = AIType::Null;
        let input = make_input(
            snapshot,
            vec![AIEvent::AttackedBy {
                attacker_guid: player_guid(10),
            }],
        );
        assert!(decide(&input).actions.is_empty());
    }
}
