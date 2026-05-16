use super::pool_manager::PoolManager;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::World;
use std::sync::Arc;

/// PoolSystem - coordinates pool spawning and replacement
pub struct PoolSystem {
    manager: Arc<PoolManager>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl PoolSystem {
    pub fn new(manager: Arc<PoolManager>, broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self {
            manager,
            broadcast_mgr,
        }
    }

    /// Spawn a pool (select and spawn members up to limit)
    pub async fn spawn_pool(&self, pool_id: u32, world: &World) -> anyhow::Result<Vec<ObjectGuid>> {
        let spawn_ids = self.manager.select_spawn_members(pool_id);
        let mut spawned = Vec::new();

        for spawn_id in spawn_ids {
            if let Some(spawn_data) = world.managers.creature_mgr.get_spawn_data_by_id(spawn_id) {
                // Pools are spawned on continents (instance_id = 0)
                // TODO: If pools should work in instances, get instance_id from context
                if let Some(guid) = world.managers.creature_mgr.spawn_creature(&spawn_data, 0) {
                    self.manager.mark_spawned(pool_id, guid, spawn_id);
                    spawned.push(guid);
                }
            }
        }

        tracing::debug!(
            "[POOL] Spawned {} members for pool {}",
            spawned.len(),
            pool_id
        );
        Ok(spawned)
    }

    /// Handle creature death in pool (spawn replacement if needed)
    pub async fn on_creature_death(&self, guid: ObjectGuid, world: &World) -> anyhow::Result<()> {
        let Some((pool_id, spawn_id)) = self.manager.get_pool_membership(guid) else {
            return Ok(()); // Not in a pool
        };

        // Mark as despawned
        self.manager.mark_despawned(pool_id, guid);

        // Select replacement member
        if let Some(replacement_id) = self.manager.select_replacement(pool_id) {
            if let Some(spawn_data) = world
                .managers
                .creature_mgr
                .get_spawn_data_by_id(replacement_id)
            {
                // Pools are spawned on continents (instance_id = 0)
                if let Some(new_guid) = world.managers.creature_mgr.spawn_creature(&spawn_data, 0) {
                    self.manager.mark_spawned(pool_id, new_guid, replacement_id);

                    tracing::debug!(
                        "[POOL] Replaced creature in pool {} (old spawn {}, new spawn {})",
                        pool_id,
                        spawn_id,
                        replacement_id
                    );
                }
            }
        }

        Ok(())
    }

    /// Check if creature can spawn (pool allows it)
    pub fn can_creature_spawn(&self, spawn_id: u32) -> bool {
        self.manager.can_creature_spawn(spawn_id)
    }
}
