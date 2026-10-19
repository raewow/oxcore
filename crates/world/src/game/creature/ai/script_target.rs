//! Script target types: how spell lists, EventAI and DB scripts name the
//! object an action applies to (the victim, a random threat-list target, the
//! most injured friend, the nearest creature with an entry, ...).

use super::units::{self, UnitInfo};
use crate::dbc::structures::SpellEntry;
use crate::game::combat::melee_range;
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::seq::SliceRandom;

pub const TARGET_T_PROVIDED_TARGET: u8 = 0;
pub const TARGET_T_HOSTILE: u8 = 1;
pub const TARGET_T_HOSTILE_SECOND_AGGRO: u8 = 2;
pub const TARGET_T_HOSTILE_LAST_AGGRO: u8 = 3;
pub const TARGET_T_HOSTILE_RANDOM: u8 = 4;
pub const TARGET_T_HOSTILE_RANDOM_NOT_TOP: u8 = 5;
pub const TARGET_T_HOSTILE_NEAREST: u8 = 6;
pub const TARGET_T_HOSTILE_FARTHEST: u8 = 7;
pub const TARGET_T_OWNER_OR_SELF: u8 = 8;
pub const TARGET_T_OWNER: u8 = 9;
pub const TARGET_T_NEAREST_CREATURE_WITH_ENTRY: u8 = 10;
pub const TARGET_T_CREATURE_WITH_GUID: u8 = 11;
pub const TARGET_T_CREATURE_FROM_INSTANCE_DATA: u8 = 12;
pub const TARGET_T_NEAREST_GAMEOBJECT_WITH_ENTRY: u8 = 13;
pub const TARGET_T_GAMEOBJECT_WITH_GUID: u8 = 14;
pub const TARGET_T_GAMEOBJECT_FROM_INSTANCE_DATA: u8 = 15;
pub const TARGET_T_FRIENDLY: u8 = 16;
pub const TARGET_T_FRIENDLY_INJURED: u8 = 17;
pub const TARGET_T_FRIENDLY_INJURED_EXCEPT: u8 = 18;
pub const TARGET_T_FRIENDLY_MISSING_BUFF: u8 = 19;
pub const TARGET_T_FRIENDLY_MISSING_BUFF_EXCEPT: u8 = 20;
pub const TARGET_T_FRIENDLY_CC: u8 = 21;
pub const TARGET_T_NEAREST_PLAYER: u8 = 25;
pub const TARGET_T_NEAREST_HOSTILE_PLAYER: u8 = 26;
pub const TARGET_T_NEAREST_FRIENDLY_PLAYER: u8 = 27;
pub const TARGET_T_RANDOM_CREATURE_WITH_ENTRY: u8 = 28;
pub const TARGET_T_RANDOM_GAMEOBJECT_WITH_ENTRY: u8 = 29;

pub const SELECT_FLAG_IN_LOS: u32 = 0x001;
pub const SELECT_FLAG_PLAYER: u32 = 0x002;
pub const SELECT_FLAG_POWER_MANA: u32 = 0x004;
pub const SELECT_FLAG_POWER_RAGE: u32 = 0x008;
pub const SELECT_FLAG_POWER_ENERGY: u32 = 0x010;
pub const SELECT_FLAG_IN_MELEE_RANGE: u32 = 0x040;
pub const SELECT_FLAG_NOT_IN_MELEE_RANGE: u32 = 0x080;
pub const SELECT_FLAG_NO_TOTEM: u32 = 0x100;
pub const SELECT_FLAG_PLAYER_NOT_GM: u32 = 0x200;
pub const SELECT_FLAG_PET: u32 = 0x400;
pub const SELECT_FLAG_NOT_PLAYER: u32 = 0x800;
pub const SELECT_FLAG_POWER_NOT_MANA: u32 = 0x1000;

/// Default search radius for friendly target types without a spell range.
const DEFAULT_FRIENDLY_RADIUS: f32 = 30.0;
/// Units under these flags count as crowd controlled.
const CC_FLAGS: u32 = crate::game::common::unit_flags::STUNNED
    | crate::game::common::unit_flags::CONFUSED
    | crate::game::common::unit_flags::FLEEING;

/// The source of a script target lookup.
#[derive(Debug, Clone)]
pub struct Source {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub faction: u32,
    pub combat_reach: f32,
    pub victim: Option<ObjectGuid>,
    pub owner: Option<ObjectGuid>,
    /// Threat list, best threat first.
    pub threat: Vec<ObjectGuid>,
}

impl Source {
    pub fn of_creature(world: &World, guid: ObjectGuid) -> Option<Self> {
        world.managers.creature_mgr.with_creature(guid, |c| Source {
            guid,
            map_id: c.map_id,
            instance_id: c.instance_id,
            position: c.position,
            faction: c.faction,
            combat_reach: c.combat_reach,
            victim: c.combat.attacking,
            owner: c.owner_guid.or(c.charm_guid),
            threat: c
                .threat_manager
                .get_threat_list()
                .into_iter()
                .map(|(target, _)| target)
                .collect(),
        })
    }
}

/// Order of a threat-list selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackingTarget {
    TopAggro,
    BottomAggro,
    Random,
    Nearest,
    Farthest,
}

/// Maximum range of a spell, 0 when it has none.
pub fn spell_max_range(world: &World, spell: &SpellEntry) -> f32 {
    if spell.range_index == 0 {
        return 0.0;
    }
    world
        .dbc
        .read()
        .get_spell_range(spell.range_index)
        .map(|r| r.range_max)
        .unwrap_or(0.0)
}

fn line_of_sight(world: &World, source: &Source, target: &Position) -> bool {
    world
        .managers
        .vmap_mgr
        .is_in_line_of_sight(source.map_id, source.instance_id, source.position, *target)
}

/// Whether a threat-list target satisfies the selection flags and, when a spell
/// is given, is within its range.
fn meets_requirement(
    world: &World,
    source: &Source,
    target: &UnitInfo,
    spell: Option<&SpellEntry>,
    flags: u32,
) -> bool {
    use crate::game::common::unit_flags as uf;
    if flags & SELECT_FLAG_PLAYER != 0 && !target.is_player {
        return false;
    }
    if flags & SELECT_FLAG_POWER_MANA != 0 && !target.has_mana {
        return false;
    }
    if flags & SELECT_FLAG_POWER_NOT_MANA != 0 && target.has_mana {
        return false;
    }
    let in_melee = melee_range::is_within_melee_range(
        &source.position,
        source.combat_reach,
        &target.position,
        target.combat_reach,
        false,
    );
    if flags & SELECT_FLAG_IN_MELEE_RANGE != 0 && !in_melee {
        return false;
    }
    if flags & SELECT_FLAG_NOT_IN_MELEE_RANGE != 0 && in_melee {
        return false;
    }
    if flags & SELECT_FLAG_IN_LOS != 0 && !line_of_sight(world, source, &target.position) {
        return false;
    }
    if flags & SELECT_FLAG_PLAYER_NOT_GM != 0 && (!target.is_player || target.game_master) {
        return false;
    }
    if flags & SELECT_FLAG_PET != 0 && (target.is_player || !target.is_player_controlled()) {
        return false;
    }
    if flags & SELECT_FLAG_NOT_PLAYER != 0 && target.is_player {
        return false;
    }
    if !target.alive
        || target.unit_flags
            & (uf::SPAWNING | uf::NOT_ATTACKABLE_1 | uf::ON_TAXI | uf::NOT_SELECTABLE)
            != 0
        || units::reaction_to(world, source.faction, target) >= ReputationRank::Friendly
    {
        return false;
    }
    match spell {
        Some(spell) => {
            let range = spell_max_range(world, spell);
            range <= 0.0 || units::distance(&source.position, &target.position) <= range
        }
        None => true,
    }
}

/// Pick a target from the threat list, skipping the first `position` entries.
pub fn select_attacking_target(
    world: &World,
    source: &Source,
    order: AttackingTarget,
    position: usize,
    spell: Option<&SpellEntry>,
    flags: u32,
) -> Option<ObjectGuid> {
    if position >= source.threat.len() {
        return None;
    }
    let candidates: Vec<UnitInfo> = source.threat[position..]
        .iter()
        .filter_map(|g| units::resolve_unit(world, *g))
        .collect();
    let suitable = |t: &&UnitInfo| meets_requirement(world, source, t, spell, flags);
    match order {
        AttackingTarget::TopAggro => candidates.iter().find(suitable).map(|t| t.guid),
        AttackingTarget::BottomAggro => candidates.iter().rev().find(suitable).map(|t| t.guid),
        AttackingTarget::Random => {
            let pool: Vec<_> = candidates.iter().filter(suitable).collect();
            pool.choose(&mut rand::thread_rng()).map(|t| t.guid)
        }
        AttackingTarget::Nearest => candidates
            .iter()
            .filter(suitable)
            .min_by(|a, b| {
                units::distance(&source.position, &a.position)
                    .total_cmp(&units::distance(&source.position, &b.position))
            })
            .map(|t| t.guid),
        AttackingTarget::Farthest => candidates
            .iter()
            .filter(suitable)
            .max_by(|a, b| {
                units::distance(&source.position, &a.position)
                    .total_cmp(&units::distance(&source.position, &b.position))
            })
            .map(|t| t.guid),
    }
}

/// Units around the source in its copy, within `radius`.
fn units_around(world: &World, source: &Source, radius: f32) -> Vec<UnitInfo> {
    let Some(map) = world
        .managers
        .map_mgr
        .get_map(source.map_id, source.instance_id)
    else {
        return Vec::new();
    };
    map.get_objects_in_range(source.position, radius)
        .into_iter()
        .filter(|g| g.is_player() || g.is_creature())
        .filter_map(|g| units::resolve_unit(world, g))
        .filter(|u| {
            u.map_id == source.map_id
                && u.instance_id == source.instance_id
                && units::distance(&u.position, &source.position) <= radius
        })
        .collect()
}

fn friendly_units(world: &World, source: &Source, radius: f32) -> Vec<UnitInfo> {
    units_around(world, source, radius)
        .into_iter()
        .filter(|u| u.alive && units::reaction_to(world, source.faction, u) >= ReputationRank::Friendly)
        .collect()
}

fn friendly_radius(world: &World, param: u32, spell: Option<&SpellEntry>) -> f32 {
    if param > 0 {
        param as f32
    } else {
        spell
            .map(|s| spell_max_range(world, s))
            .filter(|r| *r > 0.0)
            .unwrap_or(DEFAULT_FRIENDLY_RADIUS)
    }
}

fn has_aura(world: &World, unit: ObjectGuid, spell_id: u32) -> bool {
    if unit.is_player() {
        world
            .managers
            .player_mgr
            .with_player(unit, |p| p.auras.container.has_aura(spell_id))
            .unwrap_or(false)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(unit, |c| c.auras.has_aura(spell_id))
            .unwrap_or(false)
    }
}

fn nearest_creature_with_entry(
    world: &World,
    source: &Source,
    entry: u32,
    radius: f32,
    random: bool,
) -> Option<ObjectGuid> {
    let found: Vec<UnitInfo> = units_around(world, source, radius)
        .into_iter()
        .filter(|u| !u.is_player && u.alive && u.guid.entry() == entry && u.guid != source.guid)
        .collect();
    if random {
        return found.choose(&mut rand::thread_rng()).map(|u| u.guid);
    }
    found
        .into_iter()
        .min_by(|a, b| {
            units::distance(&source.position, &a.position)
                .total_cmp(&units::distance(&source.position, &b.position))
        })
        .map(|u| u.guid)
}

fn gameobject_with_entry(
    world: &World,
    source: &Source,
    entry: u32,
    radius: f32,
    random: bool,
) -> Option<ObjectGuid> {
    let map = world
        .managers
        .map_mgr
        .get_map(source.map_id, source.instance_id)?;
    let found: Vec<(ObjectGuid, f32)> = map
        .get_objects_in_range(source.position, radius)
        .into_iter()
        .filter(|g| g.is_game_object() && g.entry() == entry)
        .filter_map(|g| {
            world
                .managers
                .gameobject_mgr
                .get_position(g)
                .map(|p| (g, units::distance(&p, &source.position)))
        })
        .filter(|(_, d)| *d <= radius)
        .collect();
    if random {
        return found.choose(&mut rand::thread_rng()).map(|(g, _)| *g);
    }
    found
        .into_iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(g, _)| g)
}

fn creature_by_spawn_id(world: &World, source: &Source, spawn_id: u32) -> Option<ObjectGuid> {
    world
        .managers
        .creature_mgr
        .iter_creatures()
        .find(|e| {
            let c = e.value();
            c.spawn_id == spawn_id && c.map_id == source.map_id && c.instance_id == source.instance_id
        })
        .map(|e| *e.key())
}

fn gameobject_by_spawn_id(world: &World, source: &Source, spawn_id: u32) -> Option<ObjectGuid> {
    world
        .managers
        .gameobject_mgr
        .iter_gameobjects()
        .find(|e| {
            let go = e.value();
            go.spawn_id == spawn_id && go.map_id == source.map_id && go.instance_id == source.instance_id
        })
        .map(|e| *e.key())
}

fn nearest_player(
    world: &World,
    source: &Source,
    radius: f32,
    filter: impl Fn(&UnitInfo) -> bool,
) -> Option<ObjectGuid> {
    units_around(world, source, radius)
        .into_iter()
        .filter(|u| u.is_player && u.alive && filter(u))
        .min_by(|a, b| {
            units::distance(&source.position, &a.position)
                .total_cmp(&units::distance(&source.position, &b.position))
        })
        .map(|u| u.guid)
}

/// Resolve a script target type to an object.
pub fn get_target_by_type(
    world: &World,
    source: &Source,
    provided: Option<ObjectGuid>,
    target_type: u8,
    param1: u32,
    param2: u32,
    spell: Option<&SpellEntry>,
) -> Option<ObjectGuid> {
    let select_flags = if param1 != 0 { param1 } else { SELECT_FLAG_NO_TOTEM };
    let attacking = |order, position| {
        select_attacking_target(world, source, order, position, spell, select_flags)
    };
    match target_type {
        TARGET_T_PROVIDED_TARGET => provided,
        TARGET_T_HOSTILE => source.victim,
        TARGET_T_HOSTILE_SECOND_AGGRO => attacking(AttackingTarget::TopAggro, 1),
        TARGET_T_HOSTILE_LAST_AGGRO => attacking(AttackingTarget::BottomAggro, 0),
        TARGET_T_HOSTILE_RANDOM => attacking(AttackingTarget::Random, 0),
        TARGET_T_HOSTILE_RANDOM_NOT_TOP => attacking(AttackingTarget::Random, 1),
        TARGET_T_HOSTILE_NEAREST => attacking(AttackingTarget::Nearest, 0),
        TARGET_T_HOSTILE_FARTHEST => attacking(AttackingTarget::Farthest, 0),
        TARGET_T_OWNER_OR_SELF => Some(source.owner.unwrap_or(source.guid)),
        TARGET_T_OWNER => source.owner,
        TARGET_T_NEAREST_CREATURE_WITH_ENTRY => {
            nearest_creature_with_entry(world, source, param1, param2 as f32, false)
        }
        TARGET_T_RANDOM_CREATURE_WITH_ENTRY => {
            nearest_creature_with_entry(world, source, param1, param2 as f32, true)
        }
        TARGET_T_CREATURE_WITH_GUID => creature_by_spawn_id(world, source, param1),
        TARGET_T_CREATURE_FROM_INSTANCE_DATA | TARGET_T_GAMEOBJECT_FROM_INSTANCE_DATA => world
            .managers
            .lua_mgr
            .get_instance_state(source.map_id, source.instance_id)
            .get_guid(param1),
        TARGET_T_NEAREST_GAMEOBJECT_WITH_ENTRY => {
            gameobject_with_entry(world, source, param1, param2 as f32, false)
        }
        TARGET_T_RANDOM_GAMEOBJECT_WITH_ENTRY => {
            gameobject_with_entry(world, source, param1, param2 as f32, true)
        }
        TARGET_T_GAMEOBJECT_WITH_GUID => gameobject_by_spawn_id(world, source, param1),
        TARGET_T_FRIENDLY => {
            let radius = friendly_radius(world, param1, spell);
            let exclude = if param2 != 0 { provided } else { None };
            let pool: Vec<_> = friendly_units(world, source, radius)
                .into_iter()
                .filter(|u| Some(u.guid) != exclude)
                .collect();
            pool.choose(&mut rand::thread_rng()).map(|u| u.guid)
        }
        TARGET_T_FRIENDLY_INJURED | TARGET_T_FRIENDLY_INJURED_EXCEPT => {
            let radius = friendly_radius(world, param1, spell);
            let threshold = if param2 != 0 { param2 } else { 50 } as f32 / 100.0;
            let exclude = (target_type == TARGET_T_FRIENDLY_INJURED_EXCEPT)
                .then_some(provided)
                .flatten();
            friendly_units(world, source, radius)
                .into_iter()
                .filter(|u| Some(u.guid) != exclude && u.health_pct < threshold)
                .min_by(|a, b| a.health_pct.total_cmp(&b.health_pct))
                .map(|u| u.guid)
        }
        TARGET_T_FRIENDLY_MISSING_BUFF | TARGET_T_FRIENDLY_MISSING_BUFF_EXCEPT => {
            let radius = friendly_radius(world, param1, spell);
            let exclude = (target_type == TARGET_T_FRIENDLY_MISSING_BUFF_EXCEPT)
                .then_some(provided)
                .flatten();
            friendly_units(world, source, radius)
                .into_iter()
                .filter(|u| Some(u.guid) != exclude && !has_aura(world, u.guid, param2))
                .map(|u| u.guid)
                .next()
        }
        TARGET_T_FRIENDLY_CC => {
            let radius = friendly_radius(world, param1, spell);
            friendly_units(world, source, radius)
                .into_iter()
                .find(|u| u.unit_flags & CC_FLAGS != 0)
                .map(|u| u.guid)
        }
        TARGET_T_NEAREST_PLAYER => {
            nearest_player(world, source, friendly_radius(world, param1, spell), |_| true)
        }
        TARGET_T_NEAREST_HOSTILE_PLAYER => {
            nearest_player(world, source, friendly_radius(world, param1, spell), |u| {
                units::reaction_to(world, source.faction, u) <= ReputationRank::Hostile
            })
        }
        TARGET_T_NEAREST_FRIENDLY_PLAYER => {
            nearest_player(world, source, friendly_radius(world, param1, spell), |u| {
                units::reaction_to(world, source.faction, u) >= ReputationRank::Friendly
            })
        }
        _ => None,
    }
}
