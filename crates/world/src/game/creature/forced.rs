//! Lifecycle changes forced on a creature by links and scripts: dying on the
//! spot, despawning, and respawning without waiting for the timer.

use super::ai::{queue_event, AIEvent};
use super::death::DeathState;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// Kill a creature outright (no loot, no killer credit unless one is given).
pub fn force_kill(world: &World, guid: ObjectGuid, killer: Option<ObjectGuid>) {
    let alive = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| c.is_alive())
        .unwrap_or(false);
    if !alive {
        return;
    }
    if let Some(info) = world.managers.creature_mgr.handle_death(guid, killer) {
        world
            .systems
            .creature_movement
            .send_stop_packet(info.guid, info.position, world);
    }
    let _ = crate::handlers::creature_combat::send_creature_killed_update(world, guid);
    queue_event(world, guid, AIEvent::Died { killer_guid: killer });
}

/// Take a creature out of the world now. A summon is gone for good; a
/// database spawn dies without a corpse and respawns on its normal timer.
pub fn force_despawn(world: &World, guid: ObjectGuid) {
    let Some(transient) = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| c.spawn_id == 0)
    else {
        return;
    };
    if transient {
        super::summon::despawn_creature(world, guid);
        return;
    }
    force_kill(world, guid, None);
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        c.has_loot = false;
        if c.death_state == DeathState::Corpse {
            c.corpse_decay_timer = 0;
        }
    });
}

/// Bring a dead creature back at once: its corpse goes and the respawn timer
/// is cut to nothing.
pub fn force_respawn(world: &World, guid: ObjectGuid) {
    world.managers.creature_mgr.with_creature_mut(guid, |c| {
        if c.death_state.is_alive() {
            return;
        }
        if c.death_state == DeathState::Corpse {
            c.corpse_decay_timer = 0;
        }
        c.respawn_time = 0;
    });
}
