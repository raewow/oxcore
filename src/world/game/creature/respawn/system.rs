//! Respawn System - handles creature respawn lifecycle

use crate::shared::messages::update::SmsgUpdateObject;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::World;
use std::sync::Arc;

/// RespawnSystem - handles all respawn business logic and packet sending
pub struct RespawnSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl RespawnSystem {
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { broadcast_mgr }
    }

    /// Process respawns for all dead creatures
    pub async fn process_respawns(&self, world: &World) -> anyhow::Result<()> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Get creatures ready to respawn
        let ready_to_respawn: Vec<ObjectGuid> = world
            .managers
            .creature_mgr
            .iter_creatures()
            .filter(|e| e.should_respawn(current_time))
            .map(|e| *e.key())
            .collect();

        for guid in ready_to_respawn {
            self.respawn_creature(guid, world).await?;
        }

        Ok(())
    }

    /// Respawn a single creature
    async fn respawn_creature(&self, guid: ObjectGuid, world: &World) -> anyhow::Result<()> {
        // Get respawn info before modifying
        let respawn_info = world
            .managers
            .creature_mgr
            .with_creature_mut(guid, |creature| {
                let map_id = creature.map_id;
                let instance_id = creature.instance_id;
                let position = creature.home_position;
                let entry = creature.entry;

                // Reset creature state
                creature.respawn();

                (map_id, instance_id, position, entry)
            });

        let Some((map_id, instance_id, position, entry)) = respawn_info else {
            return Ok(());
        };

        tracing::info!(
            "[RESPAWN] Creature {:?} (entry {}) respawning at ({:.1}, {:.1}, {:.1})",
            guid,
            entry,
            position.x,
            position.y,
            position.z
        );

        // Re-register with grid
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        map.add_creature(guid, position);

        // Send CREATE_OBJECT to nearby players
        self.send_creature_spawn(guid, map_id, instance_id, position, world);

        Ok(())
    }

    /// Send CREATE_OBJECT to players near the spawn point
    fn send_creature_spawn(
        &self,
        creature_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        position: Position,
        world: &World,
    ) {
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        let visibility_distance = map.visibility_distance();

        // Get nearby players
        let nearby_players: Vec<ObjectGuid> = map
            .get_objects_in_range(position, visibility_distance)
            .into_iter()
            .filter(|g| g.is_player())
            .collect();

        if nearby_players.is_empty() {
            return;
        }

        // Build create message using CreatureManager
        let Some(create_msg) = world
            .managers
            .creature_mgr
            .build_create_msg(creature_guid, world)
        else {
            return;
        };

        // Send to each nearby player via broadcast_mgr
        for player_guid in nearby_players {
            self.broadcast_mgr
                .send_msg_to_player(player_guid, create_msg.clone());
        }
    }
}
