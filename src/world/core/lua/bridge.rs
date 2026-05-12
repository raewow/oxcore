//! Bridge layer between the AI system and Lua scripting.
//!
//! This module converts between the AI system's types (AIInput, AIAction, AIEvent)
//! and the Lua system's types (LuaCreatureSnapshot, LuaAction).

use super::actions::LuaAction;
use super::scripts::CreatureScriptState;
use super::snapshot::{LuaCreatureSnapshot, ThreatEntry as LuaThreatEntry};
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::game::creature::ai::{
    AIAction, AIEvent, AIInput, CombatEndReason, MovementType,
};
use std::collections::HashMap;

/// Convert AI system's input into a Lua creature snapshot.
pub fn ai_input_to_lua_snapshot(
    input: &AIInput,
    state: &CreatureScriptState,
) -> LuaCreatureSnapshot {
    let snap = &input.snapshot;

    // Convert threat list with available data
    let threat_list: Vec<LuaThreatEntry> = snap
        .threat_list
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            // Try to find target info from nearby_targets
            let (distance, is_player, health_pct, has_mana) = input
                .nearby_targets
                .iter()
                .find(|t| t.guid == entry.target)
                .map(|t| {
                    let dist = snap.distance_to(&t.position);
                    (dist, t.is_player, t.health_pct, t.has_mana)
                })
                .unwrap_or((0.0, false, 0.0, false));

            LuaThreatEntry {
                guid: entry.target,
                threat: entry.threat,
                distance,
                is_player,
                health_pct,
                has_mana,
            }
        })
        .collect();

    LuaCreatureSnapshot {
        guid: snap.guid,
        entry: snap.entry,
        map_id: snap.map_id,
        instance_id: snap.instance_id,
        x: snap.position.x,
        y: snap.position.y,
        z: snap.position.z,
        o: snap.position.o,
        spawn_x: snap.home_position.x,
        spawn_y: snap.home_position.y,
        spawn_z: snap.home_position.z,
        spawn_o: snap.home_position.o,
        health: snap.current_health,
        max_health: snap.max_health,
        power: snap.current_mana,
        max_power: snap.max_mana,
        power_type: if snap.max_mana > 0 { 0 } else { 1 }, // 0=mana, 1=rage (inferred)
        level: snap.level,
        is_alive: snap.is_alive,
        is_in_combat: snap.in_combat,
        current_target: snap.current_target,
        is_casting: false, // TODO: track casting state on creature
        phase: state.phase,
        timers: state.timers.clone(),
        custom_data: state.custom_data.clone(),
        combat_time_ms: state.combat_time_ms,
        threat_list,
        auras: snap.auras.clone(),
        instance_data: HashMap::new(), // Populated by caller if instance has a script
        nearby_creatures: Vec::new(), // Populated by caller from creature manager scan
        events: Vec::new(), // Events are dispatched via callbacks, not the snapshot
        diff_ms: input.diff_ms,
        summoned_creatures: state.summoned_creatures.clone(),
    }
}

/// Represents a Lua callback to invoke, derived from an AI event.
pub enum LuaCallback {
    OnEnterCombat,
    OnDeath { killer_guid: Option<ObjectGuid> },
    OnKill { victim_guid: ObjectGuid },
    OnSpellHit { spell_id: u32, caster_guid: ObjectGuid },
    OnDamageTaken { attacker_guid: ObjectGuid, damage: u32, spell_id: Option<u32> },
    OnEvade,
    OnReset,
    OnSpawn,
    JustRespawned,
    JustReachedHome,
    MovementInform { movement_type: u32, point_id: u32 },
    JustSummoned { summoned_guid: ObjectGuid, entry: u32 },
    SummonedCreatureJustDied { summoned_guid: ObjectGuid },
    SummonedCreatureDespawn { summoned_guid: ObjectGuid },
    MoveInLineOfSight { unit_guid: ObjectGuid, is_hostile: bool },
    HealedBy { healer_guid: ObjectGuid, amount: u32, spell_id: Option<u32> },
    SpellHitTarget { target_guid: ObjectGuid, spell_id: u32 },
}

/// Map an AI event to a Lua callback.
pub fn map_ai_event_to_callback(event: &AIEvent) -> Option<LuaCallback> {
    match event {
        AIEvent::CombatStarted { .. } => Some(LuaCallback::OnEnterCombat),
        AIEvent::Died { killer_guid } => Some(LuaCallback::OnDeath {
            killer_guid: *killer_guid,
        }),
        AIEvent::TargetKilled { victim_guid } => Some(LuaCallback::OnKill {
            victim_guid: *victim_guid,
        }),
        AIEvent::SpellHit { caster_guid, spell_id } => Some(LuaCallback::OnSpellHit {
            spell_id: *spell_id,
            caster_guid: *caster_guid,
        }),
        AIEvent::DamageTaken { attacker_guid, damage, spell_id, .. } => {
            Some(LuaCallback::OnDamageTaken {
                attacker_guid: *attacker_guid,
                damage: *damage,
                spell_id: *spell_id,
            })
        }
        AIEvent::CombatEnded { reason } => match reason {
            CombatEndReason::Evaded | CombatEndReason::TargetsOutOfRange => {
                Some(LuaCallback::OnEvade)
            }
            _ => None,
        },
        AIEvent::Spawned => Some(LuaCallback::OnSpawn),
        AIEvent::Respawned => Some(LuaCallback::JustRespawned),
        AIEvent::ReachedHome => Some(LuaCallback::JustReachedHome),
        AIEvent::MovementComplete { point_id } => Some(LuaCallback::MovementInform {
            movement_type: 0, // TODO: pass movement type when available
            point_id: *point_id,
        }),
        AIEvent::SummonedCreature { summoned_guid, entry } => Some(LuaCallback::JustSummoned {
            summoned_guid: *summoned_guid,
            entry: *entry,
        }),
        AIEvent::SummonedCreatureDied { summoned_guid } => {
            Some(LuaCallback::SummonedCreatureJustDied {
                summoned_guid: *summoned_guid,
            })
        }
        AIEvent::SummonedCreatureDespawned { summoned_guid } => {
            Some(LuaCallback::SummonedCreatureDespawn {
                summoned_guid: *summoned_guid,
            })
        }
        AIEvent::UnitInLineOfSight { unit_guid, is_hostile, .. } => {
            Some(LuaCallback::MoveInLineOfSight {
                unit_guid: *unit_guid,
                is_hostile: *is_hostile,
            })
        }
        AIEvent::HealingReceived { healer_guid, amount, spell_id } => {
            Some(LuaCallback::HealedBy {
                healer_guid: *healer_guid,
                amount: *amount,
                spell_id: *spell_id,
            })
        }
        AIEvent::SpellHitTarget { target_guid, spell_id } => {
            Some(LuaCallback::SpellHitTarget {
                target_guid: *target_guid,
                spell_id: *spell_id,
            })
        }
        // Events that don't have direct Lua callbacks
        AIEvent::DamageDealt { .. }
        | AIEvent::SpellInterrupted { .. }
        | AIEvent::UnitInRange { .. }
        | AIEvent::TimerExpired { .. }
        | AIEvent::UpdateTick { .. }
        | AIEvent::AssistanceRequested { .. } => None,
    }
}

/// Invoke a Lua callback on a creature AI script.
pub fn invoke_callback(
    lua_ai: &super::scripts::LuaCreatureAI,
    lua: &mlua::Lua,
    snapshot: &LuaCreatureSnapshot,
    callback: &LuaCallback,
) -> Vec<LuaAction> {
    match callback {
        LuaCallback::OnEnterCombat => lua_ai.on_enter_combat(lua, snapshot),
        LuaCallback::OnDeath { killer_guid } => lua_ai.on_death(lua, snapshot, *killer_guid),
        LuaCallback::OnKill { victim_guid } => lua_ai.on_kill(lua, snapshot, *victim_guid),
        LuaCallback::OnSpellHit { spell_id, caster_guid } => {
            lua_ai.on_spell_hit(lua, snapshot, *spell_id, *caster_guid)
        }
        LuaCallback::OnDamageTaken { attacker_guid, damage, spell_id } => {
            lua_ai.on_damage_taken(lua, snapshot, *attacker_guid, *damage, *spell_id)
        }
        LuaCallback::OnEvade => lua_ai.on_evade(lua, snapshot),
        LuaCallback::OnReset => lua_ai.on_reset(lua, snapshot),
        LuaCallback::OnSpawn => lua_ai.on_spawn(lua, snapshot),
        LuaCallback::JustRespawned => lua_ai.on_just_respawned(lua, snapshot),
        LuaCallback::JustReachedHome => lua_ai.on_just_reached_home(lua, snapshot),
        LuaCallback::MovementInform { movement_type, point_id } => {
            lua_ai.on_movement_inform(lua, snapshot, *movement_type, *point_id)
        }
        LuaCallback::JustSummoned { summoned_guid, entry } => {
            lua_ai.on_just_summoned(lua, snapshot, *summoned_guid, *entry)
        }
        LuaCallback::SummonedCreatureJustDied { summoned_guid } => {
            lua_ai.on_summoned_creature_just_died(lua, snapshot, *summoned_guid)
        }
        LuaCallback::SummonedCreatureDespawn { summoned_guid } => {
            lua_ai.on_summoned_creature_despawn(lua, snapshot, *summoned_guid)
        }
        LuaCallback::MoveInLineOfSight { unit_guid, is_hostile } => {
            lua_ai.on_move_in_line_of_sight(lua, snapshot, *unit_guid, *is_hostile)
        }
        LuaCallback::HealedBy { healer_guid, amount, spell_id } => {
            lua_ai.on_healed_by(lua, snapshot, *healer_guid, *amount, *spell_id)
        }
        LuaCallback::SpellHitTarget { target_guid, spell_id } => {
            lua_ai.on_spell_hit_target(lua, snapshot, *target_guid, *spell_id)
        }
    }
}

/// Convert Lua actions to AI actions, applying state-only actions to CreatureScriptState.
pub fn lua_actions_to_ai_actions(
    actions: Vec<LuaAction>,
    state: &mut CreatureScriptState,
) -> Vec<AIAction> {
    let mut ai_actions = Vec::new();

    for action in actions {
        match action {
            // ==================== State-only actions (mutate state, no AIAction) ====================
            LuaAction::SetTimer { timer_id, duration_ms } => {
                state.set_timer(timer_id, duration_ms);
            }
            LuaAction::SetPhase { phase } => {
                state.set_phase(phase);
            }
            LuaAction::SetCustomData { key, value } => {
                state.set_custom_data(key, value);
            }

            // ==================== Direct AIAction mappings ====================
            LuaAction::MoveTo { x, y, z, run } => {
                ai_actions.push(AIAction::MoveTo {
                    position: Position { x, y, z, o: 0.0 },
                    movement_type: if run { MovementType::Run } else { MovementType::Walk },
                });
            }
            LuaAction::MoveToTarget { target, min_distance } => {
                ai_actions.push(AIAction::MoveToTarget {
                    target_guid: target,
                    min_distance,
                });
            }
            LuaAction::ReturnToSpawn => {
                ai_actions.push(AIAction::ReturnToSpawn);
            }
            LuaAction::StopMovement => {
                ai_actions.push(AIAction::StopMovement);
            }
            LuaAction::FleeFrom { target, distance, duration_ms } => {
                ai_actions.push(AIAction::FleeFrom {
                    flee_from_guid: target,
                    distance,
                    duration_ms,
                });
            }
            LuaAction::FaceTarget { target } => {
                ai_actions.push(AIAction::FaceTarget { target_guid: target });
            }
            LuaAction::RandomMovement { radius } => {
                ai_actions.push(AIAction::RandomMovement { wander_distance: radius });
            }
            LuaAction::CastSpell { spell_id, target, triggered } => {
                let target_guid = match target {
                    super::actions::SpellTarget::Self_ => None,
                    super::actions::SpellTarget::CurrentTarget => None, // Executor uses current target
                    super::actions::SpellTarget::Guid(guid) => Some(guid),
                    // For random/lowest targets, executor needs to resolve -- use None for now
                    _ => None,
                };
                ai_actions.push(AIAction::CastSpell {
                    spell_id,
                    target_guid,
                    target_position: None,
                    triggered,
                });
            }
            LuaAction::MeleeAttack { target } => {
                ai_actions.push(AIAction::MeleeAttack { target_guid: target });
            }
            LuaAction::SetAttackTarget { target } => {
                if let Some(guid) = target {
                    ai_actions.push(AIAction::SetAttackTarget { target_guid: guid });
                } else {
                    ai_actions.push(AIAction::ClearAttackTarget);
                }
            }
            LuaAction::EnterCombat { target } => {
                ai_actions.push(AIAction::EnterCombat { target_guid: target });
            }
            LuaAction::EnterEvadeMode => {
                ai_actions.push(AIAction::EnterEvadeMode);
            }
            LuaAction::LeaveCombat => {
                ai_actions.push(AIAction::LeaveCombat);
            }
            LuaAction::AddThreat { target, amount } => {
                ai_actions.push(AIAction::AddThreat {
                    target_guid: target,
                    amount,
                });
            }
            LuaAction::ModifyThreatPercent { target, percent } => {
                ai_actions.push(AIAction::ModifyThreat {
                    target_guid: target,
                    amount: percent,
                    is_percent: true,
                });
            }
            LuaAction::ClearThreatList => {
                ai_actions.push(AIAction::ClearThreatList);
            }
            LuaAction::ResetThreat => {
                ai_actions.push(AIAction::ClearThreatList);
            }
            LuaAction::SetCombatMovement { enabled } => {
                ai_actions.push(AIAction::SetCombatMovement { enabled });
            }
            LuaAction::SetMeleeAttack { enabled } => {
                ai_actions.push(AIAction::SetMeleeAttack { enabled });
            }
            LuaAction::SetReactState { state: react } => {
                let react_state = match react {
                    super::actions::ReactState::Passive => {
                        crate::world::game::creature::ai::ReactState::Passive
                    }
                    super::actions::ReactState::Defensive => {
                        crate::world::game::creature::ai::ReactState::Defensive
                    }
                    super::actions::ReactState::Aggressive => {
                        crate::world::game::creature::ai::ReactState::Aggressive
                    }
                };
                ai_actions.push(AIAction::SetReactState { react_state });
            }
            LuaAction::Say { text } => {
                ai_actions.push(AIAction::Say { text });
            }
            LuaAction::Yell { text } => {
                ai_actions.push(AIAction::Yell { text });
            }
            LuaAction::Emote { emote_id } => {
                ai_actions.push(AIAction::PlayEmote { emote_id });
            }
            LuaAction::TextEmote { text } => {
                ai_actions.push(AIAction::TextEmote { text });
            }
            LuaAction::PlaySound { sound_id, zone_wide } => {
                ai_actions.push(AIAction::PlaySound { sound_id, zone_wide });
            }

            // ==================== Creature State ====================
            LuaAction::InterruptSpell => {
                ai_actions.push(AIAction::InterruptSpell);
            }
            LuaAction::KillSelf => {
                ai_actions.push(AIAction::KillSelf);
            }
            LuaAction::SetFaction { faction_id } => {
                ai_actions.push(AIAction::SetFaction { faction_id });
            }
            LuaAction::SetImmune { physical, spell } => {
                ai_actions.push(AIAction::SetImmune { physical, spell });
            }
            LuaAction::SetRoot { rooted } => {
                ai_actions.push(AIAction::SetRoot { rooted });
            }
            LuaAction::SetHealthPercent { percent } => {
                ai_actions.push(AIAction::SetHealthPercent { percent });
            }
            LuaAction::Morph { display_id } => {
                ai_actions.push(AIAction::Morph { display_id });
            }
            LuaAction::Demorph => {
                ai_actions.push(AIAction::Demorph);
            }

            // ==================== Spawning ====================
            LuaAction::SpawnCreature { entry, x, y, z, o, summon_type, duration_ms } => {
                let st = match summon_type {
                    super::actions::SummonType::TimedDespawn => 0,
                    super::actions::SummonType::TimedDespawnOutOfCombat => 1,
                    super::actions::SummonType::CorpseDespawn => 2,
                    super::actions::SummonType::CorpseTimedDespawn => 3,
                    super::actions::SummonType::DeadDespawn => 4,
                    super::actions::SummonType::ManualDespawn => 5,
                };
                ai_actions.push(AIAction::SpawnCreature {
                    entry,
                    position: Position { x, y, z, o },
                    summon_type: st,
                    duration_ms,
                });
            }
            LuaAction::DespawnCreature { guid } => {
                ai_actions.push(AIAction::DespawnCreature { guid });
            }
            LuaAction::DespawnCreaturesByEntry { entry } => {
                ai_actions.push(AIAction::DespawnCreaturesByEntry { entry });
            }
            LuaAction::SpawnGameObject { entry, x, y, z, o, duration_secs } => {
                ai_actions.push(AIAction::SpawnGameObject {
                    entry,
                    position: Position { x, y, z, o },
                    duration_secs,
                });
            }
            LuaAction::RespawnGameObject { guid, duration_secs } => {
                ai_actions.push(AIAction::RespawnGameObject { guid, duration_secs });
            }

            // ==================== Instance ====================
            LuaAction::SetInstanceData { data_id, value } => {
                ai_actions.push(AIAction::SetInstanceData { data_id, value });
            }
            LuaAction::SetInstanceGuid { data_id, guid } => {
                ai_actions.push(AIAction::SetInstanceGuid { data_id, guid });
            }
            LuaAction::OpenDoor { guid } => {
                ai_actions.push(AIAction::OpenDoor { guid });
            }
            LuaAction::OpenDoorByData { data_id } => {
                ai_actions.push(AIAction::OpenDoorByData { data_id });
            }
            LuaAction::CloseDoor { guid } => {
                ai_actions.push(AIAction::CloseDoor { guid });
            }
            LuaAction::CloseDoorByData { data_id } => {
                ai_actions.push(AIAction::CloseDoorByData { data_id });
            }

            // ==================== Phase 5: New Actions ====================
            LuaAction::RemoveAura { spell_id } => {
                ai_actions.push(AIAction::RemoveAura { spell_id });
            }
            LuaAction::SetUnitFlag { flag } => {
                ai_actions.push(AIAction::SetUnitFlag { flag });
            }
            LuaAction::RemoveUnitFlag { flag } => {
                ai_actions.push(AIAction::RemoveUnitFlag { flag });
            }
            LuaAction::SetCombatWithZone => {
                ai_actions.push(AIAction::SetCombatWithZone);
            }
            LuaAction::SetStandState { state } => {
                ai_actions.push(AIAction::SetStandState { state });
            }
            LuaAction::SetDynFlag { flag } => {
                ai_actions.push(AIAction::SetDynFlag { flag });
            }
            LuaAction::RemoveDynFlag { flag } => {
                ai_actions.push(AIAction::RemoveDynFlag { flag });
            }
            LuaAction::ScriptText { text_id } => {
                ai_actions.push(AIAction::ScriptText { text_id });
            }

            // ==================== Phase 6: Path Movement ====================
            LuaAction::MoveAlongPath { waypoints, run, repeating } => {
                let positions: Vec<Position> = waypoints.iter().map(|&(x, y, z)| Position { x, y, z, o: 0.0 }).collect();
                ai_actions.push(AIAction::MoveAlongPath {
                    waypoints: positions,
                    movement_type: if run { MovementType::Run } else { MovementType::Walk },
                    repeating,
                });
            }

            // ==================== Not yet implemented -- log and skip ====================
            LuaAction::ZoneText { .. }
            | LuaAction::ZoneYell { .. }
            | LuaAction::SendWorldState { .. }
            | LuaAction::PlaySoundToZone { .. }
            | LuaAction::TeleportPlayer { .. }
            | LuaAction::GiveItem { .. }
            | LuaAction::TakeItem { .. }
            | LuaAction::GiveGold { .. }
            | LuaAction::TakeGold { .. }
            | LuaAction::AddReputation { .. }
            | LuaAction::CompleteQuest { .. }
            | LuaAction::GossipMenu { .. }
            | LuaAction::GossipOption { .. }
            | LuaAction::GossipQuest { .. }
            | LuaAction::GossipSend
            | LuaAction::GossipClose
            | LuaAction::SendVendor
            | LuaAction::SendTrainer
            | LuaAction::SendBanker
            | LuaAction::SendAuctioneer
            | LuaAction::SendInnkeeper
            | LuaAction::SendTaxi
            | LuaAction::KillCreditNearestCreature { .. }
            | LuaAction::SpawnCreatureAtPlayer { .. }
            | LuaAction::CastSpellOnNearestCreature { .. } => {
                tracing::debug!("Lua action not yet implemented: {:?}", action);
            }
        }
    }

    ai_actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player_guid(id: u32) -> ObjectGuid {
        use crate::shared::protocol::HighGuid;
        ObjectGuid::new_without_entry(HighGuid::Player, id)
    }

    #[test]
    fn test_combat_started_maps_to_on_enter_combat() {
        let event = AIEvent::CombatStarted { initial_aggressor: player_guid(1) };
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::OnEnterCombat)));
    }

    #[test]
    fn test_died_maps_to_on_death() {
        let event = AIEvent::Died { killer_guid: Some(player_guid(1)) };
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::OnDeath { .. })));
    }

    #[test]
    fn test_damage_taken_maps_to_on_damage_taken() {
        let event = AIEvent::DamageTaken {
            attacker_guid: player_guid(1),
            damage: 100,
            spell_id: None,
            school: 0,
        };
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::OnDamageTaken { .. })));
    }

    #[test]
    fn test_unit_in_range_maps_to_none() {
        let event = AIEvent::UnitInRange {
            unit_guid: player_guid(1),
            distance: 10.0,
            is_hostile: true,
            is_player: true,
        };
        let callback = map_ai_event_to_callback(&event);
        assert!(callback.is_none(), "UnitInRange should not have a Lua callback");
    }

    #[test]
    fn test_combat_ended_evade_maps_to_on_evade() {
        let event = AIEvent::CombatEnded { reason: CombatEndReason::Evaded };
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::OnEvade)));
    }

    #[test]
    fn test_combat_ended_targets_out_of_range_maps_to_on_evade() {
        let event = AIEvent::CombatEnded { reason: CombatEndReason::TargetsOutOfRange };
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::OnEvade)));
    }

    #[test]
    fn test_spawned_maps_to_on_spawn() {
        let event = AIEvent::Spawned;
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::OnSpawn)));
    }

    #[test]
    fn test_respawned_maps_to_just_respawned() {
        let event = AIEvent::Respawned;
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::JustRespawned)));
    }

    #[test]
    fn test_reached_home_maps_to_just_reached_home() {
        let event = AIEvent::ReachedHome;
        let callback = map_ai_event_to_callback(&event);
        assert!(matches!(callback, Some(LuaCallback::JustReachedHome)));
    }

    #[test]
    fn test_update_tick_maps_to_none() {
        let event = AIEvent::UpdateTick { diff_ms: 50 };
        let callback = map_ai_event_to_callback(&event);
        assert!(callback.is_none());
    }
}
