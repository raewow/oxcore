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
    /// Events stopped by commands regardless of their schedule.
    suppressed: RwLock<HashSet<u16>>,
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
        let suppressed = self.suppressed.read();
        active.retain(|id| !suppressed.contains(id));
        active.extend(self.forced.read().iter().copied());
        active
    }

    pub fn is_active(&self, event: u16) -> bool {
        self.active.read().contains(&event)
    }

    /// Start or stop an event from a script or command, regardless of schedule.
    pub fn set_forced(&self, event: u16, running: bool) {
        if running {
            self.suppressed.write().remove(&event);
            self.forced.write().insert(event);
        } else {
            self.forced.write().remove(&event);
            self.suppressed.write().insert(event);
        }
    }

    /// Hand an event back to its schedule.
    pub fn clear_forced(&self, event: u16) {
        self.forced.write().remove(&event);
        self.suppressed.write().remove(&event);
    }

    /// Whether an event is known (loaded for this patch).
    pub fn is_known(&self, event: u16) -> bool {
        self.events.read().contains_key(&event)
    }

    /// Running events, sorted.
    pub fn active_events(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self.active.read().iter().copied().collect();
        ids.sort_unstable();
        ids
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

    /// Recompute running events; returns the spawns whose eligibility may have
    /// changed and the spawns linked to events that just started.
    fn advance(&self, now: i64) -> (Vec<(EventSpawnKind, u32)>, Vec<(EventSpawnKind, u32)>) {
        let next = self.compute_active(now);
        let changed: Vec<u16> = {
            let current = self.active.read();
            current.symmetric_difference(&next).copied().collect()
        };
        if changed.is_empty() {
            return (Vec::new(), Vec::new());
        }
        for id in &changed {
            tracing::info!(
                "Game event {} {}",
                id,
                if next.contains(id) { "started" } else { "stopped" }
            );
        }
        let spawns = self.event_spawns.read();
        let links = self.spawn_events.read();
        let link_map = &*links;
        let started: Vec<(EventSpawnKind, u32)> = changed
            .iter()
            .filter(|id| next.contains(id))
            .filter_map(|id| spawns.get(id).map(|list| (*id, list)))
            .flat_map(|(id, list)| {
                list.iter()
                    .filter(move |spawn| {
                        link_map
                            .get(spawn)
                            .is_some_and(|events| events.contains(&(id as i16)))
                    })
                    .copied()
            })
            .collect();
        let mut affected: Vec<(EventSpawnKind, u32)> = changed
            .iter()
            .filter_map(|id| spawns.get(id))
            .flatten()
            .copied()
            .collect();
        affected.sort_by_key(|(kind, id)| (*kind as u8, *id));
        affected.dedup();
        drop(spawns);
        drop(links);
        *self.active.write() = next;
        (affected, started)
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
    let (affected, started) = world.managers.game_event_mgr.advance(now_secs());

    // A pool whose members belong to an event is that event's pool: it rolls a
    // fresh selection each time the event starts.
    let mut event_pools: Vec<(u32, u32)> = Vec::new();
    for (kind, spawn_id) in &started {
        use crate::game::coordination::pool_types::PoolMemberType;
        let (member_kind, map_id) = match kind {
            EventSpawnKind::Creature => (
                PoolMemberType::Creature,
                world
                    .managers
                    .creature_mgr
                    .get_spawn_data_by_id(*spawn_id)
                    .map(|s| s.map_id),
            ),
            EventSpawnKind::GameObject => (
                PoolMemberType::GameObject,
                world
                    .managers
                    .gameobject_mgr
                    .get_spawn_data_by_id(*spawn_id)
                    .map(|s| s.map_id),
            ),
        };
        if let (Some(pool), Some(map_id)) =
            (world.managers.pool_mgr.top_pool_of(member_kind, *spawn_id), map_id)
        {
            if !event_pools.contains(&(pool, map_id)) {
                event_pools.push((pool, map_id));
            }
        }
    }
    for (pool, map_id) in event_pools {
        for (map, instance_id) in world.managers.map_mgr.get_active_map_keys() {
            if map == map_id {
                world.systems.pool.despawn_pool(pool, instance_id, world);
                world.systems.pool.spawn_pool(pool, instance_id, world);
            }
        }
    }

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
            spawn_creature_in_copy(world, spawn_id, instance_id);
        } else if !eligible && present {
            despawn_creature_in_copy(world, spawn_id, instance_id);
        }
    }
}

/// Whether the grid holding `position` is loaded in a copy.
pub(crate) fn is_grid_loaded(world: &World, map_id: u32, instance_id: u32, position: Position) -> bool {
    use crate::map::grid_coords::world_to_grid;
    let (gx, gy) = world_to_grid(position.x, position.y);
    world
        .managers
        .map_mgr
        .get_map(map_id, instance_id)
        .is_some_and(|m| {
            m.grid_manager()
                .read()
                .get_grid(gx, gy)
                .is_some_and(|g| g.state().is_loaded())
        })
}

/// Spawn a database creature into one copy and show it to nearby players.
pub(crate) fn spawn_creature_in_copy(
    world: &World,
    spawn_id: u32,
    instance_id: u32,
) -> Option<ObjectGuid> {
    let creature_mgr = &world.managers.creature_mgr;
    let spawn = creature_mgr.get_spawn_data_by_id(spawn_id)?;
    if creature_mgr.has_spawn(spawn.map_id, instance_id, spawn_id) {
        return None;
    }
    if !world.systems.pool.can_creature_spawn(spawn_id, instance_id) {
        return None;
    }
    let map = world
        .managers
        .map_mgr
        .get_or_create_map(spawn.map_id, instance_id);
    let guid = creature_mgr.spawn_into_map(&spawn, &map, Some(&world.managers.waypoint_mgr))?;
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
    Some(guid)
}

/// Remove a database creature from one copy.
pub(crate) fn despawn_creature_in_copy(world: &World, spawn_id: u32, instance_id: u32) {
    let creature_mgr = &world.managers.creature_mgr;
    let Some(spawn) = creature_mgr.get_spawn_data_by_id(spawn_id) else {
        return;
    };
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

fn update_gameobject(world: &World, spawn_id: u32, eligible: bool) {
    let gameobject_mgr = &world.managers.gameobject_mgr;
    let Some(spawn) = gameobject_mgr.get_spawn_data_by_id(spawn_id) else {
        return;
    };
    for instance_id in loaded_copies_at(world, spawn.map_id, spawn.position) {
        let present = gameobject_mgr.has_spawn(spawn.map_id, instance_id, spawn_id);
        if eligible && !present {
            spawn_gameobject_in_copy(world, spawn_id, instance_id);
        } else if !eligible && present {
            despawn_gameobject_in_copy(world, spawn_id, instance_id);
        }
    }
}

/// Spawn a database gameobject into one copy and show it to nearby players.
pub(crate) fn spawn_gameobject_in_copy(
    world: &World,
    spawn_id: u32,
    instance_id: u32,
) -> Option<ObjectGuid> {
    let gameobject_mgr = &world.managers.gameobject_mgr;
    let spawn = gameobject_mgr.get_spawn_data_by_id(spawn_id)?;
    if gameobject_mgr.has_spawn(spawn.map_id, instance_id, spawn_id) {
        return None;
    }
    if !world.systems.pool.can_gameobject_spawn(spawn_id, instance_id) {
        return None;
    }
    let guid = gameobject_mgr.spawn_gameobject(&spawn, instance_id)?;
    world
        .systems
        .pool
        .on_gameobject_spawned(spawn_id, instance_id, guid);
    let in_world = gameobject_mgr
        .with_gameobject(guid, |go| go.in_world)
        .unwrap_or(false);
    if !in_world {
        return Some(guid);
    }
    let map = world
        .managers
        .map_mgr
        .get_or_create_map(spawn.map_id, instance_id);
    map.add_gameobject(guid, spawn.position);
    if gameobject_mgr.is_spawn_active(spawn_id) {
        map.add_to_active(guid, spawn.position);
    }
    gameobject_mgr.add_collision_model(guid, &world.managers.vmap_mgr);
    for player in players_near(world, spawn.map_id, instance_id, spawn.position) {
        if let Some(create) = gameobject_mgr.build_create_msg(guid, player, world) {
            world.managers.broadcast_mgr.send_msg_to_player(player, create);
        }
    }
    Some(guid)
}

/// Remove a database gameobject from one copy.
pub(crate) fn despawn_gameobject_in_copy(world: &World, spawn_id: u32, instance_id: u32) {
    let gameobject_mgr = &world.managers.gameobject_mgr;
    let Some(spawn) = gameobject_mgr.get_spawn_data_by_id(spawn_id) else {
        return;
    };
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
            map.remove_from_active(guid);
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
        let (changed, started) = mgr.advance(0);
        assert_eq!(changed.len(), 2);
        // Only the positively linked spawn belongs to the event that started.
        assert_eq!(started, vec![(EventSpawnKind::Creature, 1)]);
        assert!(mgr.is_spawn_eligible(EventSpawnKind::Creature, 1));
        assert!(!mgr.is_spawn_eligible(EventSpawnKind::Creature, 2));
        assert!(mgr.advance(0).0.is_empty());
    }
}
