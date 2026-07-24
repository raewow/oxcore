use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::{MySqlPool, Row};
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

    pub async fn load(&self, world_db: &MySqlPool) -> Result<()> {
        let rows = sqlx::query(
            "SELECT condition_entry, type, value1, value2, value3, value4, flags FROM conditions",
        )
        .fetch_all(world_db)
        .await
        .context("Failed to load conditions")?;

        self.entries.clear();
        for row in rows {
            self.entries.insert(
                row.try_get::<u64, _>("condition_entry")? as u32,
                ConditionEntry {
                    kind: row.try_get("type")?,
                    values: [
                        row.try_get("value1")?,
                        row.try_get("value2")?,
                        row.try_get("value3")?,
                        row.try_get("value4")?,
                    ],
                    flags: row.try_get::<u8, _>("flags")?,
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
