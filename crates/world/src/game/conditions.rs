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
            2 => carried_count(context.target, entry.values[0] as u32, false, world)
                >= entry.values[1].max(1) as u32,
            3 => context.target.is_player()
                && world
                    .systems
                    .inventory
                    .cache()
                    .has_equipped_entry(context.target, entry.values[0] as u32),
            5 => reputation_rank(context.target, entry.values[0] as u32, world)
                .is_some_and(|rank| rank >= entry.values[1]),
            7 => skill_value(context.target, entry.values[0] as u16, world)
                .is_some_and(|value| i32::from(value) >= entry.values[1]),
            14 => world
                .managers
                .player_mgr
                .with_player(context.target, |p| {
                    let race_mask = 1i32 << (i32::from(p.race) - 1).max(0);
                    let class_mask = 1i32 << (i32::from(p.class) - 1).max(0);
                    (entry.values[0] == 0 || race_mask & entry.values[0] != 0)
                        && (entry.values[1] == 0 || class_mask & entry.values[1] != 0)
                })
                .unwrap_or(false),
            17 => world
                .managers
                .player_mgr
                .with_player(context.target, |p| {
                    let known = p.spells.knows_spell(entry.values[0] as u32);
                    match entry.values[1] {
                        0 => known,
                        1 => !known,
                        _ => false,
                    }
                })
                .unwrap_or(false),
            19 => context.target.is_player()
                && world
                    .systems
                    .quest
                    .manager
                    .get_quest_template(entry.values[0] as u32)
                    .is_some_and(|quest| {
                        world.systems.quest.can_take_quest(context.target, &quest, world)
                    }),
            22 => world
                .managers
                .player_mgr
                .with_player(context.target, |p| {
                    let quest_id = entry.values[0] as u32;
                    !p.rewarded_quests.contains(&quest_id)
                        && !p.active_quests.iter().any(|q| q.quest_id == quest_id)
                })
                .unwrap_or(false),
            23 => carried_count(context.target, entry.values[0] as u32, true, world)
                >= entry.values[1].max(1) as u32,
            27 => unit_gender(context.target, world).is_some_and(|g| i32::from(g) == entry.values[0]),
            29 => context.target.is_player()
                && match skill_value(context.target, entry.values[0] as u16, world) {
                    None => entry.values[1] == 1,
                    Some(value) => entry.values[1] != 1 && i32::from(value) < entry.values[1],
                },
            30 => reputation_rank(context.target, entry.values[0] as u32, world)
                .is_some_and(|rank| rank <= entry.values[1]),
            45 => context.target.is_player() && world.systems.group.is_in_group(context.target),
            50 => match spawned_gameobject(entry.values[0] as u32, context, world) {
                Some(gameobject) => self.evaluate(
                    entry.values[1] as u32,
                    ConditionContext {
                        target: gameobject,
                        source: context.source,
                    },
                    world,
                    visiting,
                ),
                None => false,
            },
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

fn carried_count(guid: ObjectGuid, entry: u32, include_bank: bool, world: &World) -> u32 {
    if !guid.is_player() {
        return 0;
    }
    world
        .systems
        .inventory
        .cache()
        .count_carried_by_entry(guid, entry, include_bank)
}

/// The player's rank with a faction, falling back to the race's starting standing
/// for factions the player has no entry for.
fn reputation_rank(guid: ObjectGuid, faction_id: u32, world: &World) -> Option<i32> {
    use oxcore_shared::game::reputation::ReputationRank;
    if !guid.is_player() {
        return None;
    }
    if let Some(rank) = world.systems.reputation.get_rank(guid, faction_id, world) {
        return Some(rank as i32);
    }
    let (race, class) = world
        .managers
        .player_mgr
        .with_player(guid, |p| (p.race, p.class))?;
    let base = world
        .dbc
        .read()
        .get_faction(faction_id)
        .map(|f| f.get_base_reputation(race, class))?;
    Some(ReputationRank::from_value(base) as i32)
}

fn skill_value(guid: ObjectGuid, skill_id: u16, world: &World) -> Option<u16> {
    if !guid.is_player() {
        return None;
    }
    world.systems.skills.get_skill_value(guid, skill_id, world)
}

fn unit_gender(guid: ObjectGuid, world: &World) -> Option<u8> {
    if guid.is_player() {
        return world.managers.player_mgr.with_player(guid, |p| p.gender);
    }
    let display_id = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| c.display_id)?;
    world
        .managers
        .creature_mgr
        .get_model_info(display_id)
        .map(|info| info.gender)
}

/// A game object by database guid in the same map copy as the source (or target).
fn spawned_gameobject(spawn_id: u32, context: ConditionContext, world: &World) -> Option<ObjectGuid> {
    let (map_id, instance_id) =
        object_map(context.source, world).or_else(|| object_map(context.target, world))?;
    world
        .managers
        .gameobject_mgr
        .iter_gameobjects()
        .find(|e| {
            let go = e.value();
            go.spawn_id == spawn_id && go.map_id == map_id && go.instance_id == instance_id
        })
        .map(|e| *e.key())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::inventory::PlayerInventoryData;
    use crate::game::items::Item;
    use crate::game::player::player::Player;
    use crate::Config;
    use oxcore_db::database::Databases;
    use parking_lot::RwLock;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_world() -> World {
        let pool = || {
            PgPoolOptions::new()
                .connect_lazy("postgres://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        World::new(
            Arc::new(Databases {
                world: pool(),
                character: pool(),
                auth: pool(),
                logs: oxcore_db::database::lazy_logs_pool(),
            }),
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    /// A level-10 player of the given race and class, with an empty inventory.
    fn add_player(world: &World, race: u8, class: u8) -> ObjectGuid {
        let guid = ObjectGuid::new_player(1);
        let player = Player::new(guid, "Tester".to_string(), 0, 0, 1, 10, race, class, 0);
        world.managers.player_mgr.add_player(player, 1);
        world
            .systems
            .inventory
            .cache()
            .add_player_inventory(PlayerInventoryData::new(guid));
        guid
    }

    fn add_item(world: &World, player: ObjectGuid, counter: u32, entry: u32, bag: u8, slot: u8) {
        let item = Item::new(
            ObjectGuid::new_item(counter),
            entry,
            1,
            player,
            slot,
            bag,
            0,
            0,
            0,
            Vec::new(),
            0,
            None,
            None,
            0,
            [0; 5],
        );
        world
            .systems
            .inventory
            .cache()
            .add_item(player, Arc::new(RwLock::new(item)));
    }

    fn check(world: &World, kind: i8, values: [i32; 4], player: ObjectGuid) -> bool {
        world.managers.condition_mgr.add_for_test(
            900,
            ConditionEntry {
                kind,
                values,
                flags: 0,
            },
        );
        world.managers.condition_mgr.is_satisfied(
            900,
            ConditionContext {
                target: player,
                source: player,
            },
            world,
        )
    }

    #[tokio::test]
    async fn race_class_masks_match_the_player() {
        let world = test_world();
        let player = add_player(&world, 2, 3); // orc hunter
        assert!(check(&world, 14, [0b10, 0, 0, 0], player));
        assert!(check(&world, 14, [0, 0b100, 0, 0], player));
        assert!(!check(&world, 14, [0b1, 0, 0, 0], player));
        assert!(!check(&world, 14, [0b10, 0b1, 0, 0], player));
    }

    #[tokio::test]
    async fn quest_none_fails_once_taken_or_rewarded() {
        let world = test_world();
        let player = add_player(&world, 1, 1);
        assert!(check(&world, 22, [77, 0, 0, 0], player));
        world.managers.player_mgr.with_player_mut(player, |p| {
            p.rewarded_quests.insert(77);
        });
        assert!(!check(&world, 22, [77, 0, 0, 0], player));
    }

    #[tokio::test]
    async fn spell_condition_checks_known_and_unknown() {
        let world = test_world();
        let player = add_player(&world, 1, 1);
        assert!(check(&world, 17, [133, 1, 0, 0], player));
        world.managers.player_mgr.with_player_mut(player, |p| {
            p.spells.learn_spell(133);
        });
        assert!(check(&world, 17, [133, 0, 0, 0], player));
        assert!(!check(&world, 17, [133, 1, 0, 0], player));
    }

    #[tokio::test]
    async fn banked_items_count_only_with_bank() {
        use oxcore_shared::game::inventory::{BANK_SLOT_ITEM_START, BUYBACK_SLOT_START};
        let world = test_world();
        let player = add_player(&world, 1, 1);
        add_item(&world, player, 1, 6948, INVENTORY_BAG, BANK_SLOT_ITEM_START);
        add_item(&world, player, 2, 6948, INVENTORY_BAG, BUYBACK_SLOT_START);
        assert!(!check(&world, 2, [6948, 1, 0, 0], player));
        assert!(check(&world, 23, [6948, 1, 0, 0], player));
        assert!(!check(&world, 23, [6948, 2, 0, 0], player));

        add_item(&world, player, 3, 6948, INVENTORY_BAG, 23);
        assert!(check(&world, 2, [6948, 1, 0, 0], player));
    }

    const INVENTORY_BAG: u8 = 255;
}
