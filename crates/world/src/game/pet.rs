//! Runtime ownership and lifecycle for controlled player pets.

use crate::game::creature::CreatureManager;
use crate::game::player::PlayerManager;
use crate::World;
use anyhow::{bail, Result};
use oxcore_shared::protocol::ObjectGuid;
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

const PET_FOLLOW_DISTANCE: f32 = 2.0;

pub struct PetSystem {
    player_mgr: Arc<PlayerManager>,
    creature_mgr: Arc<CreatureManager>,
}

impl PetSystem {
    pub fn new(player_mgr: Arc<PlayerManager>, creature_mgr: Arc<CreatureManager>) -> Self {
        Self {
            player_mgr,
            creature_mgr,
        }
    }

    /// Summon one transient pet and replace any existing active pet.
    pub fn summon(&self, owner_guid: ObjectGuid, entry: u32, world: &World) -> Result<ObjectGuid> {
        let (position, map_id, instance_id, phase_mask, faction) = self
            .player_mgr
            .with_player(owner_guid, |player| {
                (
                    player.movement.position,
                    player.map_id,
                    player.instance_id,
                    player.phase_mask,
                    player.faction_template(),
                )
            })
            .ok_or_else(|| anyhow::anyhow!("pet owner {owner_guid:?} is not online"))?;
        if self.creature_mgr.get_template(entry).is_none() {
            bail!("cannot summon pet with unknown creature entry {entry}");
        }

        self.dismiss(owner_guid, world);
        let pet_guid = self
            .creature_mgr
            .spawn_pet(
                entry,
                owner_guid,
                position,
                map_id,
                instance_id,
                phase_mask,
                faction,
            )
            .expect("template was checked before spawning pet");
        self.creature_mgr.with_creature_mut(pet_guid, |pet| {
            pet.motion_master.move_follow(
                owner_guid,
                PET_FOLLOW_DISTANCE,
                FRAC_PI_2,
                pet.guid,
                pet.position,
                pet.walk_speed(),
            );
            pet.following_target = Some(owner_guid);
        });
        self.player_mgr
            .with_player_mut(owner_guid, |player| player.active_pet = Some(pet_guid));
        world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id)
            .add_creature(pet_guid, position);
        Ok(pet_guid)
    }

    /// Remove the owner's active runtime pet. Safe to call repeatedly.
    pub fn dismiss(&self, owner_guid: ObjectGuid, world: &World) -> Option<ObjectGuid> {
        let pet_guid = self
            .player_mgr
            .with_player_mut(owner_guid, |player| player.active_pet.take())??;
        let (_, pet) = self.creature_mgr.remove_creature(pet_guid)?;
        if pet.owner_guid == Some(owner_guid) {
            if let Some(map) = world.managers.map_mgr.get_map(pet.map_id, pet.instance_id) {
                map.remove_creature(pet_guid, pet.position);
            }
        }
        Some(pet_guid)
    }

    pub fn on_player_logout(&self, owner_guid: ObjectGuid, world: &World) {
        self.dismiss(owner_guid, world);
    }
}
