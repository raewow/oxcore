use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerTrait, BroadcastManagerExt};
use crate::world::World;
use super::addon::CreatureAddon;
use super::manager::AddonManager;
use std::sync::Arc;

/// AddonSystem - coordinates addon application and updates
pub struct AddonSystem {
    manager: Arc<AddonManager>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl AddonSystem {
    pub fn new(manager: Arc<AddonManager>, broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { manager, broadcast_mgr }
    }

    /// Apply addon to a creature on spawn
    pub fn apply_addon(&self, creature_guid: ObjectGuid, world: &World) -> anyhow::Result<()> {
        let (spawn_id, entry) = world.managers.creature_mgr
            .with_creature_mut(creature_guid, |c| (c.spawn_id, c.entry))
            .ok_or_else(|| anyhow::anyhow!("Creature not found"))?;

        let Some(addon) = self.manager.get_addon(spawn_id, entry) else {
            return Ok(());
        };

        world.managers.creature_mgr.with_creature_mut(creature_guid, |creature| {
            // Apply mount
            if addon.has_mount() {
                // TODO: Set mount_display_id field when it's added to Creature struct
                tracing::debug!("[ADDON] Would apply mount {} to {:?}", addon.mount, creature_guid);
            }

            // Apply stand state
            let stand_state = addon.stand_state();
            if stand_state > 0 {
                // TODO: Set stand_state field when it's added to Creature struct
                tracing::debug!("[ADDON] Would apply stand_state {} to {:?}", stand_state, creature_guid);
            }

            // Apply sheath state
            let sheath_state = addon.sheath_state();
            if sheath_state > 0 {
                // TODO: Set sheath_state field when it's added to Creature struct
                tracing::debug!("[ADDON] Would apply sheath_state {} to {:?}", sheath_state, creature_guid);
            }

            // Apply emote
            if addon.has_emote() {
                // TODO: Set emote_state field when it's added to Creature struct
                tracing::debug!("[ADDON] Would apply emote {} to {:?}", addon.emote, creature_guid);
            }

            // Store auras for later application
            if !addon.auras.is_empty() {
                // TODO: Apply auras when aura system is implemented
                tracing::debug!("[ADDON] Would apply {} auras to {:?}", addon.auras.len(), creature_guid);
            }
        });

        Ok(())
    }

    /// Dynamically change mount (send update packet)
    pub async fn set_mount(&self, creature_guid: ObjectGuid, mount_id: u32, world: &World) -> anyhow::Result<()> {
        world.managers.creature_mgr.with_creature_mut(creature_guid, |_creature| {
            // TODO: Set mount_display_id field and send update packet
            tracing::debug!("[ADDON] Would set mount {} on {:?}", mount_id, creature_guid);
        });

        // TODO: Send SMSG_UPDATE_OBJECT packet with mount field change
        // let msg = SmsgCreatureDisplayUpdate {
        //     guid: creature_guid,
        //     mount_display_id: mount_id,
        // };
        // let packet = msg.to_world_packet();
        // self.broadcast_mgr.broadcast_nearby(creature_guid, &packet, false);

        Ok(())
    }

    /// Dynamically change stand state
    pub async fn set_stand_state(&self, creature_guid: ObjectGuid, stand_state: u8, world: &World) -> anyhow::Result<()> {
        world.managers.creature_mgr.with_creature_mut(creature_guid, |_creature| {
            // TODO: Set stand_state field and send update packet
            tracing::debug!("[ADDON] Would set stand_state {} on {:?}", stand_state, creature_guid);
        });

        // TODO: Send update packet (UNIT_FIELD_BYTES_1)

        Ok(())
    }
}
