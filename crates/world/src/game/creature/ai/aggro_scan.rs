//! Proximity aggro.
//!
//! Periodically, every creature that can start a fight looks at the units
//! around it — players and creatures — and attacks the first hostile one that
//! is close enough, visible, in line of sight and attackable. Creatures only
//! look while they have no victim, except in dungeons (and when flagged never
//! to leash), where they also pull in newcomers to an ongoing fight.
//!
//! Guards also attack players who are attacking their friends, from up to 30
//! yards. A creature that notices a stealthed unit it cannot quite see turns
//! to face it and pauses for a few seconds.

use super::aggro::{
    attack_distance, detects_stealth, is_in_front, CREATURE_Z_ATTACK_RANGE,
    MAX_CREATURE_STEALTH_DETECT_RANGE,
};
use super::system::queue_event;
use super::types::{AIEvent, AIState, AIType, ReactState};
use super::units::{
    self, Attacker, UnitInfo, SPELL_AURA_MOD_DETECT_RANGE, SPELL_AURA_MOD_INVISIBILITY_DETECTION,
    SPELL_AURA_MOD_STEALTH_DETECT,
};
use crate::game::common::creature_flags::{
    CREATURE_FLAG_EXTRA_NO_LEASH_EVADE, CREATURE_STATIC_FLAG_ONLY_ATTACK_PVP_ENABLING,
};
use crate::game::common::unit_flags;
use crate::game::creature::manager::INHABIT_AIR;
use crate::game::faction;
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Furthest any creature can notice a unit (detection range plus level and aura bonuses).
const MAX_SCAN_RANGE: f32 = 60.0;
/// Creatures only look around while a player is this close (an active grid).
const ACTIVE_RANGE: f32 = 120.0;
/// Guards notice players attacking their friends from at least this far.
const GUARD_ASSIST_RANGE: f32 = 30.0;
/// Stealth alerts have this cooldown.
const STEALTH_ALERT_COOLDOWN_MS: u64 = 10_000;
/// A creature alerted by a stealthed unit pauses this long.
const STEALTH_ALERT_DISTRACT_MS: u32 = 5_000;
/// Chests guarded by their surroundings pull hostiles within this radius of the chest.
const AGGRO_ON_OPEN_RADIUS: f32 = 10.0;

/// The scanning creature.
struct Scanner {
    guid: ObjectGuid,
    entry: u32,
    attacker: Attacker,
    position: Position,
    level: u8,
    ai_type: AIType,
    ai_state: AIState,
    has_victim: bool,
    react_state: ReactState,
    detection_range: f32,
    detect_range_mod: i32,
    stealth_detect: i32,
    detects_invisibility: bool,
    can_fly: bool,
    only_attack_pvp: bool,
    no_leash: bool,
    civilian: bool,
    pacified: bool,
    last_alert_ms: u64,
}

impl Scanner {
    /// Whether this creature could start a fight right now.
    fn can_initiate_attack(&self, world: &World) -> bool {
        self.react_state == ReactState::Aggressive
            && !self.pacified
            && self.attacker.unit_flags
                & (unit_flags::STUNNED | unit_flags::SPAWNING | unit_flags::NOT_SELECTABLE)
                == 0
            && !faction::is_neutral_to_all(&world.dbc.read(), self.attacker.faction)
    }

    fn distance_to(&self, target: &UnitInfo) -> f32 {
        units::distance(&self.position, &target.position)
    }

    /// Whether the creature can see `target`: stealth and invisibility checks.
    fn sees(&self, target: &UnitInfo) -> bool {
        if target.invisible && !self.detects_invisibility {
            return false;
        }
        match target.stealth {
            Some(stealth) => detects_stealth(
                self.level,
                self.stealth_detect,
                target.level,
                stealth,
                self.distance_to(target),
                is_in_front(
                    self.position.o,
                    (self.position.x, self.position.y),
                    (target.position.x, target.position.y),
                ),
            ),
            None => true,
        }
    }
}

/// Scan for units in aggro range. Called periodically from the world update.
pub async fn scan_for_aggro(world: &World) -> anyhow::Result<()> {
    let now = units::now_ms();
    let scanners: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|entry| {
            let c = entry.value();
            c.is_alive()
                && c.in_world
                && matches!(c.ai_state, AIState::Idle | AIState::Combat)
                && !matches!(
                    c.ai_profile.ai_type,
                    AIType::Null | AIType::Critter | AIType::Totem | AIType::Pet
                )
        })
        .map(|entry| *entry.key())
        .collect();

    for guid in scanners {
        check_creature_aggro(world, guid, now);
    }
    Ok(())
}

fn scanner(world: &World, guid: ObjectGuid, now: u64) -> Option<Scanner> {
    world.managers.creature_mgr.with_creature(guid, |c| Scanner {
        guid,
        entry: c.entry,
        attacker: Attacker {
            guid,
            map_id: c.map_id,
            instance_id: c.instance_id,
            faction: c.faction,
            unit_flags: c.unit_flags,
            player_controlled: false,
        },
        position: c.position,
        level: c.level,
        ai_type: c.ai_profile.ai_type,
        ai_state: c.ai_state,
        has_victim: c.combat.attacking.is_some(),
        react_state: c.ai_state_data.react_state,
        detection_range: c.ai_profile.detection_range,
        detect_range_mod: c.auras.get_total_aura_modifier(SPELL_AURA_MOD_DETECT_RANGE),
        stealth_detect: c
            .auras
            .get_total_aura_modifier_by_misc(SPELL_AURA_MOD_STEALTH_DETECT, 0),
        detects_invisibility: c.auras.has_aura_type(SPELL_AURA_MOD_INVISIBILITY_DETECTION),
        can_fly: false,
        only_attack_pvp: c.static_flags1 & CREATURE_STATIC_FLAG_ONLY_ATTACK_PVP_ENABLING != 0,
        no_leash: c.ai_profile.has_extra_flag(CREATURE_FLAG_EXTRA_NO_LEASH_EVADE),
        civilian: c.ai_profile.civilian,
        pacified: c.ai_state_data.pacified_until_ms > now,
        last_alert_ms: c.ai_state_data.last_alert_ms,
    })
}

fn check_creature_aggro(world: &World, guid: ObjectGuid, now: u64) {
    let Some(mut me) = scanner(world, guid, now) else {
        return;
    };
    me.can_fly = world.managers.creature_mgr.inhabit_type(me.entry) & INHABIT_AIR != 0;

    let Some(map) = world
        .managers
        .map_mgr
        .get_map(me.attacker.map_id, me.attacker.instance_id)
    else {
        return;
    };
    // Grids far from every player are not active.
    if map.player_count() == 0
        || map
            .get_players_in_range_limit(me.position, ACTIVE_RANGE, 1)
            .is_empty()
    {
        return;
    }

    let in_dungeon = crate::game::instance::lifecycle::is_dungeon_map(world, me.attacker.map_id);
    // Proximity aggro while already fighting only happens in dungeons.
    if me.has_victim && !(in_dungeon || me.no_leash) {
        return;
    }
    if me.ai_type == AIType::Guard && me.has_victim {
        return;
    }

    let has_lua = world.managers.lua_mgr.has_creature_ai(me.entry)
        || (me.ai_type == AIType::Event && !me.has_victim && super::event_ai::has_ooc_los(me.entry));
    let mut candidates: Vec<UnitInfo> = map
        .get_objects_in_range(me.position, MAX_SCAN_RANGE)
        .into_iter()
        .filter(|g| *g != guid && (g.is_player() || g.is_creature()))
        .filter_map(|g| units::resolve_unit(world, g))
        .filter(|t| t.map_id == me.attacker.map_id && t.instance_id == me.attacker.instance_id)
        .filter(|t| t.alive)
        .collect();
    candidates.sort_by(|a, b| me.distance_to(a).total_cmp(&me.distance_to(b)));

    let can_initiate = me.can_initiate_attack(world);
    let mut line_of_sight_reported = false;

    let (map_id, instance_id, my_position) =
        (me.attacker.map_id, me.attacker.instance_id, me.position);
    for target in &candidates {
        let distance = me.distance_to(target);
        let target_position = target.position;
        let los = || {
            world.managers.vmap_mgr.is_in_line_of_sight(
                map_id,
                instance_id,
                my_position,
                target_position,
            )
        };

        // Lua scripts see every player in view (sensors, tower detectors).
        if has_lua && target.is_player && !line_of_sight_reported {
            let range = attack_distance(
                me.detection_range,
                me.level,
                target.level,
                me.detect_range_mod,
                target.detected_range_mod,
            );
            if distance <= range && me.sees(target) && los() {
                line_of_sight_reported = true;
                let is_hostile = units::reaction_to(world, me.attacker.faction, target)
                    <= ReputationRank::Hostile;
                queue_event(
                    world,
                    guid,
                    AIEvent::UnitInLineOfSight {
                        unit_guid: target.guid,
                        is_hostile,
                    },
                );
            }
        }

        if !can_initiate {
            continue;
        }

        if target.stealth.is_some() && !me.sees(target) {
            maybe_stealth_alert(world, &mut me, target, distance, now, &los);
            continue;
        }
        if !me.sees(target) {
            continue;
        }

        let engage = if me.ai_type == AIType::Guard {
            guard_wants(world, &me, target, distance)
        } else {
            basic_wants(world, &me, target, distance)
        };
        if engage && los() {
            queue_event(
                world,
                guid,
                AIEvent::UnitInRange {
                    unit_guid: target.guid,
                    distance,
                    is_hostile: true,
                    is_player: target.is_player,
                },
            );
            return;
        }
    }
}

/// Basic proximity aggro: a hostile, attackable unit within the attack distance.
fn basic_wants(world: &World, me: &Scanner, target: &UnitInfo, distance: f32) -> bool {
    if me.only_attack_pvp
        && target.is_player_controlled()
        && target.unit_flags & unit_flags::PVP == 0
    {
        return false;
    }
    if !me.can_fly && (me.position.z - target.position.z).abs() > CREATURE_Z_ATTACK_RANGE {
        return false;
    }
    let range = attack_distance(
        me.detection_range,
        me.level,
        target.level,
        me.detect_range_mod,
        target.detected_range_mod,
    );
    distance <= range
        && units::is_targetable_by(&me.attacker, target)
        && units::reaction_to(world, me.attacker.faction, target) <= ReputationRank::Hostile
}

/// Guard proximity aggro: hostile units, monsters hostile to every player, and
/// players attacking the guard's friends (from at least 30 yards).
fn guard_wants(world: &World, me: &Scanner, target: &UnitInfo, distance: f32) -> bool {
    if !me.can_fly && !target.is_player && (me.position.z - target.position.z).abs() > CREATURE_Z_ATTACK_RANGE
    {
        return false;
    }
    let mut radius = attack_distance(
        me.detection_range,
        me.level,
        target.level,
        me.detect_range_mod,
        target.detected_range_mod,
    );
    let mut attacking_friend = false;
    if target.is_player
        && units::reaction_to(world, me.attacker.faction, target) < ReputationRank::Friendly
    {
        attacking_friend = target.victim.is_some_and(|victim| {
            units::resolve_unit(world, victim).is_some_and(|v| {
                v.on_taxi
                    || units::reaction_to(world, me.attacker.faction, &v) >= ReputationRank::Friendly
            })
        });
        if attacking_friend {
            radius = radius.max(GUARD_ASSIST_RANGE);
        }
    }
    if distance > radius || !units::is_valid_attack_target(world, &me.attacker, target) {
        return false;
    }
    let hostile_to_players =
        !target.is_player && faction::is_hostile_to_players(&world.dbc.read(), target.faction);
    hostile_to_players
        || units::reaction_to(world, me.attacker.faction, target) <= ReputationRank::Hostile
        || attacking_friend
}

/// A stealthed unit just beyond what the creature sees: within five yards of
/// detection the creature turns to it and pauses for five seconds, at most
/// once every ten seconds.
fn maybe_stealth_alert(
    world: &World,
    me: &mut Scanner,
    target: &UnitInfo,
    distance: f32,
    now: u64,
    los: &dyn Fn() -> bool,
) {
    let Some(stealth) = target.stealth else {
        return;
    };
    if me.has_victim || me.civilian || distance > MAX_CREATURE_STEALTH_DETECT_RANGE + 5.0 {
        return;
    }
    if now < me.last_alert_ms + STEALTH_ALERT_COOLDOWN_MS {
        return;
    }
    // Seen five yards further out than detection reaches?
    let in_front = is_in_front(
        me.position.o,
        (me.position.x, me.position.y),
        (target.position.x, target.position.y),
    );
    let alerted = detects_stealth(
        me.level,
        me.stealth_detect,
        target.level,
        stealth,
        (distance - 5.0).max(0.0),
        in_front,
    );
    if !alerted || !units::is_valid_attack_target(world, &me.attacker, target) || !los() {
        return;
    }
    me.last_alert_ms = now;
    let angle = (target.position.y - me.position.y).atan2(target.position.x - me.position.x);
    let position = me.position;
    world
        .managers
        .creature_mgr
        .with_creature_mut(me.guid, |c| {
            c.ai_state_data.last_alert_ms = now;
            c.move_spline.stop();
            c.position.o = angle;
            c.motion_master
                .move_distract(me.guid, STEALTH_ALERT_DISTRACT_MS);
        });
    world
        .systems
        .creature_movement
        .send_facing_packet(me.guid, position, angle, world);
    queue_event(
        world,
        me.guid,
        AIEvent::StealthAlert {
            unit_guid: target.guid,
        },
    );
}

/// Opening a treasure chest pulls the hostile creatures within ten yards of it
/// onto the opener.
pub fn aggro_on_open(world: &World, opener: ObjectGuid, object: ObjectGuid) {
    const GAMEOBJECT_TYPE_CHEST: u32 = 3;
    const TREASURE_FACTION: u32 = 77;
    const LOCK_KEY_SKILL: u32 = 2;
    const LOCKTYPE_TREASURE: u32 = 6;

    let Some((entry, map_id, instance_id, position)) = world
        .managers
        .gameobject_mgr
        .with_gameobject(object, |go| (go.entry, go.map_id, go.instance_id, go.position))
    else {
        return;
    };
    let Some(template) = world.managers.gameobject_mgr.get_template(entry) else {
        return;
    };
    if template.go_type != GAMEOBJECT_TYPE_CHEST || template.faction != TREASURE_FACTION {
        return;
    }
    let lock_id = template.data[0].max(0) as u32;
    let treasure_lock = world.dbc.read().get_lock(lock_id).is_some_and(|lock| {
        (0..8).any(|i| lock.lock_type[i] == LOCK_KEY_SKILL && lock.lock_index[i] == LOCKTYPE_TREASURE)
    });
    if !treasure_lock {
        return;
    }
    let Some(user) = units::resolve_unit(world, opener) else {
        return;
    };
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return;
    };
    for guid in map.get_objects_in_range(position, AGGRO_ON_OPEN_RADIUS) {
        if !guid.is_creature() {
            continue;
        }
        let Some((attacker, creature_pos, owned, can_react)) =
            world.managers.creature_mgr.with_creature(guid, |c| {
                (
                    Attacker {
                        guid,
                        map_id: c.map_id,
                        instance_id: c.instance_id,
                        faction: c.faction,
                        unit_flags: c.unit_flags,
                        player_controlled: false,
                    },
                    c.position,
                    c.owner_guid.is_some() || c.charm_guid.is_some(),
                    c.is_alive()
                        && c.unit_flags
                            & (unit_flags::STUNNED | unit_flags::CONFUSED | unit_flags::FLEEING)
                            == 0,
                )
            })
        else {
            continue;
        };
        if owned || !can_react {
            continue;
        }
        if units::distance(&creature_pos, &position) > AGGRO_ON_OPEN_RADIUS {
            continue;
        }
        if units::reaction_to(world, attacker.faction, &user) > ReputationRank::Hostile {
            continue;
        }
        if !units::is_valid_attack_target(world, &attacker, &user) {
            continue;
        }
        if !world
            .managers
            .vmap_mgr
            .is_in_line_of_sight(map_id, instance_id, creature_pos, user.position)
        {
            continue;
        }
        queue_event(world, guid, AIEvent::Engage { target_guid: opener });
    }
}
