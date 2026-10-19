//! Spawn events of battleground copies.
//!
//! A battleground's creatures and objects are grouped into events
//! (`event1`, `event2`): `event1` is a slot such as "the Stables banner" or
//! "the doors", and at most one `event2` per slot is active in a copy. A
//! spawn tied to events exists in a copy only while all of its events are
//! active. Scripts switch events; the copy spawns and removes what changes.

use std::collections::HashMap;

use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::RwLock;
use sqlx::PgPool;

use super::scope::EVENT_NONE;
use crate::game::game_event::EventSpawnKind;
use crate::World;

/// The door slot: its single event (`event2` 0) holds the starting gates.
pub const EVENT_DOOR: u8 = 254;
/// The ghost-gate slot: barriers that keep the dead in the start area.
pub const EVENT_GHOST_GATE: u8 = 253;

/// Which battleground events each spawn belongs to.
#[derive(Default)]
pub struct EventIndex {
    by_spawn: RwLock<HashMap<(EventSpawnKind, u32), Vec<(u8, u8)>>>,
    by_event: RwLock<HashMap<(u8, u8), Vec<(EventSpawnKind, u32)>>>,
}

impl EventIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load `creature_battleground` and `gameobject_battleground`.
    pub async fn load(&self, pool: &PgPool) -> Result<usize> {
        let creatures = sqlx::query_as::<_, (i64, i32, i32)>(
            "SELECT guid::bigint, event1::int, event2::int FROM world.creature_battleground",
        )
        .fetch_all(pool)
        .await?;
        let gameobjects = sqlx::query_as::<_, (i64, i32, i32)>(
            "SELECT guid::bigint, event1::int, event2::int FROM world.gameobject_battleground",
        )
        .fetch_all(pool)
        .await?;
        let rows = creatures
            .into_iter()
            .map(|row| (EventSpawnKind::Creature, row))
            .chain(
                gameobjects
                    .into_iter()
                    .map(|row| (EventSpawnKind::GameObject, row)),
            );
        let mut by_spawn: HashMap<(EventSpawnKind, u32), Vec<(u8, u8)>> = HashMap::new();
        let mut by_event: HashMap<(u8, u8), Vec<(EventSpawnKind, u32)>> = HashMap::new();
        let mut count = 0;
        for (kind, (guid, event1, event2)) in rows {
            let (Ok(event1), Ok(event2), Ok(spawn_id)) =
                (u8::try_from(event1), u8::try_from(event2), u32::try_from(guid))
            else {
                continue;
            };
            if event1 == EVENT_NONE || event2 == EVENT_NONE {
                continue;
            }
            by_spawn.entry((kind, spawn_id)).or_default().push((event1, event2));
            by_event.entry((event1, event2)).or_default().push((kind, spawn_id));
            count += 1;
        }
        *self.by_spawn.write() = by_spawn;
        *self.by_event.write() = by_event;
        Ok(count)
    }

    /// The events a spawn belongs to (empty: always spawned).
    pub fn events_of(&self, kind: EventSpawnKind, spawn_id: u32) -> Vec<(u8, u8)> {
        self.by_spawn
            .read()
            .get(&(kind, spawn_id))
            .cloned()
            .unwrap_or_default()
    }

    /// The spawns of an event on a map.
    fn spawns_of(&self, world: &World, map_id: u32, event1: u8, event2: u8) -> Vec<(EventSpawnKind, u32)> {
        self.by_event
            .read()
            .get(&(event1, event2))
            .map(|spawns| {
                spawns
                    .iter()
                    .copied()
                    .filter(|&(kind, spawn_id)| spawn_map(world, kind, spawn_id) == Some(map_id))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn spawn_map(world: &World, kind: EventSpawnKind, spawn_id: u32) -> Option<u32> {
    match kind {
        EventSpawnKind::Creature => world
            .managers
            .creature_mgr
            .get_spawn_data_by_id(spawn_id)
            .map(|s| s.map_id),
        EventSpawnKind::GameObject => world
            .managers
            .gameobject_mgr
            .get_spawn_data_by_id(spawn_id)
            .map(|s| s.map_id),
    }
}

/// Whether a spawn may exist in a copy under the copy's active events.
pub fn is_spawn_allowed(
    world: &World,
    map_id: u32,
    instance_id: u32,
    kind: EventSpawnKind,
    spawn_id: u32,
) -> bool {
    let events = world.managers.bg_events.events_of(kind, spawn_id);
    events.iter().all(|&(event1, event2)| {
        world
            .managers
            .scope_states
            .is_event_active(map_id, instance_id, event1, event2)
    })
}

/// Switch an event on (replacing the slot's active event) or off, spawning
/// and removing what changes in loaded grids.
pub async fn spawn_event(world: &World, map_id: u32, instance_id: u32, event1: u8, event2: u8, spawn: bool) {
    let states = &world.managers.scope_states;
    let current = states.active_event(map_id, instance_id, event1);
    if event2 == EVENT_NONE || (spawn && current == event2) || (!spawn && current != event2) {
        return;
    }
    if spawn {
        if current != EVENT_NONE {
            apply_event(world, map_id, instance_id, event1, current, false).await;
        }
        states.set_active_event(map_id, instance_id, event1, event2);
    } else {
        states.set_active_event(map_id, instance_id, event1, EVENT_NONE);
    }
    apply_event(world, map_id, instance_id, event1, event2, spawn).await;
}

async fn apply_event(world: &World, map_id: u32, instance_id: u32, event1: u8, event2: u8, spawn: bool) {
    for (kind, spawn_id) in world
        .managers
        .bg_events
        .spawns_of(world, map_id, event1, event2)
    {
        let allowed = spawn && is_spawn_allowed(world, map_id, instance_id, kind, spawn_id);
        match kind {
            EventSpawnKind::Creature => {
                let present = world
                    .managers
                    .creature_mgr
                    .has_spawn(map_id, instance_id, spawn_id);
                if allowed && !present {
                    let Some(spawn) = world.managers.creature_mgr.get_spawn_data_by_id(spawn_id)
                    else {
                        continue;
                    };
                    if !crate::game::game_event::is_grid_loaded(world, map_id, instance_id, spawn.position) {
                        continue;
                    }
                    if let Some(guid) =
                        crate::game::game_event::spawn_creature_in_copy(world, spawn_id, instance_id)
                    {
                        // Boxed: the script's answer may switch further events.
                        Box::pin(crate::game::instance::scripts::on_creature_create(
                            world,
                            map_id,
                            instance_id,
                            spawn.entry,
                            guid,
                        ))
                        .await;
                    }
                } else if !allowed && present {
                    crate::game::game_event::despawn_creature_in_copy(world, spawn_id, instance_id);
                }
            }
            EventSpawnKind::GameObject => {
                let present = world
                    .managers
                    .gameobject_mgr
                    .has_spawn(map_id, instance_id, spawn_id);
                if allowed && !present {
                    let Some(spawn) = world.managers.gameobject_mgr.get_spawn_data_by_id(spawn_id)
                    else {
                        continue;
                    };
                    if !crate::game::game_event::is_grid_loaded(world, map_id, instance_id, spawn.position) {
                        continue;
                    }
                    if let Some(guid) =
                        crate::game::game_event::spawn_gameobject_in_copy(world, spawn_id, instance_id)
                    {
                        // Boxed: the script's answer may switch further events.
                        Box::pin(crate::game::instance::scripts::on_gameobject_create(
                            world,
                            map_id,
                            instance_id,
                            spawn.entry,
                            guid,
                        ))
                        .await;
                    }
                } else if !allowed && present {
                    crate::game::game_event::despawn_gameobject_in_copy(world, spawn_id, instance_id);
                }
            }
        }
    }
}

/// The gameobjects of an event that are spawned in a copy.
pub fn event_gameobjects(world: &World, map_id: u32, instance_id: u32, event1: u8, event2: u8) -> Vec<ObjectGuid> {
    let spawn_ids: Vec<u32> = world
        .managers
        .bg_events
        .spawns_of(world, map_id, event1, event2)
        .into_iter()
        .filter(|(kind, _)| *kind == EventSpawnKind::GameObject)
        .map(|(_, spawn_id)| spawn_id)
        .collect();
    let gameobject_mgr = &world.managers.gameobject_mgr;
    gameobject_mgr
        .gameobjects_in_copy(map_id, instance_id)
        .into_iter()
        .filter(|&guid| {
            gameobject_mgr
                .with_gameobject(guid, |go| spawn_ids.contains(&go.spawn_id))
                .unwrap_or(false)
        })
        .collect()
}

/// The creatures of an event that are spawned in a copy.
pub fn event_creatures(world: &World, map_id: u32, instance_id: u32, event1: u8, event2: u8) -> Vec<ObjectGuid> {
    let spawn_ids: Vec<u32> = world
        .managers
        .bg_events
        .spawns_of(world, map_id, event1, event2)
        .into_iter()
        .filter(|(kind, _)| *kind == EventSpawnKind::Creature)
        .map(|(_, spawn_id)| spawn_id)
        .collect();
    let creature_mgr = &world.managers.creature_mgr;
    creature_mgr
        .creatures_in_copy(map_id, instance_id)
        .into_iter()
        .filter(|&guid| {
            creature_mgr
                .get_spawn_id(guid)
                .is_some_and(|id| spawn_ids.contains(&id))
        })
        .collect()
}

/// Open (or close) every door of an active event.
pub fn door_event(world: &World, map_id: u32, instance_id: u32, event1: u8, event2: u8, open: bool) {
    if !world
        .managers
        .scope_states
        .is_event_active(map_id, instance_id, event1, event2)
    {
        return;
    }
    for guid in event_gameobjects(world, map_id, instance_id, event1, event2) {
        crate::game::instance::scripts::set_door(world, guid, open);
    }
}

/// Whether a gameobject is part of an event (used to tell flags and banners apart).
pub fn gameobject_event(world: &World, guid: ObjectGuid) -> Option<(u8, u8)> {
    let spawn_id = world
        .managers
        .gameobject_mgr
        .with_gameobject(guid, |go| go.spawn_id)?;
    world
        .managers
        .bg_events
        .events_of(EventSpawnKind::GameObject, spawn_id)
        .first()
        .copied()
}
