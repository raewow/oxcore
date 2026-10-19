//! Creatures helping each other.
//!
//! * Assistance: when a creature starts attacking, idle creatures of the same
//!   faction within 10 yards that can see it join the fight 1.5 seconds later.
//!   Creatures pulled this way share the leash clock of the one that called.
//! * Call for help: while fighting more than 10 yards from home, a creature
//!   calls once a second within its template's call-for-help range; creatures
//!   friendly to it (not only its own faction) join at once, and creatures whose
//!   faction flees from calls for help run away instead.
//! * Zone combat: raid bosses keep every player in the dungeon in combat.

use super::system::queue_event;
use super::types::{AIAction, AIEvent, AIState, PendingAssistance, ReactState};
use super::units::{self, Attacker, UnitInfo};
use crate::game::common::creature_flags::CREATURE_FLAG_EXTRA_NO_ASSIST;
use crate::game::common::unit_flags;
use crate::game::creature::movement::MovementGeneratorType;
use crate::game::faction::{
    self, FACTION_TEMPLATE_FLEE_FROM_CALL_FOR_HELP, FACTION_TEMPLATE_RESPOND_TO_CALL_FOR_HELP,
};
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Radius of the assistance call on aggro.
pub const ASSISTANCE_RADIUS: f32 = 10.0;
/// Delay before assistants join.
pub const ASSISTANCE_DELAY_MS: u32 = 1500;
/// Creatures that flee from a call for help run this far.
const FLEE_FROM_CALL_DISTANCE: f32 = 10.0;

/// A creature that could answer a call, as seen by the caller.
#[derive(Debug, Clone)]
struct Helper {
    guid: ObjectGuid,
    position: Position,
    faction: u32,
    alive: bool,
    in_combat: bool,
    unit_flags: u32,
    owned: bool,
    evading: bool,
    react_state: ReactState,
    flags_extra: u32,
    pacified: bool,
    wandering: bool,
}

/// The calling creature.
struct Caller {
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    position: Position,
    faction: u32,
    victim: Option<ObjectGuid>,
    is_pet: bool,
    charmed: bool,
    call_for_help_range: f32,
    assistance_called: bool,
}

fn caller(world: &World, guid: ObjectGuid) -> Option<Caller> {
    world.managers.creature_mgr.with_creature(guid, |c| Caller {
        guid,
        map_id: c.map_id,
        instance_id: c.instance_id,
        position: c.position,
        faction: c.faction,
        victim: c.combat.attacking,
        is_pet: c.owner_guid.is_some(),
        charmed: c.charm_guid.is_some(),
        call_for_help_range: c.ai_profile.call_for_help_range,
        assistance_called: c.ai_state_data.assistance_called,
    })
}

fn helper(world: &World, guid: ObjectGuid, now_ms: u64) -> Option<Helper> {
    world.managers.creature_mgr.with_creature(guid, |c| Helper {
        guid,
        position: c.position,
        faction: c.faction,
        alive: c.is_alive(),
        in_combat: c.combat.in_combat,
        unit_flags: c.unit_flags,
        owned: c.owner_guid.is_some() || c.charm_guid.is_some(),
        evading: matches!(c.ai_state, AIState::Evading | AIState::Returning),
        react_state: c.ai_state_data.react_state,
        flags_extra: c.ai_profile.flags_extra,
        pacified: c.ai_state_data.pacified_until_ms > now_ms,
        wandering: c.motion_master.active_generator() == MovementGeneratorType::Random,
    })
}

/// Creatures in the caller's copy within `radius` that it can see.
fn creatures_around(world: &World, caller: &Caller, radius: f32, now_ms: u64) -> Vec<Helper> {
    let Some(map) = world
        .managers
        .map_mgr
        .get_map(caller.map_id, caller.instance_id)
    else {
        return Vec::new();
    };
    map.get_objects_in_range(caller.position, radius)
        .into_iter()
        .filter(|g| g.is_creature() && *g != caller.guid)
        .filter_map(|g| helper(world, g, now_ms))
        .filter(|h| units::distance(&h.position, &caller.position) <= radius)
        .filter(|h| {
            world.managers.vmap_mgr.is_in_line_of_sight(
                caller.map_id,
                caller.instance_id,
                caller.position,
                h.position,
            )
        })
        .collect()
}

/// Whether `helper` may be called against `enemy` on behalf of `caller`.
fn can_be_targeted_by_call(
    world: &World,
    helper: &Helper,
    caller_faction: u32,
    enemy: &UnitInfo,
    same_faction_only: bool,
) -> bool {
    let flags = faction::template_flags(&world.dbc.read(), helper.faction);
    if flags & (FACTION_TEMPLATE_RESPOND_TO_CALL_FOR_HELP | FACTION_TEMPLATE_FLEE_FROM_CALL_FOR_HELP)
        == 0
    {
        return false;
    }
    if !helper.alive || helper.in_combat || helper.owned {
        return false;
    }
    if helper.unit_flags & (unit_flags::STUNNED | unit_flags::SPAWNING | unit_flags::NOT_SELECTABLE)
        != 0
    {
        return false;
    }
    if helper.faction != caller_faction {
        let friendly = faction::template_reaction(&world.dbc.read(), helper.faction, caller_faction)
            >= ReputationRank::Friendly;
        if same_faction_only || !friendly {
            return false;
        }
    }
    // Neutral creatures answer too (kodos in Mulgore), but never against a friend.
    if units::reaction_to(world, helper.faction, enemy) >= ReputationRank::Friendly {
        return false;
    }
    let attacker = Attacker {
        guid: helper.guid,
        map_id: enemy.map_id,
        instance_id: enemy.instance_id,
        faction: helper.faction,
        unit_flags: helper.unit_flags,
        player_controlled: false,
    };
    units::is_targetable_by(&attacker, enemy)
}

/// Whether `helper` joins the fight against `enemy` (rather than ignoring the call).
fn can_respond_against(world: &World, helper: &Helper, enemy: &UnitInfo, caller: &Caller) -> bool {
    let flags = faction::template_flags(&world.dbc.read(), helper.faction);
    if flags & FACTION_TEMPLATE_RESPOND_TO_CALL_FOR_HELP == 0
        || flags & FACTION_TEMPLATE_FLEE_FROM_CALL_FOR_HELP != 0
    {
        return false;
    }
    if helper.flags_extra & CREATURE_FLAG_EXTRA_NO_ASSIST != 0
        || helper.react_state == ReactState::Passive
        || helper.pacified
        || helper.evading
    {
        return false;
    }
    // Never pull a player into combat with a creature beyond their sight, except in dungeons.
    if enemy.is_player_controlled()
        && !crate::game::instance::lifecycle::is_dungeon_map(world, caller.map_id)
    {
        let visibility = world
            .managers
            .map_mgr
            .get_map(caller.map_id, caller.instance_id)
            .map(|m| m.visibility_distance())
            .unwrap_or(100.0);
        if units::distance(&helper.position, &enemy.position) > visibility {
            return false;
        }
    }
    true
}

/// Whether `helper` runs away from `enemy` instead of answering.
fn can_flee_from(world: &World, helper: &Helper, enemy: &UnitInfo) -> bool {
    let flags = faction::template_flags(&world.dbc.read(), helper.faction);
    flags & FACTION_TEMPLATE_FLEE_FROM_CALL_FOR_HELP != 0
        && helper.wandering
        && units::distance(&helper.position, &enemy.position) < FLEE_FROM_CALL_DISTANCE
}

/// Called when a creature starts attacking: find same-faction assistants who
/// will join after the assistance delay. Once per fight.
pub fn call_assistance(world: &World, guid: ObjectGuid) {
    let now = units::now_ms();
    let Some(me) = caller(world, guid) else {
        return;
    };
    if me.assistance_called || me.is_pet || me.charmed || me.call_for_help_range <= 0.0 {
        return;
    }
    let Some(victim) = me.victim else {
        return;
    };
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        c.ai_state_data.assistance_called = true;
    });
    let Some(enemy) = units::resolve_unit(world, victim) else {
        return;
    };

    let assistants: Vec<ObjectGuid> = creatures_around(world, &me, ASSISTANCE_RADIUS, now)
        .into_iter()
        .filter(|h| {
            can_be_targeted_by_call(world, h, me.faction, &enemy, true)
                && can_respond_against(world, h, &enemy, &me)
        })
        .map(|h| h.guid)
        .collect();
    if assistants.is_empty() {
        return;
    }
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        c.ai_state_data.pending_assistance = Some(PendingAssistance {
            delay_ms: ASSISTANCE_DELAY_MS,
            victim,
            assistants,
        });
    });
}

/// Count down a pending assistance call and pull the assistants in when it fires.
pub fn process_pending_assistance(world: &World, guid: ObjectGuid, diff_ms: u32) {
    let fired = world
        .managers
        .creature_mgr
        .with_creature_mut(guid, |c| {
            let pending = c.ai_state_data.pending_assistance.as_mut()?;
            if pending.delay_ms > diff_ms {
                pending.delay_ms -= diff_ms;
                return None;
            }
            let pending = c.ai_state_data.pending_assistance.take()?;
            Some((pending, c.ai_state_data.leash_extension.clone()))
        })
        .flatten();
    let Some((pending, leash_clock)) = fired else {
        return;
    };

    let now = units::now_ms();
    let Some(me) = caller(world, guid) else {
        return;
    };
    let Some(enemy) = units::resolve_unit(world, pending.victim) else {
        return;
    };
    if !enemy.alive || enemy.map_id != me.map_id || enemy.instance_id != me.instance_id {
        return;
    }
    for assistant in pending.assistants {
        let Some(h) = helper(world, assistant, now) else {
            continue;
        };
        if !(can_be_targeted_by_call(world, &h, me.faction, &enemy, true)
            && can_respond_against(world, &h, &enemy, &me))
        {
            continue;
        }
        world.managers.creature_mgr.with_creature_mut(assistant, |c| {
            c.ai_state_data.assistance_called = true;
            // Pulled together: hitting any of them keeps the rest from leashing.
            c.ai_state_data.leash_extension = leash_clock.clone();
        });
        queue_event(
            world,
            assistant,
            AIEvent::AssistanceRequested {
                caller_guid: guid,
                target_guid: pending.victim,
            },
        );
    }
}

/// The once-a-second call for help of a creature fighting away from home.
pub fn call_for_help(world: &World, guid: ObjectGuid, radius: f32) {
    if radius <= 0.0 {
        return;
    }
    let now = units::now_ms();
    let Some(me) = caller(world, guid) else {
        return;
    };
    if me.is_pet || me.charmed {
        return;
    }
    let Some(victim) = me.victim else {
        return;
    };
    let Some(enemy) = units::resolve_unit(world, victim) else {
        return;
    };

    for h in creatures_around(world, &me, radius, now) {
        if !can_be_targeted_by_call(world, &h, me.faction, &enemy, false) {
            continue;
        }
        if can_respond_against(world, &h, &enemy, &me) {
            queue_event(
                world,
                h.guid,
                AIEvent::AssistanceRequested {
                    caller_guid: guid,
                    target_guid: victim,
                },
            );
        } else if can_flee_from(world, &h, &enemy) {
            super::executor::execute_actions(
                world,
                h.guid,
                vec![AIAction::FleeFrom {
                    flee_from_guid: victim,
                    distance: FLEE_FROM_CALL_DISTANCE,
                    duration_ms: 3000,
                }],
            );
        }
    }
}

/// Put every attackable player in the creature's dungeon copy (and their pets)
/// in combat with it. On the first pulse a creature without a victim attacks
/// the nearest hostile player; later pulses only add players who are not yet
/// fighting. Only works in dungeons.
pub fn set_in_combat_with_zone(world: &World, guid: ObjectGuid, initial_pulse: bool) {
    let Some((map_id, instance_id, position, faction, unit_flags, victim)) =
        world.managers.creature_mgr.with_creature(guid, |c| {
            (
                c.map_id,
                c.instance_id,
                c.position,
                c.faction,
                c.unit_flags,
                c.combat.attacking,
            )
        })
    else {
        return;
    };
    if !crate::game::instance::lifecycle::is_dungeon_map(world, map_id) {
        tracing::warn!(
            "Creature {:?} asked for zone combat outside a dungeon (map {})",
            guid,
            map_id
        );
        return;
    }
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return;
    };
    let attacker = Attacker {
        guid,
        map_id,
        instance_id,
        faction,
        unit_flags,
        player_controlled: false,
    };

    let players: Vec<UnitInfo> = map
        .get_players_in_range(position, f32::MAX)
        .into_iter()
        .filter_map(|p| units::resolve_unit(world, p))
        .filter(|p| p.map_id == map_id && p.instance_id == instance_id)
        .collect();

    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        c.ai_state_data.combat_with_zone = true;
    });

    if initial_pulse && victim.is_none() {
        let nearest = players
            .iter()
            .filter(|p| units::is_valid_attack_target(world, &attacker, p))
            .min_by(|a, b| {
                units::distance(&a.position, &position)
                    .total_cmp(&units::distance(&b.position, &position))
            });
        if let Some(nearest) = nearest {
            super::executor::execute_actions(
                world,
                guid,
                vec![AIAction::EnterCombat {
                    target_guid: nearest.guid,
                }],
            );
        }
    }

    let mut joined = Vec::new();
    for player in &players {
        if !initial_pulse && player.in_combat {
            continue;
        }
        if !units::is_valid_attack_target(world, &attacker, player) {
            continue;
        }
        joined.push(player.guid);
        if let Some(pet) = world
            .managers
            .player_mgr
            .with_player(player.guid, |p| p.active_pet)
            .flatten()
        {
            if let Some(pet_info) = units::resolve_unit(world, pet) {
                if units::is_valid_attack_target(world, &attacker, &pet_info) {
                    joined.push(pet);
                }
            }
        }
    }
    let actions = joined
        .into_iter()
        .map(|target_guid| AIAction::AddThreat {
            target_guid,
            amount: 0.0,
        })
        .collect::<Vec<_>>();
    if !actions.is_empty() {
        super::executor::execute_actions(world, guid, actions);
        for target in world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.threat_manager.online_targets())
            .unwrap_or_default()
        {
            super::executor::put_in_combat_with(world, target, guid);
        }
    }
}
