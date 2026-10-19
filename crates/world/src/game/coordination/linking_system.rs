//! Creature linking: linked creatures react when their master is pulled,
//! evades, dies, respawns or despawns, and some cannot spawn while their
//! master is alive (or dead).

use super::link_flags::{LinkEvent, LinkFlags};
use super::linking_manager::LinkingManager;
use crate::game::broadcast_mgr::BroadcastManagerTrait;
use crate::game::creature::ai::{self, AIAction, AIEvent};
use crate::game::creature::forced;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::sync::Arc;

/// A creature taking part in a link.
#[derive(Debug, Clone)]
struct Linked {
    guid: ObjectGuid,
    spawn_id: u32,
    entry: u32,
    map_id: u32,
    instance_id: u32,
    home: Position,
    alive: bool,
    in_combat: bool,
    controlled: bool,
    respawn_time: u64,
    force_raid_combat: bool,
}

/// LinkingSystem - coordinates event propagation between linked creatures
pub struct LinkingSystem {
    manager: Arc<LinkingManager>,
}

fn linked(world: &World, guid: ObjectGuid) -> Option<Linked> {
    world.managers.creature_mgr.with_creature(guid, |c| Linked {
        guid,
        spawn_id: c.spawn_id,
        entry: c.entry,
        map_id: c.map_id,
        instance_id: c.instance_id,
        home: c.home_position,
        alive: c.is_alive(),
        in_combat: c.combat.in_combat,
        controlled: c.owner_guid.is_some() || c.charm_guid.is_some(),
        respawn_time: c.respawn_time,
        force_raid_combat: c.ai_profile.static_flags2
            & crate::game::common::creature_flags::CREATURE_STATIC_FLAG_2_FORCE_RAID_COMBAT
            != 0,
    })
}

/// Creatures of the source's copy matching `filter`.
fn creatures_in_copy(world: &World, map_id: u32, instance_id: u32, filter: impl Fn(&Linked) -> bool) -> Vec<Linked> {
    let guids: Vec<ObjectGuid> = world
        .managers
        .creature_mgr
        .iter_creatures()
        .filter(|e| e.value().map_id == map_id && e.value().instance_id == instance_id)
        .map(|e| *e.key())
        .collect();
    guids
        .into_iter()
        .filter_map(|g| linked(world, g))
        .filter(|l| filter(l))
        .collect()
}

/// A slave belongs to a master when its spawn point is within the search range
/// of the master's (no range means any distance).
fn in_range(slave_home: &Position, master_home: &Position, search_range: u32) -> bool {
    if search_range == 0 {
        return true;
    }
    let (dx, dy) = (slave_home.x - master_home.x, slave_home.y - master_home.y);
    dx * dx + dy * dy < (search_range * search_range) as f32
}

fn now_secs_ms() -> u64 {
    ai::units::now_ms()
}

impl LinkingSystem {
    pub fn new(manager: Arc<LinkingManager>, _broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { manager }
    }

    /// React to `event` happening to `source`.
    pub fn on_event(&self, world: &World, source: ObjectGuid, event: LinkEvent) {
        let Some(me) = linked(world, source) else {
            return;
        };
        if me.controlled {
            return;
        }

        let mask = event.slave_mask();
        for link in self.manager.slaves_of_entry(me.entry) {
            if link.map_id != me.map_id {
                continue;
            }
            let flags = link.flags & mask;
            if flags.is_empty() {
                continue;
            }
            for slave in creatures_in_copy(world, me.map_id, me.instance_id, |c| {
                c.entry == link.slave_entry && c.guid != me.guid
            }) {
                if in_range(&slave.home, &me.home, link.search_range) {
                    self.process_slave(world, event, &me, flags, &slave);
                }
            }
        }
        if me.spawn_id != 0 {
            for link in self.manager.slaves_of_spawn(me.spawn_id) {
                let flags = link.flags & mask;
                if flags.is_empty() {
                    continue;
                }
                for slave in creatures_in_copy(world, me.map_id, me.instance_id, |c| {
                    c.spawn_id == link.slave_guid
                }) {
                    self.process_slave(world, event, &me, flags, &slave);
                }
            }
        }

        // The event travelling up to the master.
        let master_mask = event.master_mask();
        if master_mask.is_empty() {
            return;
        }
        let Some(master) = self.find_master(world, &me, master_mask) else {
            return;
        };
        match event {
            LinkEvent::Aggro { enemy } => {
                if master.controlled {
                    return;
                }
                ai::queue_event(world, master.guid, AIEvent::Engage { target_guid: enemy });
                if !master.alive {
                    forced::force_respawn(world, master.guid);
                }
            }
            LinkEvent::Evade => {
                if !master.alive {
                    forced::force_respawn(world, master.guid);
                }
            }
            LinkEvent::Respawn => {
                if master.alive {
                    set_following(world, &me, &master);
                }
            }
            LinkEvent::Die | LinkEvent::Despawn => {}
        }
    }

    /// The master a slave answers to, when its link carries `flags`.
    fn find_master(&self, world: &World, me: &Linked, flags: LinkFlags) -> Option<Linked> {
        if me.spawn_id != 0 {
            if let Some(link) = self.manager.master_of_spawn(me.spawn_id) {
                if !link.flags.intersects(flags) {
                    return None;
                }
                return creatures_in_copy(world, me.map_id, me.instance_id, |c| {
                    c.spawn_id == link.master_guid
                })
                .into_iter()
                .next();
            }
        }
        let link = self.manager.master_of_entry(me.entry, me.map_id)?;
        if !link.flags.intersects(flags) {
            return None;
        }
        creatures_in_copy(world, me.map_id, me.instance_id, |c| c.entry == link.master_entry)
            .into_iter()
            .find(|m| in_range(&me.home, &m.home, link.search_range))
    }

    fn process_slave(&self, world: &World, event: LinkEvent, master: &Linked, flags: LinkFlags, slave: &Linked) {
        if slave.controlled {
            return;
        }
        let in_dungeon = crate::game::instance::lifecycle::is_dungeon_map(world, master.map_id);
        match event {
            LinkEvent::Aggro { enemy } => {
                if !flags.contains(LinkFlags::AGGRO_ON_AGGRO) {
                    return;
                }
                let zone = in_dungeon && master.force_raid_combat;
                if slave.in_combat {
                    if zone {
                        ai::assist::set_in_combat_with_zone(world, slave.guid, false);
                    } else {
                        ai::executor::execute_actions(
                            world,
                            slave.guid,
                            vec![AIAction::AddThreat {
                                target_guid: enemy,
                                amount: 0.0,
                            }],
                        );
                    }
                } else if slave.alive {
                    ai::executor::execute_actions(
                        world,
                        slave.guid,
                        vec![AIAction::EnterCombat { target_guid: enemy }],
                    );
                    if zone {
                        ai::assist::set_in_combat_with_zone(world, slave.guid, true);
                    }
                }
            }
            LinkEvent::Evade => {
                if flags.contains(LinkFlags::DESPAWN_ON_EVADE) && slave.alive {
                    forced::force_despawn(world, slave.guid);
                }
                if flags.contains(LinkFlags::EVADE_ON_EVADE) && slave.alive {
                    ai::executor::execute_actions(world, slave.guid, vec![AIAction::EnterEvadeMode]);
                }
                if flags.contains(LinkFlags::RESPAWN_ON_EVADE) && !slave.alive {
                    forced::force_respawn(world, slave.guid);
                }
            }
            LinkEvent::Die => {
                if flags.contains(LinkFlags::SELFKILL_ON_DEATH) && slave.alive {
                    forced::force_kill(world, slave.guid, None);
                }
                if flags.contains(LinkFlags::DESPAWN_ON_DEATH) && slave.alive {
                    forced::force_despawn(world, slave.guid);
                }
                if flags.contains(LinkFlags::RESPAWN_ON_DEATH) && !slave.alive {
                    forced::force_respawn(world, slave.guid);
                }
            }
            LinkEvent::Respawn => {
                if flags.contains(LinkFlags::RESPAWN_ON_RESPAWN) {
                    // Only a slave still waiting on its timer, so a group respawn cannot loop.
                    if !slave.alive && slave.respawn_time > now_secs_ms() {
                        forced::force_respawn(world, slave.guid);
                    }
                } else if flags.contains(LinkFlags::DESPAWN_ON_RESPAWN) && slave.alive {
                    forced::force_despawn(world, slave.guid);
                }
                if flags.contains(LinkFlags::FOLLOW) && slave.alive && !slave.in_combat {
                    set_following(world, slave, master);
                }
            }
            LinkEvent::Despawn => {
                if flags.contains(LinkFlags::DESPAWN_ON_DESPAWN) {
                    forced::force_despawn(world, slave.guid);
                }
            }
        }
    }

    /// Whether a linked creature may spawn now: some only spawn while their
    /// master is dead, others only while it is alive.
    pub fn can_spawn(&self, world: &World, guid: ObjectGuid) -> bool {
        let Some(me) = linked(world, guid) else {
            return true;
        };
        let restriction = LinkFlags::CANT_SPAWN_IF_BOSS_DEAD | LinkFlags::CANT_SPAWN_IF_BOSS_ALIVE;
        let link_flags = if me.spawn_id != 0 {
            self.manager.master_of_spawn(me.spawn_id).map(|l| l.flags)
        } else {
            None
        }
        .or_else(|| self.manager.master_of_entry(me.entry, me.map_id).map(|l| l.flags));
        let Some(flags) = link_flags.filter(|f| f.intersects(restriction)) else {
            return true;
        };
        let master_alive = self
            .find_master(world, &me, restriction)
            .is_some_and(|m| m.alive);
        if flags.contains(LinkFlags::CANT_SPAWN_IF_BOSS_DEAD) && !master_alive {
            return false;
        }
        if flags.contains(LinkFlags::CANT_SPAWN_IF_BOSS_ALIVE) && master_alive {
            return false;
        }
        true
    }
}

/// Make `who` follow `whom`, keeping the offset between their spawn points.
fn set_following(world: &World, who: &Linked, whom: &Linked) {
    let (dx, dy, dz) = (
        who.home.x - whom.home.x,
        who.home.y - whom.home.y,
        who.home.z - whom.home.z,
    );
    let distance = ((dx * dx + dy * dy + dz * dz).sqrt() - 1.0).max(0.0);
    let mut angle = dy.atan2(dx) - whom.home.o;
    if angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    let target = whom.guid;
    world.managers.creature_mgr.with_creature_mut(who.guid, |c| {
        let position = c.position;
        let speed = c.walk_speed();
        c.motion_master
            .move_follow(target, distance, angle, who.guid, position, speed);
        c.following_target = Some(target);
    });
}
