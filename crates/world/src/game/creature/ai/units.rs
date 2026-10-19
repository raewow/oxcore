//! What creature AI needs to know about another unit, read in one short lock.

use super::types::AIState;
use crate::game::combat::melee_range::{self, DEFAULT_COMBAT_REACH};
use crate::game::common::unit_flags;
use crate::game::faction;
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::{ObjectGuid, Position};

pub const SPELL_AURA_MOD_TAUNT: u32 = 11;
pub const SPELL_AURA_MOD_STEALTH: u32 = 16;
pub const SPELL_AURA_MOD_STEALTH_DETECT: u32 = 17;
pub const SPELL_AURA_MOD_INVISIBILITY: u32 = 18;
pub const SPELL_AURA_MOD_INVISIBILITY_DETECTION: u32 = 19;
pub const SPELL_AURA_FEIGN_DEATH: u32 = 66;
pub const SPELL_AURA_MOD_DETECT_RANGE: u32 = 91;
pub const SPELL_AURA_MOD_DETECTED_RANGE: u32 = 152;
pub const SPELL_AURA_MOD_STEALTH_LEVEL: u32 = 154;

/// Snapshot of a player or creature as seen by creature AI.
#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub guid: ObjectGuid,
    pub is_player: bool,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub alive: bool,
    pub level: u8,
    pub faction: u32,
    pub unit_flags: u32,
    pub combat_reach: f32,
    pub health_pct: f32,
    pub has_mana: bool,
    pub in_combat: bool,
    /// The unit this one is attacking.
    pub victim: Option<ObjectGuid>,
    /// Player that controls this unit (the unit itself for players, the owner for pets).
    pub controlling_player: Option<ObjectGuid>,
    pub game_master: bool,
    pub on_taxi: bool,
    /// A creature walking home after a fight.
    pub evading: bool,
    pub feigning_death: bool,
    /// Stealth skill when stealthed.
    pub stealth: Option<i32>,
    pub invisible: bool,
    /// Detected-range auras: positive values make the unit noticed from further away.
    pub detected_range_mod: i32,
}

impl UnitInfo {
    pub fn is_player_controlled(&self) -> bool {
        self.controlling_player.is_some()
    }
}

/// Look up a unit by GUID, player or creature.
pub fn resolve_unit(world: &World, guid: ObjectGuid) -> Option<UnitInfo> {
    if guid.is_player() {
        world.managers.player_mgr.with_player(guid, |p| {
            let auras = &p.auras.container;
            let stealthed = auras.has_aura_type(SPELL_AURA_MOD_STEALTH);
            UnitInfo {
                guid,
                is_player: true,
                map_id: p.map_id,
                instance_id: p.instance_id,
                position: p.movement.position,
                alive: p.is_alive(),
                level: p.level,
                faction: p.faction_template(),
                unit_flags: p.unit_flags,
                combat_reach: DEFAULT_COMBAT_REACH,
                health_pct: if p.stats.max_health > 0 {
                    p.stats.health as f32 / p.stats.max_health as f32
                } else {
                    0.0
                },
                has_mana: p.stats.max_mana > 0,
                in_combat: p.combat.in_combat,
                victim: p.combat.attack_target,
                controlling_player: Some(guid),
                game_master: p.gm_mode,
                on_taxi: p.unit_flags & unit_flags::ON_TAXI != 0,
                evading: false,
                feigning_death: auras.has_aura_type(SPELL_AURA_FEIGN_DEATH),
                stealth: stealthed.then(|| {
                    auras.get_total_aura_modifier(SPELL_AURA_MOD_STEALTH)
                        + auras.get_total_aura_modifier(SPELL_AURA_MOD_STEALTH_LEVEL)
                }),
                invisible: auras.has_aura_type(SPELL_AURA_MOD_INVISIBILITY),
                detected_range_mod: auras.get_total_aura_modifier(SPELL_AURA_MOD_DETECTED_RANGE),
            }
        })
    } else {
        world.managers.creature_mgr.with_creature(guid, |c| {
            let stealthed = c.auras.has_aura_type(SPELL_AURA_MOD_STEALTH);
            UnitInfo {
                guid,
                is_player: false,
                map_id: c.map_id,
                instance_id: c.instance_id,
                position: c.position,
                alive: c.is_alive(),
                level: c.level,
                faction: c.faction,
                unit_flags: c.unit_flags,
                combat_reach: c.combat_reach,
                health_pct: if c.max_health > 0 {
                    c.current_health as f32 / c.max_health as f32
                } else {
                    0.0
                },
                has_mana: c.max_mana > 0,
                in_combat: c.combat.in_combat,
                victim: c.combat.attacking,
                controlling_player: c
                    .owner_guid
                    .filter(|o| o.is_player())
                    .or(c.charm_guid.filter(|o| o.is_player())),
                game_master: false,
                on_taxi: false,
                evading: matches!(c.ai_state, AIState::Evading | AIState::Returning),
                feigning_death: c.auras.has_aura_type(SPELL_AURA_FEIGN_DEATH),
                stealth: stealthed.then(|| {
                    c.level as i32 * 5 + c.auras.get_total_aura_modifier(SPELL_AURA_MOD_STEALTH_LEVEL)
                }),
                invisible: c.auras.has_aura_type(SPELL_AURA_MOD_INVISIBILITY),
                detected_range_mod: c.auras.get_total_aura_modifier(SPELL_AURA_MOD_DETECTED_RANGE),
            }
        })
    }
}

/// How a creature of faction template `observer_faction` feels about `target`.
pub fn reaction_to(world: &World, observer_faction: u32, target: &UnitInfo) -> ReputationRank {
    if target.is_player {
        // Player first, then the DBC: the same order the reputation system locks in.
        return world
            .managers
            .player_mgr
            .with_player(target.guid, |p| {
                let dbc = world.dbc.read();
                faction::creature_reaction_to_player(&dbc, observer_faction, p)
            })
            .unwrap_or(ReputationRank::Neutral);
    }
    faction::template_reaction(&world.dbc.read(), observer_faction, target.faction)
}

/// How `target` feels about a creature of faction template `creature_faction`.
pub fn reaction_from(world: &World, target: &UnitInfo, creature_faction: u32) -> ReputationRank {
    if target.is_player {
        return world
            .managers
            .player_mgr
            .with_player(target.guid, |p| {
                let dbc = world.dbc.read();
                faction::player_reaction_to_creature(&dbc, p, creature_faction)
            })
            .unwrap_or(ReputationRank::Neutral);
    }
    faction::template_reaction(&world.dbc.read(), target.faction, creature_faction)
}

/// The attacking creature as far as attackability checks are concerned.
#[derive(Debug, Clone, Copy)]
pub struct Attacker {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    pub faction: u32,
    pub unit_flags: u32,
    pub player_controlled: bool,
}

/// Whether a non-player attacker may hit `target` at all, ignoring factions:
/// alive, in the same copy, selectable, not a game master, not on a flight path,
/// not walking home after a fight, and not shielded by the immunity flags.
pub fn is_targetable_by(attacker: &Attacker, target: &UnitInfo) -> bool {
    if attacker.guid == target.guid || !target.alive {
        return false;
    }
    if attacker.map_id != target.map_id || attacker.instance_id != target.instance_id {
        return false;
    }
    if target.unit_flags
        & (unit_flags::SPAWNING
            | unit_flags::ON_TAXI
            | unit_flags::NOT_ATTACKABLE_1
            | unit_flags::NOT_SELECTABLE)
        != 0
    {
        return false;
    }
    if target.game_master || target.on_taxi || target.evading {
        return false;
    }
    if attacker.player_controlled {
        if target.unit_flags & unit_flags::IMMUNE_TO_PLAYER != 0 {
            return false;
        }
    } else {
        if target.unit_flags & unit_flags::IMMUNE_TO_NPC != 0 {
            return false;
        }
        if target.feigning_death {
            return false;
        }
    }
    let target_flag = if target.is_player_controlled() {
        unit_flags::IMMUNE_TO_PLAYER
    } else {
        unit_flags::IMMUNE_TO_NPC
    };
    attacker.unit_flags & target_flag == 0
}

/// Targetable and not friendly: creature against creature needs one side hostile,
/// against a player-controlled unit neither side may be friendly.
pub fn is_valid_attack_target(world: &World, attacker: &Attacker, target: &UnitInfo) -> bool {
    if !is_targetable_by(attacker, target) {
        return false;
    }
    let ours = reaction_to(world, attacker.faction, target);
    if !attacker.player_controlled && !target.is_player_controlled() {
        return ours <= ReputationRank::Hostile
            || reaction_from(world, target, attacker.faction) <= ReputationRank::Hostile;
    }
    ours <= ReputationRank::Neutral && reaction_from(world, target, attacker.faction) <= ReputationRank::Neutral
}

/// Whether the creature can already hit `target` in melee.
pub fn in_melee_range(position: &Position, combat_reach: f32, target: &UnitInfo) -> bool {
    melee_range::is_within_melee_range(
        position,
        combat_reach,
        &target.position,
        target.combat_reach,
        false,
    )
}

/// 3D distance between two points.
pub fn distance(a: &Position, b: &Position) -> f32 {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Current wall-clock time in unix milliseconds.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
