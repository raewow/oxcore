//! Creature groups (creature_groups): packs that are pulled, reset and
//! respawned together, and formations whose members walk behind their leader.

use super::ai::{self, AIAction, AIEvent, AIState};
use crate::World;
use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Members walk in formation behind the leader.
pub const OPTION_FORMATION_MOVE: u32 = 0x001;
/// Pulling one member pulls the whole group.
pub const OPTION_AGGRO_TOGETHER: u32 = 0x002;
/// One member evading makes the whole group evade.
pub const OPTION_EVADE_TOGETHER: u32 = 0x004;
/// One member respawning brings the whole group back.
pub const OPTION_RESPAWN_TOGETHER: u32 = 0x008;
/// The leader evading respawns every dead member.
pub const OPTION_RESPAWN_ALL_ON_MASTER_EVADE: u32 = 0x010;
/// Any member evading respawns every dead member.
pub const OPTION_RESPAWN_ALL_ON_ANY_EVADE: u32 = 0x020;
/// The leader hears when a member dies.
pub const OPTION_INFORM_LEADER_ON_MEMBER_DIED: u32 = 0x040;
/// Every member hears when any member dies.
pub const OPTION_INFORM_MEMBERS_ON_ANY_DIED: u32 = 0x080;

#[derive(Debug, Clone)]
struct Member {
    follow_distance: f32,
    follow_angle: f32,
    flags: u32,
}

#[derive(Debug, Clone)]
struct Group {
    /// Leader spawn id.
    leader: u32,
    /// Member spawn ids (the leader is not one of them).
    members: HashMap<u32, Member>,
    /// Union of the members' flags.
    options: u32,
}

fn groups() -> &'static DashMap<u32, Group> {
    static GROUPS: OnceLock<DashMap<u32, Group>> = OnceLock::new();
    GROUPS.get_or_init(DashMap::new)
}

fn group_by_spawn() -> &'static DashMap<u32, u32> {
    static INDEX: OnceLock<DashMap<u32, u32>> = OnceLock::new();
    INDEX.get_or_init(DashMap::new)
}

/// Add one creature_groups row.
pub fn add_member(leader: u32, member: u32, follow_distance: f32, follow_angle: f32, flags: u32) {
    if leader == member {
        return;
    }
    let mut group = groups().entry(leader).or_insert_with(|| Group {
        leader,
        members: HashMap::new(),
        options: 0,
    });
    group.members.insert(
        member,
        Member {
            follow_distance,
            follow_angle,
            flags,
        },
    );
    group.options |= flags;
    group_by_spawn().insert(leader, leader);
    group_by_spawn().insert(member, leader);
}

/// Load creature_groups.
pub async fn load(pool: &sqlx::PgPool) -> anyhow::Result<usize> {
    let rows = sqlx::query(
        "SELECT leader_guid, member_guid, dist::REAL AS dist, angle::REAL AS angle, flags \
         FROM world.creature_groups ORDER BY leader_guid",
    )
    .fetch_all(pool)
    .await?;
    for row in &rows {
        add_member(
            row.try_get::<i64, _>("leader_guid")? as u32,
            row.try_get::<i64, _>("member_guid")? as u32,
            row.try_get::<f32, _>("dist")?,
            row.try_get::<f32, _>("angle")?,
            row.try_get::<i64, _>("flags")? as u32,
        );
    }
    Ok(rows.len())
}

/// The group a spawn belongs to.
fn group_of(world: &World, guid: ObjectGuid) -> Option<(Group, u32, u32, u32)> {
    let (spawn_id, map_id, instance_id) = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| (c.spawn_id, c.map_id, c.instance_id))?;
    if spawn_id == 0 {
        return None;
    }
    let leader = *group_by_spawn().get(&spawn_id)?;
    let group = groups().get(&leader)?.clone();
    Some((group, spawn_id, map_id, instance_id))
}

/// Runtime GUIDs of the group's spawns in one dungeon copy (or continent).
fn group_creatures(world: &World, group: &Group, map_id: u32, instance_id: u32) -> HashMap<u32, ObjectGuid> {
    world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|e| {
            let c = e.value();
            c.map_id == map_id
                && c.instance_id == instance_id
                && (c.spawn_id == group.leader || group.members.contains_key(&c.spawn_id))
        })
        .map(|e| (e.value().spawn_id, *e.key()))
        .collect()
}

/// A member started attacking: idle members join the fight against the same
/// target, sharing the attacker's leash clock.
pub fn on_member_attack_start(world: &World, guid: ObjectGuid, target: ObjectGuid) {
    let Some((group, spawn_id, map_id, instance_id)) = group_of(world, guid) else {
        return;
    };
    if group.options & OPTION_AGGRO_TOGETHER == 0 {
        return;
    }
    let leash = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| c.ai_state_data.leash_extension.clone());
    for (other_spawn, other) in group_creatures(world, &group, map_id, instance_id) {
        if other_spawn == spawn_id {
            continue;
        }
        let joins = world
            .managers
            .creature_mgr
            .with_creature_mut(other, |c| {
                if !c.is_alive() || c.in_world == false || c.combat.attacking.is_some() {
                    return false;
                }
                c.ai_state_data.assistance_called = true;
                if let Some(leash) = &leash {
                    c.ai_state_data.leash_extension = leash.clone();
                }
                true
            })
            .unwrap_or(false);
        if joins {
            ai::queue_event(world, other, AIEvent::AttackedBy { attacker_guid: target });
        }
    }
}

/// A member evaded: the group evades together and may bring its dead back.
pub fn on_member_evade(world: &World, guid: ObjectGuid) {
    let Some((group, spawn_id, map_id, instance_id)) = group_of(world, guid) else {
        return;
    };
    let creatures = group_creatures(world, &group, map_id, instance_id);
    let mut leader_evaded = spawn_id == group.leader;
    if group.options & OPTION_EVADE_TOGETHER != 0 {
        for (other_spawn, other) in &creatures {
            if *other_spawn == spawn_id {
                continue;
            }
            let fighting = world
                .managers
                .creature_mgr
                .with_creature(*other, |c| c.is_alive() && c.ai_state == AIState::Combat)
                .unwrap_or(false);
            if fighting {
                if *other_spawn == group.leader {
                    leader_evaded = true;
                }
                ai::executor::execute_actions(world, *other, vec![AIAction::EnterEvadeMode]);
            }
        }
    }
    if group.options & OPTION_RESPAWN_ALL_ON_ANY_EVADE != 0
        || (group.options & OPTION_RESPAWN_ALL_ON_MASTER_EVADE != 0 && leader_evaded)
    {
        respawn_all(world, &creatures, spawn_id);
    }
}

/// A member respawned: the whole group may come back with it.
pub fn on_member_respawn(world: &World, guid: ObjectGuid) {
    let Some((group, spawn_id, map_id, instance_id)) = group_of(world, guid) else {
        return;
    };
    if group.options & OPTION_RESPAWN_TOGETHER == 0 {
        return;
    }
    respawn_all(world, &group_creatures(world, &group, map_id, instance_id), spawn_id);
}

/// Bring back every dead member still waiting on its respawn timer.
fn respawn_all(world: &World, creatures: &HashMap<u32, ObjectGuid>, except: u32) {
    let now = ai::units::now_ms();
    for (spawn, guid) in creatures {
        if *spawn == except {
            continue;
        }
        let waiting = world
            .managers
            .creature_mgr
            .with_creature(*guid, |c| !c.is_alive() && c.respawn_time > now)
            .unwrap_or(false);
        if waiting {
            super::forced::force_respawn(world, *guid);
        }
    }
}

/// A member died: tell the leader and/or the other members.
pub fn on_member_died(world: &World, guid: ObjectGuid) {
    let Some((group, spawn_id, map_id, instance_id)) = group_of(world, guid) else {
        return;
    };
    let inform_leader = group.options & OPTION_INFORM_LEADER_ON_MEMBER_DIED != 0;
    let inform_members = group.options & OPTION_INFORM_MEMBERS_ON_ANY_DIED != 0;
    if !inform_leader && !inform_members {
        return;
    }
    let member_entry = guid.entry();
    let was_leader = spawn_id == group.leader;
    for (other_spawn, other) in group_creatures(world, &group, map_id, instance_id) {
        if other_spawn == spawn_id {
            continue;
        }
        let is_leader = other_spawn == group.leader;
        if !(inform_members || (inform_leader && is_leader && !was_leader)) {
            continue;
        }
        let alive = world
            .managers
            .creature_mgr
            .with_creature(other, |c| c.is_alive())
            .unwrap_or(false);
        if alive {
            ai::queue_event(
                world,
                other,
                AIEvent::GroupMemberDied {
                    member_guid: guid,
                    member_entry,
                    was_leader: inform_members && was_leader,
                },
            );
        }
    }
}

/// Members of formations that are idle and alive walk behind their leader.
pub fn update_formations(world: &World) {
    for entry in groups().iter() {
        let group = entry.value();
        if group.options & OPTION_FORMATION_MOVE == 0 {
            continue;
        }
        // Group the spawns by copy: each copy has its own leader.
        let mut by_copy: HashMap<(u32, u32), HashMap<u32, ObjectGuid>> = HashMap::new();
        for creature in world.managers.creature_mgr.iter_creatures() {
            let c = creature.value();
            if c.spawn_id == group.leader || group.members.contains_key(&c.spawn_id) {
                by_copy
                    .entry((c.map_id, c.instance_id))
                    .or_default()
                    .insert(c.spawn_id, *creature.key());
            }
        }
        for creatures in by_copy.values() {
            let Some(&leader) = creatures.get(&group.leader) else {
                continue;
            };
            let leader_alive = world
                .managers
                .creature_mgr
                .with_creature(leader, |c| c.is_alive())
                .unwrap_or(false);
            if !leader_alive {
                continue;
            }
            for (spawn, member) in &group.members {
                if member.flags & OPTION_FORMATION_MOVE == 0 {
                    continue;
                }
                let Some(&guid) = creatures.get(spawn) else {
                    continue;
                };
                world.managers.creature_mgr.with_creature_mut(guid, |c| {
                    if !c.is_alive()
                        || c.ai_state != AIState::Idle
                        || c.following_target == Some(leader)
                    {
                        return;
                    }
                    let position = c.position;
                    let speed = c.walk_speed();
                    c.motion_master.move_follow(
                        leader,
                        member.follow_distance,
                        member.follow_angle,
                        guid,
                        position,
                        speed,
                    );
                    c.following_target = Some(leader);
                });
            }
        }
    }
}
