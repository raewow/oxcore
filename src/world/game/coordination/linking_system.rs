use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerTrait, BroadcastManagerExt};
use crate::world::game::creature::death::DeathState;
use crate::world::World;
use super::link_flags::LinkEvent;
use super::linking_manager::LinkingManager;
use std::sync::Arc;

/// LinkingSystem - coordinates event propagation between linked creatures
pub struct LinkingSystem {
    manager: Arc<LinkingManager>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl LinkingSystem {
    pub fn new(manager: Arc<LinkingManager>, broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { manager, broadcast_mgr }
    }

    /// Process linking events for a creature
    pub async fn process_event(
        &self,
        creature_guid: ObjectGuid,
        event: LinkEvent,
        world: &World,
    ) -> anyhow::Result<()> {
        let Some(spawn_id) = world.managers.creature_mgr.get_spawn_id(creature_guid) else {
            return Ok(());
        };

        // Propagate to slaves
        for (slave_guid, event) in self.manager.propagate_event_to_slaves(spawn_id, event) {
            self.apply_event(slave_guid, event, world);
        }

        // Propagate to masters (reverse direction)
        for (master_guid, event) in self.manager.propagate_event_to_masters(spawn_id, event) {
            self.apply_event(master_guid, event, world);
        }

        Ok(())
    }

    /// Apply a link event to a creature
    async fn apply_event(&self, guid: ObjectGuid, event: LinkEvent, world: &World) {
        match event {
            LinkEvent::Aggro { target } | LinkEvent::EnterCombat { target } => {
                // Make creature aggro the target
                // TODO: Integrate with AI system when available
                tracing::debug!("[LINKING] Creature {:?} forced to aggro {:?}", guid, target);
            }
            LinkEvent::Death => {
                // Kill the creature
                world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                    creature.current_health = 0;
                });
                // TODO: Integrate with death system
                tracing::debug!("[LINKING] Creature {:?} killed by master death", guid);
            }
            LinkEvent::Evade | LinkEvent::LeaveCombat => {
                // Make creature evade
                // TODO: Integrate with AI system when available
                tracing::debug!("[LINKING] Creature {:?} forced to evade", guid);
            }
            LinkEvent::Respawn => {
                // Schedule immediate respawn
                world.managers.creature_mgr.with_creature_mut(guid, |creature| {
                    if creature.death_state == DeathState::Dead {
                        creature.respawn_time = 0; // Immediate respawn
                    }
                });
                tracing::debug!("[LINKING] Creature {:?} scheduled for respawn", guid);
            }
        }
    }
}
