//! Temporary creatures summoned by scripts and AI, and their despawn rules.
//!
//! A summon is a transient creature with no database spawn: it never respawns.
//! It despawns on its own schedule — after a timer, after a timer spent out of
//! combat, when it dies, or when its corpse decays — and its summoner hears
//! about it summoning, dying and despawning.

use super::ai::{queue_event, AIEvent};
use super::death::DeathState;
use super::spawn::CreatureSpawnData;
use crate::World;
use dashmap::DashMap;
use oxcore_shared::messages::inventory::SmsgDestroyObject;
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::sync::OnceLock;

/// When a summon goes away. Every summon also goes once its corpse has decayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummonDespawn {
    /// After the timer, alive or dead.
    Timed,
    /// After the timer has run out while out of combat; combat resets it.
    TimedOutOfCombat,
    /// As soon as it dies.
    Corpse,
    /// The timer starts when it dies.
    CorpseTimed,
    /// When its corpse decays.
    Dead,
    /// After the timer (any combat state), or when its corpse decays.
    TimedOrDead,
    /// After the timer (any combat state), or as soon as it dies.
    TimedOrCorpse,
    /// After the timer out of combat, or when its corpse decays.
    OutOfCombatTimedOrDead,
    /// After the timer out of combat, or as soon as it dies.
    OutOfCombatTimedOrCorpse,
    /// Dies when the timer runs out, then goes when its corpse decays.
    TimedDeath,
    /// Only when a script despawns it.
    Manual,
}

impl SummonDespawn {
    /// The summon type codes the Lua bridge emits.
    pub fn from_script_code(code: u8) -> Self {
        match code {
            0 => SummonDespawn::Timed,
            1 => SummonDespawn::TimedOutOfCombat,
            2 => SummonDespawn::Corpse,
            3 => SummonDespawn::CorpseTimed,
            4 => SummonDespawn::Dead,
            5 => SummonDespawn::Manual,
            _ => SummonDespawn::TimedOrDead,
        }
    }

    /// The despawn types of database scripts.
    pub fn from_temp_summon_type(code: u32) -> Self {
        match code {
            1 => SummonDespawn::OutOfCombatTimedOrDead,
            2 => SummonDespawn::OutOfCombatTimedOrCorpse,
            3 => SummonDespawn::Timed,
            4 => SummonDespawn::TimedOutOfCombat,
            5 => SummonDespawn::Corpse,
            6 => SummonDespawn::CorpseTimed,
            7 => SummonDespawn::Dead,
            8 => SummonDespawn::Manual,
            9 => SummonDespawn::TimedOrDead,
            10 => SummonDespawn::TimedOrCorpse,
            11 => SummonDespawn::TimedDeath,
            _ => SummonDespawn::Dead,
        }
    }

    /// Summons whose dead count toward a "unique" limit (they come back as corpses).
    pub fn keeps_corpse(self) -> bool {
        matches!(
            self,
            SummonDespawn::Dead
                | SummonDespawn::TimedOrDead
                | SummonDespawn::OutOfCombatTimedOrDead
                | SummonDespawn::TimedDeath
                | SummonDespawn::CorpseTimed
        )
    }
}

#[derive(Debug, Clone)]
struct SummonState {
    summoner: Option<ObjectGuid>,
    despawn: SummonDespawn,
    duration_ms: u32,
    remaining_ms: u32,
    death_reported: bool,
}

fn summons() -> &'static DashMap<ObjectGuid, SummonState> {
    static SUMMONS: OnceLock<DashMap<ObjectGuid, SummonState>> = OnceLock::new();
    SUMMONS.get_or_init(DashMap::new)
}

/// The creature that summoned `guid`, if it is a summon.
pub fn summoner_of(guid: ObjectGuid) -> Option<ObjectGuid> {
    summons().get(&guid).and_then(|s| s.summoner)
}

/// Whether the creature is a temporary summon.
pub fn is_summon(guid: ObjectGuid) -> bool {
    summons().contains_key(&guid)
}

/// Whether `summoner` has a living summon of creature `entry`.
pub fn has_summon_with_entry(summoner: ObjectGuid, entry: u32) -> bool {
    summons()
        .iter()
        .any(|s| s.summoner == Some(summoner) && s.key().entry() == entry && !s.death_reported)
}

/// Summon a temporary creature. Returns its GUID.
#[allow(clippy::too_many_arguments)]
pub fn summon_creature(
    world: &World,
    summoner: Option<ObjectGuid>,
    entry: u32,
    map_id: u32,
    instance_id: u32,
    position: Position,
    despawn: SummonDespawn,
    duration_ms: u32,
) -> Option<ObjectGuid> {
    let spawn = CreatureSpawnData::new(0, entry, map_id, position, 0);
    let map = world
        .managers
        .map_mgr
        .get_or_create_map(map_id, instance_id);
    let Some(guid) = world.managers.creature_mgr.spawn_into_map(
        &spawn,
        &map,
        Some(&world.managers.waypoint_mgr),
    ) else {
        tracing::debug!("Summon of creature entry {} failed (no template?)", entry);
        return None;
    };
    summons().insert(
        guid,
        SummonState {
            summoner,
            despawn,
            duration_ms,
            remaining_ms: duration_ms,
            death_reported: false,
        },
    );
    if let Some(summoner) = summoner.filter(|s| s.is_creature()) {
        queue_event(
            world,
            summoner,
            AIEvent::SummonedCreature {
                summoned_guid: guid,
                entry,
            },
        );
    }
    queue_event(world, guid, AIEvent::Spawned);
    Some(guid)
}

/// Remove a creature from the world: off its map, out of every client's view,
/// out of the creature registry. Its summoner, if any, is told.
pub fn despawn_creature(world: &World, guid: ObjectGuid) {
    let Some((position, map_id, instance_id)) = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| (c.position, c.map_id, c.instance_id))
    else {
        summons().remove(&guid);
        return;
    };

    if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
        map.remove_creature(guid, position);
        map.remove_from_active(guid);
        let players = map.get_players_in_range(position, map.visibility_distance());
        world
            .managers
            .broadcast_mgr
            .broadcast_msg_to_players(&players, &SmsgDestroyObject { guid });
    }
    world.systems.loot.remove_loot(guid);
    world.managers.creature_mgr.remove_creature(guid);
    super::ai::forget_lua_state(guid);
    super::ai::event_ai::forget(guid);

    if let Some((_, state)) = summons().remove(&guid) {
        if let Some(summoner) = state.summoner.filter(|s| s.is_creature()) {
            queue_event(
                world,
                summoner,
                AIEvent::SummonedCreatureDespawned {
                    summoned_guid: guid,
                },
            );
        }
    }
}

/// Run the despawn rules of every summon.
pub fn update_summons(world: &World, diff_ms: u32) {
    let guids: Vec<ObjectGuid> = summons().iter().map(|e| *e.key()).collect();
    for guid in guids {
        let Some((alive, in_combat, corpse_gone)) = world
            .managers
            .creature_mgr
            .with_creature(guid, |c| {
                (
                    c.is_alive(),
                    c.combat.in_combat,
                    c.death_state == DeathState::Dead,
                )
            })
        else {
            summons().remove(&guid);
            continue;
        };

        let (despawn_now, report_death, summoner) = {
            let Some(mut state) = summons().get_mut(&guid) else {
                continue;
            };
            let report_death = !alive && !state.death_reported;
            if report_death {
                state.death_reported = true;
                // The corpse timer starts at death.
                if state.despawn == SummonDespawn::CorpseTimed {
                    state.remaining_ms = state.duration_ms;
                }
            }
            let timer_expired = |state: &mut SummonState| {
                if state.remaining_ms <= diff_ms {
                    true
                } else {
                    state.remaining_ms -= diff_ms;
                    false
                }
            };
            let mut out_of_combat_timer = |state: &mut SummonState| {
                if in_combat {
                    state.remaining_ms = state.duration_ms;
                    false
                } else {
                    timer_expired(state)
                }
            };
            let mut kill_now = false;
            let despawn_now = corpse_gone
                || match state.despawn {
                    SummonDespawn::Timed => timer_expired(&mut state),
                    SummonDespawn::TimedOutOfCombat => out_of_combat_timer(&mut state),
                    SummonDespawn::Corpse => !alive,
                    SummonDespawn::CorpseTimed => !alive && timer_expired(&mut state),
                    SummonDespawn::Dead | SummonDespawn::Manual => false,
                    SummonDespawn::TimedOrDead => alive && timer_expired(&mut state),
                    SummonDespawn::TimedOrCorpse => !alive || timer_expired(&mut state),
                    SummonDespawn::OutOfCombatTimedOrDead => alive && out_of_combat_timer(&mut state),
                    SummonDespawn::OutOfCombatTimedOrCorpse => {
                        !alive || out_of_combat_timer(&mut state)
                    }
                    SummonDespawn::TimedDeath => {
                        kill_now = alive && timer_expired(&mut state);
                        false
                    }
                };
            if kill_now {
                drop(state);
                super::forced::force_kill(world, guid, None);
                continue;
            }
            (despawn_now, report_death, state.summoner)
        };

        if report_death {
            if let Some(summoner) = summoner.filter(|s| s.is_creature()) {
                queue_event(
                    world,
                    summoner,
                    AIEvent::SummonedCreatureDied {
                        summoned_guid: guid,
                    },
                );
            }
        }
        if despawn_now {
            despawn_creature(world, guid);
        }
    }
}

/// Temporary game objects: GUID -> (map, copy, milliseconds left).
fn temporary_gameobjects() -> &'static DashMap<ObjectGuid, (u32, u32, u32)> {
    static OBJECTS: OnceLock<DashMap<ObjectGuid, (u32, u32, u32)>> = OnceLock::new();
    OBJECTS.get_or_init(DashMap::new)
}

/// Show a game object that is in the world to the players around it.
pub fn show_gameobject(world: &World, guid: ObjectGuid, map_id: u32, instance_id: u32, position: Position) {
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return;
    };
    map.add_gameobject(guid, position);
    world
        .managers
        .gameobject_mgr
        .add_collision_model(guid, &world.managers.vmap_mgr);
    for viewer in map.get_players_in_range(position, map.visibility_distance()) {
        if let Some(create) = world
            .managers
            .gameobject_mgr
            .build_create_msg(guid, viewer, world)
        {
            world.managers.broadcast_mgr.send_msg_to_player(viewer, create);
        }
    }
}

/// Take a game object out of the world and out of every client's view.
pub fn hide_gameobject(world: &World, guid: ObjectGuid) {
    let Some((map_id, instance_id, position)) = world
        .managers
        .gameobject_mgr
        .with_gameobject_mut(guid, |go| {
            go.in_world = false;
            (go.map_id, go.instance_id, go.position)
        })
    else {
        return;
    };
    world
        .managers
        .gameobject_mgr
        .remove_collision_model(guid, &world.managers.vmap_mgr);
    if let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) {
        map.remove_gameobject(guid, position);
        let players = map.get_players_in_range(position, map.visibility_distance());
        world
            .managers
            .broadcast_mgr
            .broadcast_msg_to_players(&players, &SmsgDestroyObject { guid });
    }
}

/// Remove a game object for good: out of view, out of the registry.
pub fn delete_gameobject(world: &World, guid: ObjectGuid) {
    hide_gameobject(world, guid);
    temporary_gameobjects().remove(&guid);
    world.systems.pool.on_object_despawned(guid);
    world.managers.gameobject_mgr.remove_gameobject(guid);
}

/// Summon a temporary game object that disappears after `duration_secs`
/// (0 keeps it until something removes it).
pub fn summon_gameobject(
    world: &World,
    entry: u32,
    map_id: u32,
    instance_id: u32,
    position: Position,
    duration_secs: u32,
) -> Option<ObjectGuid> {
    let spawn = crate::game::gameobject::GameObjectSpawnData {
        spawn_id: 0,
        entry,
        map_id,
        position,
        rotation0: 0.0,
        rotation1: 0.0,
        rotation2: (position.o / 2.0).sin(),
        rotation3: (position.o / 2.0).cos(),
        spawntimesecs: 0,
        animprogress: 100,
        state: 1,
    };
    let guid = world
        .managers
        .gameobject_mgr
        .spawn_gameobject(&spawn, instance_id)?;
    show_gameobject(world, guid, map_id, instance_id, position);
    if duration_secs > 0 {
        temporary_gameobjects().insert(guid, (map_id, instance_id, duration_secs * 1000));
    }
    Some(guid)
}

/// Bring back a despawned game object for `duration_secs` (0 = until it is used).
pub fn respawn_gameobject(world: &World, guid: ObjectGuid, duration_secs: u32) {
    let Some(placed) = world
        .managers
        .gameobject_mgr
        .with_gameobject_mut(guid, |go| {
            if go.in_world {
                return None;
            }
            go.in_world = true;
            go.respawn_time = 0;
            Some((go.map_id, go.instance_id, go.position))
        })
        .flatten()
    else {
        return;
    };
    let (map_id, instance_id, position) = placed;
    show_gameobject(world, guid, map_id, instance_id, position);
    if duration_secs > 0 {
        temporary_gameobjects().insert(guid, (map_id, instance_id, duration_secs * 1000));
    }
}

/// Remove temporary game objects whose time is up.
pub fn update_temporary_gameobjects(world: &World, diff_ms: u32) {
    let mut expired = Vec::new();
    for mut entry in temporary_gameobjects().iter_mut() {
        let left = &mut entry.value_mut().2;
        if *left <= diff_ms {
            expired.push(*entry.key());
        } else {
            *left -= diff_ms;
        }
    }
    for guid in expired {
        temporary_gameobjects().remove(&guid);
        let transient = world
            .managers
            .gameobject_mgr
            .with_gameobject(guid, |go| go.spawn_id == 0)
            .unwrap_or(false);
        hide_gameobject(world, guid);
        if transient {
            world.managers.gameobject_mgr.remove_gameobject(guid);
        }
    }
}
