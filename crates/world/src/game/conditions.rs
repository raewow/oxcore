use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::{PgPool, Row};
use std::collections::HashSet;

use crate::World;
use oxcore_shared::protocol::ObjectGuid;

#[derive(Debug, Clone)]
pub struct ConditionEntry {
    pub kind: i8,
    pub values: [i32; 4],
    pub flags: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ConditionContext {
    pub target: ObjectGuid,
    pub source: ObjectGuid,
}

pub struct ConditionManager {
    entries: DashMap<u32, ConditionEntry>,
}

impl ConditionManager {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    pub async fn load(&self, world_db: &PgPool) -> Result<()> {
        let rows = sqlx::query(
            "SELECT condition_entry, type, value1, value2, value3, value4, flags FROM world.conditions",
        )
        .fetch_all(world_db)
        .await
        .context("Failed to load conditions")?;

        self.entries.clear();
        for row in rows {
            self.entries.insert(
                u32::try_from(row.try_get::<i64, _>("condition_entry")?)?,
                ConditionEntry {
                    kind: i8::try_from(row.try_get::<i16, _>("type")?)?,
                    values: [
                        row.try_get("value1")?,
                        row.try_get("value2")?,
                        row.try_get("value3")?,
                        row.try_get("value4")?,
                    ],
                    flags: u8::try_from(row.try_get::<i16, _>("flags")?)?,
                },
            );
        }
        tracing::info!(count = self.entries.len(), "Loaded conditions");
        Ok(())
    }

    #[cfg(test)]
    pub fn add_for_test(&self, id: u32, entry: ConditionEntry) {
        self.entries.insert(id, entry);
    }

    pub fn is_satisfied(&self, id: u32, context: ConditionContext, world: &World) -> bool {
        self.evaluate(id, context, world, &mut HashSet::new())
    }

    fn evaluate(
        &self,
        id: u32,
        mut context: ConditionContext,
        world: &World,
        visiting: &mut HashSet<u32>,
    ) -> bool {
        if !visiting.insert(id) {
            tracing::warn!(condition_id = id, "Condition cycle rejected");
            return false;
        }
        let Some(entry) = self.entries.get(&id).map(|entry| entry.clone()) else {
            tracing::warn!(condition_id = id, "Unknown condition rejected");
            return false;
        };
        if entry.flags & 0x02 != 0 {
            std::mem::swap(&mut context.target, &mut context.source);
        }
        let result = match entry.kind {
            -3 => !self.evaluate(entry.values[0] as u32, context, world, visiting),
            -2 => entry
                .values
                .into_iter()
                .filter(|id| *id != 0)
                .any(|id| self.evaluate(id as u32, context, world, visiting)),
            -1 => entry
                .values
                .into_iter()
                .filter(|id| *id != 0)
                .all(|id| self.evaluate(id as u32, context, world, visiting)),
            16 => object_entry(context.source, world)
                .is_some_and(|entry_id| entry.values.into_iter().any(|value| value == entry_id)),
            41 => object_health_percent(context.target, world)
                .is_some_and(|percent| compare(percent as i32, entry.values[0], entry.values[1])),
            48 => world
                .managers
                .gameobject_mgr
                .with_gameobject(context.target, |gameobject| gameobject.in_world)
                .unwrap_or(false),
            49 => world
                .managers
                .gameobject_mgr
                .with_gameobject(context.target, |gameobject| {
                    gameobject.loot_state as i32 == entry.values[0]
                })
                .unwrap_or(false),
            55 => world
                .managers
                .gameobject_mgr
                .with_gameobject(context.target, |gameobject| {
                    gameobject.go_state as i32 == entry.values[0]
                })
                .unwrap_or(false),
            1 => has_aura(context.target, entry.values[0] as u32, world),
            4 => player_zone(context.source, world)
                .or_else(|| player_zone(context.target, world))
                .is_some_and(|zone| zone == entry.values[0] as u32),
            6 => world
                .managers
                .player_mgr
                .with_player(context.target, |p| p.get_team() as i32 == entry.values[0])
                .unwrap_or(false),
            8 => world
                .managers
                .player_mgr
                .with_player(context.target, |p| {
                    p.rewarded_quests.contains(&(entry.values[0] as u32))
                })
                .unwrap_or(false),
            9 => world
                .managers
                .player_mgr
                .with_player(context.target, |p| {
                    p.active_quests
                        .iter()
                        .any(|q| q.quest_id == entry.values[0] as u32)
                })
                .unwrap_or(false),
            12 => world
                .managers
                .game_event_mgr
                .is_active(entry.values[0] as u16),
            15 => unit_level(context.target, world)
                .is_some_and(|level| compare(level, entry.values[0], entry.values[1])),
            20 => nearby_creature(
                context.target,
                entry.values[0] as u32,
                entry.values[1] as f32,
                entry.values[2] == 0,
                entry.values[3] != 0,
                world,
            ),
            21 => nearby_gameobject(context.target, entry.values[0] as u32, entry.values[1] as f32, world),
            24 => compare(
                world.managers.creature_mgr.get_patch() as i32,
                entry.values[0],
                entry.values[1],
            ),
            28 => {
                if entry.values[0] != 0 {
                    context.target.is_player()
                        || crate::game::creature::ai::units::resolve_unit(world, context.target)
                            .is_some_and(|u| u.is_player_controlled())
                } else {
                    context.target.is_player()
                }
            }
            33 => object_map(context.source, world)
                .or_else(|| object_map(context.target, world))
                .is_some_and(|(map, _)| map == entry.values[0] as u32),
            34 => object_map(context.source, world)
                .or_else(|| object_map(context.target, world))
                .is_some_and(|(map, instance)| {
                    let value = world
                        .managers
                        .lua_mgr
                        .get_instance_state(map, instance)
                        .data
                        .get(&(entry.values[0] as u32))
                        .copied()
                        .unwrap_or(0) as i32;
                    compare(value, entry.values[1], entry.values[2])
                }),
            37 => match (object_position(context.source, world), object_position(context.target, world)) {
                (Some((map, instance, a)), Some((_, _, b))) => world
                    .managers
                    .vmap_mgr
                    .is_in_line_of_sight(map, instance, a, b),
                _ => false,
            },
            38 => match (object_position(context.source, world), object_position(context.target, world)) {
                (Some((_, _, a)), Some((_, _, b))) => compare(
                    crate::game::creature::ai::units::distance(&a, &b) as i32,
                    entry.values[0],
                    entry.values[1],
                ),
                _ => false,
            },
            43 => crate::game::creature::ai::units::resolve_unit(world, context.target)
                .is_some_and(|u| u.in_combat),
            44 => {
                let source_faction = crate::game::creature::ai::units::resolve_unit(world, context.source)
                    .map(|u| u.faction);
                match (crate::game::creature::ai::units::resolve_unit(world, context.target), source_faction) {
                    (Some(target), Some(source_faction)) => {
                        let rank = crate::game::creature::ai::units::reaction_from(
                            world,
                            &target,
                            source_faction,
                        ) as i32;
                        compare(rank, entry.values[0], entry.values[1])
                    }
                    _ => false,
                }
            }
            46 => crate::game::creature::ai::units::resolve_unit(world, context.target)
                .is_some_and(|u| u.alive),
            52 => db_guid(context.source, world)
                .is_some_and(|guid| guid != 0 && entry.values.iter().any(|v| *v as u32 == guid)),
            53 => {
                use chrono::Timelike;
                let now = chrono::Local::now();
                let (hour, minute) = (now.hour() as i32, now.minute() as i32);
                hour >= entry.values[0]
                    && (hour > entry.values[0] || minute >= entry.values[1])
                    && hour <= entry.values[2]
                    && (hour < entry.values[2] || minute <= entry.values[3])
            }
            56 => nearby_player(context.target, entry.values[0], entry.values[1] as f32, world),
            _ => {
                tracing::warn!(
                    condition_id = id,
                    kind = entry.kind,
                    "Unsupported condition rejected"
                );
                false
            }
        };
        visiting.remove(&id);
        if entry.flags & 0x01 != 0 {
            !result
        } else {
            result
        }
    }
}

fn compare(actual: i32, expected: i32, comparator: i32) -> bool {
    match comparator {
        0 => actual == expected,
        1 => actual >= expected,
        2 => actual <= expected,
        _ => false,
    }
}

fn object_entry(guid: ObjectGuid, world: &World) -> Option<i32> {
    if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |creature| creature.entry as i32)
    } else if guid.is_game_object() {
        world
            .managers
            .gameobject_mgr
            .with_gameobject(guid, |gameobject| gameobject.entry as i32)
    } else {
        None
    }
}

fn object_health_percent(guid: ObjectGuid, world: &World) -> Option<u32> {
    if guid.is_player() {
        world.managers.player_mgr.with_player(guid, |player| {
            player.stats.health.saturating_mul(100) / player.stats.max_health.max(1)
        })
    } else if guid.is_creature_or_pet() {
        world.managers.creature_mgr.with_creature(guid, |creature| {
            creature.current_health.saturating_mul(100) / creature.max_health.max(1)
        })
    } else {
        None
    }
}

fn has_aura(guid: ObjectGuid, spell_id: u32, world: &World) -> bool {
    if guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(guid, |p| p.auras.container.has_aura(spell_id))
            .unwrap_or(false)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.auras.has_aura(spell_id))
            .unwrap_or(false)
    }
}

fn player_zone(guid: ObjectGuid, world: &World) -> Option<u32> {
    world.managers.player_mgr.with_player(guid, |p| p.zone_id)
}

fn unit_level(guid: ObjectGuid, world: &World) -> Option<i32> {
    crate::game::creature::ai::units::resolve_unit(world, guid).map(|u| u.level as i32)
}

fn object_position(guid: ObjectGuid, world: &World) -> Option<(u32, u32, oxcore_shared::protocol::Position)> {
    if guid.is_game_object() {
        return world
            .managers
            .gameobject_mgr
            .with_gameobject(guid, |go| (go.map_id, go.instance_id, go.position));
    }
    crate::game::creature::ai::units::resolve_unit(world, guid)
        .map(|u| (u.map_id, u.instance_id, u.position))
}

fn object_map(guid: ObjectGuid, world: &World) -> Option<(u32, u32)> {
    object_position(guid, world).map(|(map, instance, _)| (map, instance))
}

fn db_guid(guid: ObjectGuid, world: &World) -> Option<u32> {
    if guid.is_game_object() {
        world
            .managers
            .gameobject_mgr
            .with_gameobject(guid, |go| go.spawn_id)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.spawn_id)
    }
}

fn nearby_creature(
    around: ObjectGuid,
    entry: u32,
    radius: f32,
    alive_only: bool,
    exclude_self: bool,
    world: &World,
) -> bool {
    let Some((map_id, instance_id, position)) = object_position(around, world) else {
        return false;
    };
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return false;
    };
    map.get_objects_in_range(position, radius)
        .into_iter()
        .filter(|g| g.is_creature() && g.entry() == entry && !(exclude_self && *g == around))
        .any(|g| {
            world
                .managers
                .creature_mgr
                .with_creature(g, |c| {
                    (!alive_only || c.is_alive())
                        && crate::game::creature::ai::units::distance(&c.position, &position)
                            <= radius
                })
                .unwrap_or(false)
        })
}

fn nearby_gameobject(around: ObjectGuid, entry: u32, radius: f32, world: &World) -> bool {
    let Some((map_id, instance_id, position)) = object_position(around, world) else {
        return false;
    };
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return false;
    };
    map.get_objects_in_range(position, radius)
        .into_iter()
        .filter(|g| g.is_game_object() && g.entry() == entry)
        .any(|g| {
            world
                .managers
                .gameobject_mgr
                .with_gameobject(g, |go| {
                    go.in_world
                        && crate::game::creature::ai::units::distance(&go.position, &position)
                            <= radius
                })
                .unwrap_or(false)
        })
}

/// 0: any player, 1: a player hostile to the searcher, 2: a friendly player.
fn nearby_player(around: ObjectGuid, mode: i32, radius: f32, world: &World) -> bool {
    use crate::game::creature::ai::units;
    use oxcore_shared::game::reputation::ReputationRank;
    let Some(me) = units::resolve_unit(world, around) else {
        return false;
    };
    let Some(map) = world.managers.map_mgr.get_map(me.map_id, me.instance_id) else {
        return false;
    };
    map.get_players_in_range(me.position, radius)
        .into_iter()
        .filter(|g| *g != around)
        .filter_map(|g| units::resolve_unit(world, g))
        .filter(|p| p.alive && units::distance(&p.position, &me.position) <= radius)
        .any(|p| {
            let reaction = units::reaction_to(world, me.faction, &p);
            match mode {
                0 => true,
                1 => reaction <= ReputationRank::Hostile,
                2 => reaction >= ReputationRank::Friendly,
                _ => false,
            }
        })
}
