//! Objective scripts: battleground and open-world PvP logic in Lua.
//!
//! A battleground copy runs its map's instance script; an open-world zone runs
//! its zone script. Both get the same events (area triggers, objects used,
//! deaths, auras lost, the match clock) and the same actions (world states,
//! spawn events, announcements, team rewards), so an objective behaves the
//! same wherever it is.

use mlua::Value;
use oxcore_shared::game::chat::{ChatMsg, Team};
use oxcore_shared::protocol::{ObjectGuid, Position};

use super::scope::{self, Scope};
use crate::core::lua::objective::ScopeScript;
use crate::core::lua::snapshot::{LuaGuid, MatchView, ScopePlayer, ScopeView};
use crate::core::lua::{AnnounceChat, CreatureScriptState, LuaAction, SummonType};
use crate::game::creature::summon::SummonDespawn;
use crate::World;

/// The script owning a scope, if it has one.
pub fn scope_script(world: &World, scope: Scope) -> Option<ScopeScript> {
    let lua_mgr = &world.managers.lua_mgr;
    match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => (instance_id != 0 && lua_mgr.has_instance_script(map_id)).then_some(ScopeScript::Instance(map_id)),
        Scope::Zone { zone_id } => lua_mgr.has_zone_script(zone_id).then_some(ScopeScript::Zone(zone_id)),
    }
}

/// The scopes a player is in: their copy (if instanced) and their zone.
pub fn scopes_of(world: &World, player: ObjectGuid) -> Vec<Scope> {
    let Some((map_id, instance_id, zone_id)) = world
        .managers
        .player_mgr
        .with_player(player, |p| (p.map_id, p.instance_id, p.zone_id))
    else {
        return Vec::new();
    };
    let mut scopes = Vec::with_capacity(2);
    if instance_id != 0 {
        scopes.push(Scope::Copy {
            map_id,
            instance_id,
        });
    }
    if zone_id != 0 {
        scopes.push(Scope::Zone { zone_id });
    }
    scopes
}

/// A number declared on a script table (e.g. `ScoreColumns = 2`).
pub fn declared_number(world: &World, script: ScopeScript, name: &str) -> Option<u32> {
    world.managers.lua_mgr.with_lua(|lua| script.field::<u32>(lua, name))
}

fn watched_auras(world: &World, script: ScopeScript) -> Vec<u32> {
    world
        .managers
        .lua_mgr
        .with_lua(|lua| script.number_list(lua, "WatchedAuras"))
}

/// What the scope's script sees beyond its own data.
pub fn scope_view(world: &World, scope: Scope) -> ScopeView {
    let watched = scope_script(world, scope)
        .map(|script| watched_auras(world, script))
        .unwrap_or_default();
    let bg_match = match scope {
        Scope::Copy { instance_id, .. } => world.managers.battleground_mgr.get_match(instance_id),
        Scope::Zone { .. } => None,
    };
    let players = scope::players_in_scope(world, scope)
        .into_iter()
        .filter_map(|guid| {
            let team = bg_match
                .as_ref()
                .and_then(|m| m.team_of(guid))
                .or_else(|| crate::game::battleground::system::player_team(world, guid))
                .map(|t| t as u32)
                .unwrap_or(0);
            world.managers.player_mgr.with_player(guid, |p| ScopePlayer {
                guid,
                name: p.name.clone(),
                team,
                x: p.movement.position.x,
                y: p.movement.position.y,
                z: p.movement.position.z,
                alive: p.is_alive(),
                level: p.level,
                class: p.class,
                race: p.race,
                area_id: p.zone_id,
                is_gm: p.gm_mode,
                watched_auras: watched
                    .iter()
                    .copied()
                    .filter(|&spell| p.auras.container.has_aura(spell))
                    .collect(),
            })
        })
        .collect();
    let (events, battleground) = match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => (
            world.managers.scope_states.active_events(map_id, instance_id),
            bg_match.map(|m| MatchView {
                bg_type: m.bg_type.as_u32(),
                status: m.status.as_str(),
                bracket: m.bracket,
                min_level: m.min_level,
                max_level: m.max_level,
                elapsed_ms: m.elapsed_since_open_ms(),
                weekend: is_bg_weekend(world, m.bg_type),
                winner: m.winner.map(|t| t as u32).unwrap_or(0),
                alliance_players: m.players_of(Team::Alliance),
                horde_players: m.players_of(Team::Horde),
            }),
        ),
        Scope::Zone { .. } => (Default::default(), None),
    };
    ScopeView {
        players,
        world_states: world.managers.scope_states.world_states(scope),
        events,
        battleground,
    }
}

/// "Call to Arms" holiday weekends: game events 18 (Alterac Valley),
/// 19 (Warsong Gulch) and 20 (Arathi Basin).
fn is_bg_weekend(world: &World, bg_type: oxcore_shared::game::battleground::BattleGroundTypeId) -> bool {
    use oxcore_shared::game::battleground::BattleGroundTypeId as T;
    let event = match bg_type {
        T::AlteracValley => 18,
        T::WarsongGulch => 19,
        T::ArathiBasin => 20,
        T::None => return false,
    };
    world.managers.game_event_mgr.is_active(event)
}

/// Build the scope's input table and call a callback with extra arguments.
fn call<A>(world: &World, scope: Scope, callback: &str, args: A) -> Option<crate::core::lua::objective::CallbackResult>
where
    A: mlua::IntoLuaMulti,
{
    let script = scope_script(world, scope)?;
    let has = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.has_callback(lua, callback));
    if !has {
        return None;
    }
    let result = match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => {
            let snapshot = crate::game::instance::scripts::snapshot(world, map_id, instance_id);
            world.managers.lua_mgr.with_lua(|lua| {
                let input = snapshot.to_lua_table(lua).ok()?;
                Some(script.call(lua, callback, input, args))
            })
        }
        Scope::Zone { zone_id } => {
            let snapshot = crate::core::lua::zone_executor::zone_snapshot(zone_id, world, 0);
            world.managers.lua_mgr.with_lua(|lua| {
                let input = snapshot.to_lua_table(lua).ok()?;
                Some(script.call(lua, callback, input, args))
            })
        }
    };
    result
}

/// Make sure a battleground copy's script has loaded before events reach it.
async fn ensure_loaded(world: &World, scope: Scope) {
    if let Scope::Copy {
        map_id,
        instance_id,
    } = scope
    {
        crate::game::instance::scripts::ensure_loaded(world, map_id, instance_id).await;
    }
}

/// A step of the match clock ("PREPARATION", "BEGIN", ...).
pub async fn match_event(world: &World, scope: Scope, event: &str) {
    ensure_loaded(world, scope).await;
    if let Some(result) = call(world, scope, "OnMatchEvent", event.to_string()) {
        apply(world, scope, None, None, result.actions).await;
    }
}

/// An area trigger in a scripted scope. Returns whether a script claimed it.
pub async fn on_area_trigger(world: &World, player: ObjectGuid, trigger_id: u32) -> bool {
    for scope in scopes_of(world, player) {
        ensure_loaded(world, scope).await;
        if let Some(result) = call(world, scope, "OnAreaTrigger", (LuaGuid(player), trigger_id)) {
            let handled = result.handled;
            apply(world, scope, None, Some(player), result.actions).await;
            if handled {
                return true;
            }
        }
    }
    false
}

/// A player used (clicked or opened) a gameobject in a scripted scope.
/// Returns whether a script claimed it.
pub async fn on_gameobject_use(world: &World, player: ObjectGuid, go: ObjectGuid) -> bool {
    let Some(entry) = world.managers.gameobject_mgr.with_gameobject(go, |g| g.entry) else {
        return false;
    };
    let event = crate::game::pvp::events::gameobject_event(world, go);
    for scope in scopes_of(world, player) {
        ensure_loaded(world, scope).await;
        let args = (
            LuaGuid(player),
            LuaGuid(go),
            entry,
            event.map(|e| e.0).unwrap_or(super::scope::EVENT_NONE),
            event.map(|e| e.1).unwrap_or(super::scope::EVENT_NONE),
        );
        if let Some(result) = call(world, scope, "OnGameObjectUse", args) {
            let handled = result.handled;
            apply(world, scope, None, Some(player), result.actions).await;
            if handled {
                return true;
            }
        }
    }
    false
}

/// A player died in a scripted scope (`killer` may be a creature or nothing).
pub async fn on_player_death(world: &World, victim: ObjectGuid, killer: Option<ObjectGuid>) {
    for scope in scopes_of(world, victim) {
        let killer_arg = killer.map(LuaGuid);
        if let Some(result) = call(world, scope, "OnPlayerDeath", (LuaGuid(victim), killer_arg)) {
            apply(world, scope, None, Some(victim), result.actions).await;
        }
    }
}

/// Fire `OnAuraRemoved` for watched auras players lost since the last look,
/// whatever removed them (expiry, a click, death, immunity).
pub async fn watch_auras(world: &World, scope: Scope) {
    let Some(script) = scope_script(world, scope) else {
        return;
    };
    let watched = watched_auras(world, script);
    if watched.is_empty() {
        return;
    }
    let present = scope::players_in_scope(world, scope);
    let mut lost: Vec<(ObjectGuid, u32)> = Vec::new();
    for &player in &present {
        let now: Vec<u32> = world
            .managers
            .player_mgr
            .with_player(player, |p| {
                watched
                    .iter()
                    .copied()
                    .filter(|&spell| p.auras.container.has_aura(spell))
                    .collect()
            })
            .unwrap_or_default();
        for spell in world.managers.scope_states.swap_watched(scope, player, now) {
            lost.push((player, spell));
        }
    }
    // Players who left the scope lose whatever they had.
    for player in world.managers.scope_states.watched_players(scope) {
        if !present.contains(&player) {
            for spell in world.managers.scope_states.forget_watched(scope, player) {
                lost.push((player, spell));
            }
        }
    }
    for (player, spell) in lost {
        if let Some(result) = call(world, scope, "OnAuraRemoved", (LuaGuid(player), spell)) {
            apply(world, scope, None, Some(player), result.actions).await;
        }
    }
}

/// The graveyard a script picks for a ghost: a WorldSafeLocs id, or a table
/// `{ x, y, z, o, map }`.
pub fn select_graveyard(world: &World, scope: Scope, player: ObjectGuid) -> Option<(u32, Position)> {
    let script = scope_script(world, scope)?;
    let map_id = match scope {
        Scope::Copy { map_id, .. } => map_id,
        Scope::Zone { .. } => world.managers.player_mgr.with_player(player, |p| p.map_id)?,
    };
    let snapshot_map = match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => Some(crate::game::instance::scripts::snapshot(world, map_id, instance_id)),
        Scope::Zone { .. } => None,
    };
    let zone_snapshot = match scope {
        Scope::Zone { zone_id } => Some(crate::core::lua::zone_executor::zone_snapshot(zone_id, world, 0)),
        Scope::Copy { .. } => None,
    };
    let answer = world.managers.lua_mgr.with_lua(|lua| {
        let input = match (&snapshot_map, &zone_snapshot) {
            (Some(s), _) => s.to_lua_table(lua).ok()?,
            (_, Some(s)) => s.to_lua_table(lua).ok()?,
            _ => return None,
        };
        match script.query(lua, "SelectGraveyard", input, LuaGuid(player))? {
            Value::Integer(id) => Some(Err(id as u32)),
            Value::Number(id) => Some(Err(id as u32)),
            Value::Table(t) => Some(Ok((
                t.get::<u32>("map").unwrap_or(map_id),
                Position::new(
                    t.get("x").ok()?,
                    t.get("y").ok()?,
                    t.get("z").ok()?,
                    t.get("o").unwrap_or(0.0),
                ),
            ))),
            _ => None,
        }
    })?;
    match answer {
        Ok(place) => Some(place),
        Err(safe_loc) => {
            let dbc = world.dbc.read();
            let loc = dbc.get_world_safe_locs(safe_loc)?;
            Some((loc.map_id, Position::new(loc.x, loc.y, loc.z, 0.0)))
        }
    }
}

fn team_from(value: u32) -> Option<Team> {
    match value {
        469 => Some(Team::Alliance),
        67 => Some(Team::Horde),
        _ => None,
    }
}

/// Players in a scope, optionally only one side.
fn players_of(world: &World, scope: Scope, team: Option<u32>) -> Vec<ObjectGuid> {
    let players = scope::players_in_scope(world, scope);
    let Some(team) = team.and_then(team_from) else {
        return players;
    };
    let bg_match = match scope {
        Scope::Copy { instance_id, .. } => world.managers.battleground_mgr.get_match(instance_id),
        Scope::Zone { .. } => None,
    };
    players
        .into_iter()
        .filter(|&g| {
            bg_match
                .as_ref()
                .and_then(|m| m.team_of(g))
                .or_else(|| crate::game::battleground::system::player_team(world, g))
                == Some(team)
        })
        .collect()
}

fn scope_place(world: &World, scope: Scope, trigger: Option<ObjectGuid>) -> Option<(u32, u32)> {
    match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => Some((map_id, instance_id)),
        Scope::Zone { .. } => {
            let anchor = trigger.or_else(|| scope::players_in_scope(world, scope).first().copied())?;
            world
                .managers
                .player_mgr
                .with_player(anchor, |p| (p.map_id, p.instance_id))
        }
    }
}

fn despawn_kind(summon: SummonType) -> SummonDespawn {
    match summon {
        SummonType::TimedDespawn => SummonDespawn::Timed,
        SummonType::TimedDespawnOutOfCombat => SummonDespawn::TimedOutOfCombat,
        SummonType::CorpseDespawn => SummonDespawn::Corpse,
        SummonType::CorpseTimedDespawn => SummonDespawn::CorpseTimed,
        SummonType::DeadDespawn => SummonDespawn::Dead,
        SummonType::ManualDespawn => SummonDespawn::Manual,
    }
}

fn set_scope_data(world: &World, scope: Scope, data_id: u32, value: u32) {
    match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => world
            .managers
            .lua_mgr
            .set_instance_data(map_id, instance_id, data_id, value),
        Scope::Zone { zone_id } => world.managers.lua_mgr.set_zone_data(zone_id, data_id, value),
    }
}

fn scope_guid(world: &World, scope: Scope, data_id: u32) -> Option<ObjectGuid> {
    match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => world
            .managers
            .lua_mgr
            .get_instance_state(map_id, instance_id)
            .get_guid(data_id),
        Scope::Zone { zone_id } => world.managers.lua_mgr.get_zone_state(zone_id).get_guid(data_id),
    }
}

/// Carry out an objective script's actions in its scope.
///
/// `actor` is the creature a callback was about (creature actions run through
/// it); `trigger` is the player who caused the event (player-addressed actions
/// without a target fall back to them).
pub async fn apply(
    world: &World,
    scope: Scope,
    actor: Option<ObjectGuid>,
    trigger: Option<ObjectGuid>,
    actions: Vec<LuaAction>,
) {
    if actions.is_empty() {
        return;
    }
    let mut creature_actions = Vec::new();
    let mut player_actions = Vec::new();
    for action in actions {
        match action {
            LuaAction::SetInstanceData { data_id, value } => set_scope_data(world, scope, data_id, value),
            LuaAction::SetInstanceGuid { data_id, guid } => match scope {
                Scope::Copy {
                    map_id,
                    instance_id,
                } => {
                    let mut state = world.managers.lua_mgr.get_instance_state(map_id, instance_id);
                    state.set_guid(data_id, guid);
                    world
                        .managers
                        .lua_mgr
                        .set_instance_state(map_id, instance_id, state);
                }
                Scope::Zone { zone_id } => world
                    .managers
                    .lua_mgr
                    .with_zone_state_mut(zone_id, |s| s.set_guid(data_id, guid)),
            },
            LuaAction::SetTimer {
                timer_id,
                duration_ms,
            } => match scope {
                Scope::Copy {
                    map_id,
                    instance_id,
                } => {
                    let mut state = world.managers.lua_mgr.get_instance_state(map_id, instance_id);
                    state.set_timer(timer_id, duration_ms);
                    world
                        .managers
                        .lua_mgr
                        .set_instance_state(map_id, instance_id, state);
                }
                Scope::Zone { zone_id } => world
                    .managers
                    .lua_mgr
                    .with_zone_state_mut(zone_id, |s| s.set_timer(timer_id, duration_ms)),
            },
            LuaAction::OpenDoor { guid } => crate::game::instance::scripts::set_door(world, guid, true),
            LuaAction::CloseDoor { guid } => crate::game::instance::scripts::set_door(world, guid, false),
            LuaAction::OpenDoorByData { data_id } => {
                if let Some(guid) = scope_guid(world, scope, data_id) {
                    crate::game::instance::scripts::set_door(world, guid, true);
                }
            }
            LuaAction::CloseDoorByData { data_id } => {
                if let Some(guid) = scope_guid(world, scope, data_id) {
                    crate::game::instance::scripts::set_door(world, guid, false);
                }
            }
            LuaAction::SetWorldState { state_id, value } | LuaAction::SendWorldState { state_id, value } => {
                scope::set_world_state(world, scope, state_id, value);
            }
            LuaAction::SpawnEvent {
                event1,
                event2,
                spawn,
            } => {
                if let Scope::Copy {
                    map_id,
                    instance_id,
                } = scope
                {
                    super::events::spawn_event(world, map_id, instance_id, event1, event2, spawn).await;
                }
            }
            LuaAction::DoorEvent { event1, event2, open } => {
                if let Scope::Copy {
                    map_id,
                    instance_id,
                } = scope
                {
                    super::events::door_event(world, map_id, instance_id, event1, event2, open);
                }
            }
            LuaAction::Announce {
                text,
                text_id,
                chat,
                team,
                player,
            } => announce(world, scope, text, text_id, chat, team, player.or(trigger)),
            LuaAction::ZoneText { text } => {
                announce(world, scope, Some(text), None, AnnounceChat::System, None, None)
            }
            LuaAction::ZoneYell { text } => {
                announce(world, scope, Some(text), None, AnnounceChat::BossEmote, None, None)
            }
            LuaAction::PlaySoundToScope { sound_id, team } => {
                let players = players_of(world, scope, team);
                world.managers.broadcast_mgr.broadcast_msg_to_players(
                    &players,
                    &oxcore_shared::messages::battleground::SmsgPlaySound { sound_id },
                );
            }
            LuaAction::PlaySoundToZone { sound_id } => {
                let players = players_of(world, scope, None);
                world.managers.broadcast_mgr.broadcast_msg_to_players(
                    &players,
                    &oxcore_shared::messages::battleground::SmsgPlaySound { sound_id },
                );
            }
            LuaAction::CastSpellOnTeam { spell_id, team } => {
                for player in players_of(world, scope, Some(team)) {
                    let _ = world
                        .systems
                        .spells
                        .cast_spell(player, spell_id, Some(player), true, world)
                        .await;
                }
            }
            LuaAction::PlayerCastSpell { player, spell_id } => {
                let _ = world
                    .systems
                    .spells
                    .cast_spell(player, spell_id, Some(player), true, world)
                    .await;
            }
            LuaAction::PlayerRemoveAura { player, spell_id } => {
                let _ = world.systems.auras.remove_spell_auras(player, spell_id, world).await;
            }
            LuaAction::RewardHonorToTeam { team, amount } => {
                for player in players_of(world, scope, Some(team)) {
                    reward_honor(world, player, amount);
                }
            }
            LuaAction::RewardHonor { player, amount } => reward_honor(world, player, amount),
            LuaAction::RewardReputationToTeam {
                team,
                faction_id,
                amount,
            } => {
                for player in players_of(world, scope, Some(team)) {
                    let _ = world
                        .systems
                        .reputation
                        .modify_reputation(player, faction_id, amount, world);
                }
            }
            LuaAction::UpdateScore {
                player,
                column,
                value,
            } => crate::game::battleground::system::add_score(world, player, column, value),
            LuaAction::EndMatch { winner } => {
                if let Scope::Copy { instance_id, .. } = scope {
                    crate::game::battleground::system::end_match(world, instance_id, team_from(winner)).await;
                }
            }
            LuaAction::SetFlagCarrier { team, player } => {
                if let (Scope::Copy { instance_id, .. }, Some(team)) = (scope, team_from(team)) {
                    crate::game::battleground::system::set_flag_carrier(world, instance_id, team, player);
                }
            }
            LuaAction::SetInCaptureArea { player, inside } => {
                world
                    .managers
                    .player_mgr
                    .with_player_mut(player, |p| p.pvp.in_capture_point = inside);
                if inside {
                    super::flag::update_pvp(world, player, true, false);
                }
            }
            LuaAction::DespawnGameObject { guid, entry } => {
                let mut targets: Vec<ObjectGuid> = guid.into_iter().collect();
                if let (Some(entry), Some((map_id, instance_id))) = (entry, scope_place(world, scope, trigger)) {
                    targets.extend(
                        world
                            .managers
                            .gameobject_mgr
                            .gameobjects_in_copy(map_id, instance_id)
                            .into_iter()
                            .filter(|&g| {
                                world
                                    .managers
                                    .gameobject_mgr
                                    .with_gameobject(g, |go| go.entry == entry)
                                    .unwrap_or(false)
                            }),
                    );
                }
                for guid in targets {
                    crate::game::creature::summon::delete_gameobject(world, guid);
                }
            }
            LuaAction::SpawnGameObject {
                entry,
                x,
                y,
                z,
                o,
                duration_secs,
            } if actor.is_none() => {
                if let Some((map_id, instance_id)) = scope_place(world, scope, trigger) {
                    crate::game::creature::summon::summon_gameobject(
                        world,
                        entry,
                        map_id,
                        instance_id,
                        Position::new(x, y, z, o),
                        duration_secs,
                    );
                }
            }
            LuaAction::SpawnCreature {
                entry,
                x,
                y,
                z,
                o,
                summon_type,
                duration_ms,
            } if actor.is_none() => {
                if let Some((map_id, instance_id)) = scope_place(world, scope, trigger) {
                    if let Some(guid) = crate::game::creature::summon::summon_creature(
                        world,
                        None,
                        entry,
                        map_id,
                        instance_id,
                        Position::new(x, y, z, o),
                        despawn_kind(summon_type),
                        duration_ms,
                    ) {
                        if let Some(create) = world.managers.creature_mgr.build_create_msg(guid, world) {
                            if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
                                for viewer in map.get_players_in_range(
                                    Position::new(x, y, z, o),
                                    map.visibility_distance(),
                                ) {
                                    world
                                        .managers
                                        .broadcast_mgr
                                        .send_msg_to_player(viewer, create.clone());
                                }
                            }
                        }
                    }
                }
            }
            LuaAction::DespawnCreature { guid } if actor.is_none() => {
                crate::game::creature::summon::despawn_creature(world, guid);
            }
            LuaAction::LeaveBattleground { player } => {
                crate::game::battleground::system::leave_battleground(world, player, true).await;
            }
            LuaAction::KillCredit { player, entry } => {
                world
                    .systems
                    .quest
                    .handle_kill_credit(player, entry, ObjectGuid::empty());
            }
            LuaAction::RepopTeam { team } => {
                for player in players_of(world, scope, team) {
                    if !world.managers.player_mgr.is_player_alive(player) {
                        let _ = world.systems.death.repop_at_graveyard(player, world);
                    }
                }
            }
            LuaAction::TeleportPlayer { .. }
            | LuaAction::GiveItem { .. }
            | LuaAction::TakeItem { .. }
            | LuaAction::GiveGold { .. }
            | LuaAction::TakeGold { .. }
            | LuaAction::AddReputation { .. }
            | LuaAction::CompleteQuest { .. }
                if actor.is_none() =>
            {
                player_actions.push(action)
            }
            other => creature_actions.push(other),
        }
    }

    if !player_actions.is_empty() {
        let player = trigger.unwrap_or_else(ObjectGuid::empty);
        if let Err(e) =
            crate::core::lua::execute_gossip_actions(player_actions, player, ObjectGuid::empty(), world).await
        {
            tracing::warn!("Objective script player actions failed: {}", e);
        }
    }
    if !creature_actions.is_empty() {
        match actor {
            Some(creature) => {
                let mut state = CreatureScriptState::default();
                let ai_actions = crate::core::lua::bridge::lua_actions_to_ai_actions(creature_actions, &mut state);
                crate::game::creature::ai::executor::execute_actions(world, creature, ai_actions);
            }
            None => tracing::debug!(
                "Objective script in {:?} returned {} actions with no creature to run them",
                scope,
                creature_actions.len()
            ),
        }
    }
}

/// Bonus honour: added to the player's honour and, in a match, the scoreboard.
fn reward_honor(world: &World, player: ObjectGuid, amount: u32) {
    if amount == 0 {
        return;
    }
    if crate::game::player::honor::add_honor(
        world,
        player,
        amount as f32,
        crate::game::player::honor::HonorType::Bonus,
        None,
    ) {
        crate::game::battleground::system::add_bonus_honor(world, player, amount);
    }
}

/// Send an announcement to a scope or one side of it.
fn announce(
    world: &World,
    scope: Scope,
    text: Option<String>,
    text_id: Option<u32>,
    chat: AnnounceChat,
    team: Option<u32>,
    player: Option<ObjectGuid>,
) {
    let line = match (text, text_id) {
        (Some(text), _) => text,
        (None, Some(id)) => match world.systems.gossip_manager.get_broadcast_text(id) {
            Some(t) if !t.male_text.is_empty() => t.male_text,
            Some(t) => t.female_text,
            None => return,
        },
        (None, None) => return,
    };
    let name = player
        .and_then(|p| world.managers.player_mgr.get_player_name(p))
        .unwrap_or_default();
    let line = line.replace("$n", &name).replace("$N", &name);
    let msgtype = match chat {
        AnnounceChat::Neutral => ChatMsg::BgSystemNeutral,
        AnnounceChat::Alliance => ChatMsg::BgSystemAlliance,
        AnnounceChat::Horde => ChatMsg::BgSystemHorde,
        AnnounceChat::System => ChatMsg::System,
        AnnounceChat::BossEmote => ChatMsg::RaidBossEmote,
    };
    let players = players_of(world, scope, team);
    let message = crate::game::battleground::system::system_chat(&line, msgtype);
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_to_players(&players, &message);
}
