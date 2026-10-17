//! Game events: which world events are running, and which spawns they control.
//!
//! An event runs for `length` minutes every `occurence` minutes between its
//! start and end dates (disabled and script-controlled events never start on
//! their own). Spawns listed in `game_event_creature` / `game_event_gameobject`
//! with a positive event exist only while that event runs; a negative event
//! id means the spawn exists only while that event is *not* running. Grid
//! loading consults this, and when an event starts or stops, spawns in loaded
//! grids are added or removed straight away.

use crate::World;
use anyhow::{Context, Result};
use oxcore_shared::messages::inventory::SmsgDestroyObject;
use oxcore_shared::protocol::{ObjectGuid, Position};
use parking_lot::RwLock;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct GameEventData {
    start: i64,
    end: i64,
    occurence_secs: i64,
    length_secs: i64,
    /// Started and stopped only by scripts.
    hardcoded: bool,
}

impl GameEventData {
    fn is_running(&self, now: i64) -> bool {
        if self.hardcoded || now < self.start || now >= self.end {
            return false;
        }
        if self.occurence_secs <= 0 {
            return now < self.start + self.length_secs;
        }
        (now - self.start) % self.occurence_secs < self.length_secs
    }
}

/// Spawn kinds an event can control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventSpawnKind {
    Creature,
    GameObject,
}

#[derive(Default)]
pub struct GameEventManager {
    events: RwLock<HashMap<u16, GameEventData>>,
    /// Spawn -> signed event ids it depends on.
    spawn_events: RwLock<HashMap<(EventSpawnKind, u32), Vec<i16>>>,
    /// Event id -> spawns that reference it (either sign).
    event_spawns: RwLock<HashMap<u16, Vec<(EventSpawnKind, u32)>>>,
    active: RwLock<HashSet<u16>>,
    /// Events started by scripts or commands regardless of their schedule.
    forced: RwLock<HashSet<u16>>,
}

impl GameEventManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load events (for the content patch) and their spawn links.
    pub async fn load(&self, pool: &PgPool, patch: u8) -> Result<()> {
        let rows: Vec<(i64, i64, i64, i64, i64, i16, i16)> = sqlx::query_as(
            "SELECT entry,
                    EXTRACT(EPOCH FROM start_time)::BIGINT,
                    EXTRACT(EPOCH FROM end_time)::BIGINT,
                    occurence::BIGINT, length::BIGINT, hardcoded, disabled
             FROM world.game_event
             WHERE $1 BETWEEN patch_min AND patch_max",
        )
        .bind(i16::from(patch))
        .fetch_all(pool)
        .await
        .context("Failed to load game events")?;

        let mut events = HashMap::new();
        for (entry, start, end, occurence, length, hardcoded, disabled) in rows {
            let Ok(id) = u16::try_from(entry) else {
                continue;
            };
            if disabled != 0 {
                continue;
            }
            events.insert(
                id,
                GameEventData {
                    start,
                    end,
                    occurence_secs: occurence * 60,
                    length_secs: length * 60,
                    hardcoded: hardcoded != 0,
                },
            );
        }

        let mut spawn_events: HashMap<(EventSpawnKind, u32), Vec<i16>> = HashMap::new();
        let mut event_spawns: HashMap<u16, Vec<(EventSpawnKind, u32)>> = HashMap::new();
        for (kind, table) in [
            (EventSpawnKind::Creature, "game_event_creature"),
            (EventSpawnKind::GameObject, "game_event_gameobject"),
        ] {
            let links: Vec<(i64, i16)> =
                sqlx::query_as(&format!("SELECT guid, event FROM world.{table}"))
                    .fetch_all(pool)
                    .await
                    .with_context(|| format!("Failed to load {table}"))?;
            for (guid, event) in links {
                let (Ok(spawn_id), Some(id)) =
                    (u32::try_from(guid), u16::try_from(event.unsigned_abs()).ok())
                else {
                    continue;
                };
                if event == 0 {
                    continue;
                }
                spawn_events.entry((kind, spawn_id)).or_default().push(event);
                event_spawns.entry(id).or_default().push((kind, spawn_id));
            }
        }

        tracing::info!(
            "Loaded {} game events controlling {} spawns",
            events.len(),
            spawn_events.len()
        );
        *self.events.write() = events;
        *self.spawn_events.write() = spawn_events;
        *self.event_spawns.write() = event_spawns;
        *self.active.write() = self.compute_active(now_secs());
        Ok(())
    }

    fn compute_active(&self, now: i64) -> HashSet<u16> {
        let mut active: HashSet<u16> = self
            .events
            .read()
            .iter()
            .filter(|(_, event)| event.is_running(now))
            .map(|(&id, _)| id)
            .collect();
        active.extend(self.forced.read().iter().copied());
        active
    }

    pub fn is_active(&self, event: u16) -> bool {
        self.active.read().contains(&event)
    }

    /// Start or stop an event from a script or command, regardless of schedule.
    pub fn set_forced(&self, event: u16, running: bool) {
        if running {
            self.forced.write().insert(event);
        } else {
            self.forced.write().remove(&event);
        }
    }

    /// Whether a spawn may exist under the events running now.
    pub fn is_spawn_eligible(&self, kind: EventSpawnKind, spawn_id: u32) -> bool {
        let links = self.spawn_events.read();
        let Some(events) = links.get(&(kind, spawn_id)) else {
            return true;
        };
        let active = self.active.read();
        let mut has_positive = false;
        let mut positive_running = false;
        for &event in events {
            let id = event.unsigned_abs();
            if event > 0 {
                has_positive = true;
                positive_running |= active.contains(&id);
            } else if active.contains(&id) {
                return false;
            }
        }
        !has_positive || positive_running
    }

    /// Recompute running events; returns the spawns whose eligibility may have changed.
    fn advance(&self, now: i64) -> Vec<(EventSpawnKind, u32)> {
        let next = self.compute_active(now);
        let changed: Vec<u16> = {
            let current = self.active.read();
            current.symmetric_difference(&next).copied().collect()
        };
        if changed.is_empty() {
            return Vec::new();
        }
        for id in &changed {
            tracing::info!(
                "Game event {} {}",
                id,
                if next.contains(id) { "started" } else { "stopped" }
            );
        }
        *self.active.write() = next;
        let spawns = self.event_spawns.read();
        let mut affected: Vec<(EventSpawnKind, u32)> = changed
            .iter()
            .filter_map(|id| spawns.get(id))
            .flatten()
            .copied()
            .collect();
        affected.sort_by_key(|(kind, id)| (*kind as u8, *id));
        affected.dedup();
        affected
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Apply event transitions: spawn newly eligible objects whose grid is loaded,
/// and remove objects whose event no longer allows them.
pub fn update(world: &World) {
    let affected = world.managers.game_event_mgr.advance(now_secs());
    for (kind, spawn_id) in affected {
        let eligible = world.managers.game_event_mgr.is_spawn_eligible(kind, spawn_id);
        match kind {
            EventSpawnKind::Creature => update_creature(world, spawn_id, eligible),
            EventSpawnKind::GameObject => update_gameobject(world, spawn_id, eligible),
        }
    }
}

/// Copies of a map whose grid at `position` is loaded.
fn loaded_copies_at(world: &World, map_id: u32, position: Position) -> Vec<u32> {
    use crate::map::grid_coords::world_to_grid;
    let (gx, gy) = world_to_grid(position.x, position.y);
    world
        .managers
        .map_mgr
        .get_active_map_keys()
        .into_iter()
        .filter(|&(map, _)| map == map_id)
        .filter(|&(map, instance)| {
            world
                .managers
                .map_mgr
                .get_map(map, instance)
                .is_some_and(|m| {
                    m.grid_manager()
                        .read()
                        .get_grid(gx, gy)
                        .is_some_and(|g| g.state().is_loaded())
                })
        })
        .map(|(_, instance)| instance)
        .collect()
}

fn players_near(world: &World, map_id: u32, instance_id: u32, position: Position) -> Vec<ObjectGuid> {
    world
        .managers
        .map_mgr
        .get_map(map_id, instance_id)
        .map(|map| map.get_players_in_range(position, map.visibility_distance()))
        .unwrap_or_default()
}

fn update_creature(world: &World, spawn_id: u32, eligible: bool) {
    let creature_mgr = &world.managers.creature_mgr;
    let Some(spawn) = creature_mgr.get_spawn_data_by_id(spawn_id) else {
        return;
    };
    for instance_id in loaded_copies_at(world, spawn.map_id, spawn.position) {
        let present = creature_mgr.has_spawn(spawn.map_id, instance_id, spawn_id);
        if eligible && !present {
            if !world.systems.pool.can_creature_spawn(spawn_id, instance_id) {
                continue;
            }
            let map = world
                .managers
                .map_mgr
                .get_or_create_map(spawn.map_id, instance_id);
            let Some(guid) =
                creature_mgr.spawn_into_map(&spawn, &map, Some(&world.managers.waypoint_mgr))
            else {
                continue;
            };
            if spawn.spawn_flags & crate::game::creature::spawn::spawn_flags::ACTIVE != 0 {
                map.add_to_active(guid, spawn.position);
            }
            world
                .systems
                .pool
                .on_creature_spawned(spawn_id, instance_id, guid);
            if let Some(create) = creature_mgr.build_create_msg(guid, world) {
                for player in players_near(world, spawn.map_id, instance_id, spawn.position) {
                    world
                        .managers
                        .broadcast_mgr
                        .send_msg_to_player(player, create.clone());
                }
            }
        } else if !eligible && present {
            let guids: Vec<ObjectGuid> = creature_mgr
                .creatures_in_copy(spawn.map_id, instance_id)
                .into_iter()
                .filter(|&guid| creature_mgr.get_spawn_id(guid) == Some(spawn_id))
                .collect();
            for guid in guids {
                let position = creature_mgr.get_position(guid).unwrap_or(spawn.position);
                if let Some(map) = world.managers.map_mgr.get_map(spawn.map_id, instance_id) {
                    map.remove_creature(guid, position);
                    map.remove_from_active(guid);
                }
                let viewers = players_near(world, spawn.map_id, instance_id, position);
                world
                    .managers
                    .broadcast_mgr
                    .broadcast_msg_to_players(&viewers, &SmsgDestroyObject { guid });
                world.systems.loot.remove_loot(guid);
                world.systems.pool.on_object_despawned(guid);
                creature_mgr.remove_creature(guid);
            }
        }
    }
}

fn update_gameobject(world: &World, spawn_id: u32, eligible: bool) {
    let gameobject_mgr = &world.managers.gameobject_mgr;
    let Some(spawn) = gameobject_mgr.get_spawn_data_by_id(spawn_id) else {
        return;
    };
    for instance_id in loaded_copies_at(world, spawn.map_id, spawn.position) {
        let present = gameobject_mgr.has_spawn(spawn.map_id, instance_id, spawn_id);
        if eligible && !present {
            if !world.systems.pool.can_gameobject_spawn(spawn_id, instance_id) {
                continue;
            }
            let Some(guid) = gameobject_mgr.spawn_gameobject(&spawn, instance_id) else {
                continue;
            };
            world
                .systems
                .pool
                .on_gameobject_spawned(spawn_id, instance_id, guid);
            let in_world = gameobject_mgr
                .with_gameobject(guid, |go| go.in_world)
                .unwrap_or(false);
            if !in_world {
                continue;
            }
            let map = world
                .managers
                .map_mgr
                .get_or_create_map(spawn.map_id, instance_id);
            map.add_gameobject(guid, spawn.position);
            gameobject_mgr.add_collision_model(guid, &world.managers.vmap_mgr);
            for player in players_near(world, spawn.map_id, instance_id, spawn.position) {
                if let Some(create) = gameobject_mgr.build_create_msg(guid, player, world) {
                    world.managers.broadcast_mgr.send_msg_to_player(player, create);
                }
            }
        } else if !eligible && present {
            let guids: Vec<ObjectGuid> = gameobject_mgr
                .gameobjects_in_copy(spawn.map_id, instance_id)
                .into_iter()
                .filter(|&guid| {
                    gameobject_mgr
                        .with_gameobject(guid, |go| go.spawn_id)
                        .is_some_and(|id| id == spawn_id)
                })
                .collect();
            for guid in guids {
                if let Some(map) = world.managers.map_mgr.get_map(spawn.map_id, instance_id) {
                    map.remove_gameobject(guid, spawn.position);
                }
                let viewers = players_near(world, spawn.map_id, instance_id, spawn.position);
                world
                    .managers
                    .broadcast_mgr
                    .broadcast_msg_to_players(&viewers, &SmsgDestroyObject { guid });
                world.systems.pool.on_object_despawned(guid);
                gameobject_mgr.remove_collision_model(guid, &world.managers.vmap_mgr);
                gameobject_mgr.remove_gameobject(guid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(start: i64, occurence_min: i64, length_min: i64) -> GameEventData {
        GameEventData {
            start,
            end: start + 365 * 86_400,
            occurence_secs: occurence_min * 60,
            length_secs: length_min * 60,
            hardcoded: false,
        }
    }

    #[test]
    fn events_run_for_their_length_every_occurence() {
        let e = event(1_000, 60, 10);
        assert!(!e.is_running(999));
        assert!(e.is_running(1_000));
        assert!(e.is_running(1_000 + 599));
        assert!(!e.is_running(1_000 + 600));
        assert!(e.is_running(1_000 + 3_600));
        assert!(!e.is_running(1_000 + 365 * 86_400));
    }

    #[test]
    fn script_controlled_events_never_start_on_their_own() {
        let mut e = event(0, 60, 60);
        e.hardcoded = true;
        assert!(!e.is_running(10));
    }

    #[test]
    fn positive_links_need_a_running_event_and_negative_links_forbid_one() {
        let mgr = GameEventManager::new();
        mgr.spawn_events
            .write()
            .insert((EventSpawnKind::Creature, 1), vec![5]);
        mgr.spawn_events
            .write()
            .insert((EventSpawnKind::Creature, 2), vec![-5]);
        assert!(!mgr.is_spawn_eligible(EventSpawnKind::Creature, 1));
        assert!(mgr.is_spawn_eligible(EventSpawnKind::Creature, 2));
        assert!(mgr.is_spawn_eligible(EventSpawnKind::Creature, 3));

        mgr.set_forced(5, true);
        mgr.event_spawns.write().insert(
            5,
            vec![(EventSpawnKind::Creature, 1), (EventSpawnKind::Creature, 2)],
        );
        let changed = mgr.advance(0);
        assert_eq!(changed.len(), 2);
        assert!(mgr.is_spawn_eligible(EventSpawnKind::Creature, 1));
        assert!(!mgr.is_spawn_eligible(EventSpawnKind::Creature, 2));
        assert!(mgr.advance(0).is_empty());
    }
}
