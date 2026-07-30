//! Spawn pool integration with the world (SpawnPool, DespawnPool and UpdatePool,
//! driven from grid loading and respawns).
//!
//! Pools decide *which* of their candidate spawns exist; the grid system is
//! what actually instantiates them, so pooled objects follow the same lazy
//! loading rules — and the same `instance_id` — as any other spawn. Objects a
//! pool swaps in while their grid is already loaded are spawned here directly.

use super::pool_manager::{PoolDespawn, PoolManager, PoolRoll};
use super::pool_types::{PoolMemberKey, PoolMemberType};
use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
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

    pub fn manager(&self) -> &Arc<PoolManager> {
        &self.manager
    }

    /// Roll the initial roster of every auto-spawn pool.
    ///
    /// Nothing is instantiated here: the selected members spawn when their
    /// grid is loaded, on whichever map/instance they belong to.
    pub fn initialize(&self) -> usize {
        self.manager.initialize_rosters()
    }

    /// May this creature spawn in this map instance?
    /// (pool roster check, called by grid loading)
    pub fn can_creature_spawn(&self, spawn_id: u32, instance_id: u32) -> bool {
        self.manager.can_creature_spawn(spawn_id, instance_id)
    }

    /// May this gameobject spawn in this map instance?
    /// (pool roster check, called by grid loading)
    pub fn can_gameobject_spawn(&self, spawn_id: u32, instance_id: u32) -> bool {
        self.manager.can_gameobject_spawn(spawn_id, instance_id)
    }

    /// Register a creature the grid system just spawned.
    pub fn on_creature_spawned(&self, spawn_id: u32, instance_id: u32, guid: ObjectGuid) {
        if let Some(pool_id) = self.manager.get_pool_for_creature(spawn_id) {
            self.manager.mark_spawned(
                pool_id,
                instance_id,
                (PoolMemberType::Creature, spawn_id),
                guid,
            );
        }
    }

    /// Register a gameobject the grid system just spawned.
    pub fn on_gameobject_spawned(&self, spawn_id: u32, instance_id: u32, guid: ObjectGuid) {
        if let Some(pool_id) = self.manager.get_pool_for_gameobject(spawn_id) {
            self.manager.mark_spawned(
                pool_id,
                instance_id,
                (PoolMemberType::GameObject, spawn_id),
                guid,
            );
        }
    }

    /// A pooled object left the world (grid unload). It keeps its roster slot,
    /// so it comes back when the grid is loaded again.
    pub fn on_object_despawned(&self, guid: ObjectGuid) {
        self.manager.mark_despawned(guid);
    }

    /// A pooled creature is ready to respawn: re-roll its pool.
    ///
    /// Returns true when this very creature was rolled again and should
    /// respawn normally; false when the pool replaced it with another member
    /// (this creature is despawned instead).
    pub fn on_creature_respawn(&self, guid: ObjectGuid, world: &World) -> bool {
        let Some((pool_id, instance_id, key)) = self.manager.get_pool_membership(guid) else {
            return true; // Not in a pool: normal respawn
        };

        let roll = self.manager.update_pool(pool_id, instance_id, key);
        if roll.trigger_respawns {
            return true;
        }

        tracing::debug!(
            "[POOL] Pool {} replaced member {:?} on respawn in instance {} ({} out, {} in)",
            pool_id,
            key,
            instance_id,
            roll.to_despawn.len(),
            roll.to_spawn.len()
        );

        self.apply_roll(&roll, instance_id, world);
        false
    }

    /// Roll a pool's roster back to full and spawn every member whose grid is
    /// already loaded. For scripts and GM commands.
    pub fn spawn_pool(&self, pool_id: u32, instance_id: u32, world: &World) -> Vec<ObjectGuid> {
        self.manager.fill_roster(pool_id, instance_id);

        self.manager
            .selected_objects(pool_id, instance_id)
            .into_iter()
            .filter_map(|key| {
                let owner = self.pool_of(key)?;
                self.spawn_member(owner, key, instance_id, world)
            })
            .collect()
    }

    /// Clear a pool's roster and remove its objects.
    pub fn despawn_pool(&self, pool_id: u32, instance_id: u32, world: &World) {
        for despawn in self.manager.clear_roster(pool_id, instance_id) {
            self.despawn_member(&despawn, instance_id, world);
        }
    }

    /// Apply a re-roll: despawn the members that left, spawn the ones that
    /// joined (as far as their grids are loaded).
    fn apply_roll(&self, roll: &PoolRoll, instance_id: u32, world: &World) {
        for despawn in &roll.to_despawn {
            self.despawn_member(despawn, instance_id, world);
        }

        for key in &roll.to_spawn {
            if let Some(pool_id) = self.pool_of(*key) {
                self.spawn_member(pool_id, *key, instance_id, world);
            }
        }
    }

    /// Pool a member belongs to.
    fn pool_of(&self, key: PoolMemberKey) -> Option<u32> {
        match key.0 {
            PoolMemberType::Creature => self.manager.get_pool_for_creature(key.1),
            PoolMemberType::GameObject => self.manager.get_pool_for_gameobject(key.1),
            PoolMemberType::Pool => Some(key.1),
        }
    }

    /// Instantiate one roster member, if its grid is loaded.
    fn spawn_member(
        &self,
        pool_id: u32,
        key: PoolMemberKey,
        instance_id: u32,
        world: &World,
    ) -> Option<ObjectGuid> {
        match key.0 {
            PoolMemberType::Creature => self.spawn_creature(pool_id, key.1, instance_id, world),
            PoolMemberType::GameObject => self.spawn_gameobject(pool_id, key.1, instance_id, world),
            PoolMemberType::Pool => None,
        }
    }

    fn spawn_creature(
        &self,
        pool_id: u32,
        spawn_id: u32,
        instance_id: u32,
        world: &World,
    ) -> Option<ObjectGuid> {
        if world.managers.creature_mgr.has_spawn(spawn_id) {
            return None; // Already in the world
        }

        let spawn = world.managers.creature_mgr.get_spawn_data_by_id(spawn_id)?;

        // Not loaded yet: the grid system spawns it when players come near.
        if !grid_is_loaded(world, spawn.map_id, instance_id, spawn.position) {
            return None;
        }

        let map = world
            .managers
            .map_mgr
            .get_or_create_map(spawn.map_id, instance_id);
        let guid = world.managers.creature_mgr.spawn_into_map(
            &spawn,
            &map,
            Some(&world.managers.waypoint_mgr),
        )?;

        self.manager.mark_spawned(
            pool_id,
            instance_id,
            (PoolMemberType::Creature, spawn_id),
            guid,
        );

        if let Some(create_msg) = world.managers.creature_mgr.build_create_msg(guid, world) {
            for player_guid in players_near(world, spawn.map_id, instance_id, spawn.position) {
                self.broadcast_mgr
                    .send_msg_to_player(player_guid, create_msg.clone());
            }
        }

        tracing::debug!(
            "[POOL] Pool {} spawned creature {} ({:?})",
            pool_id,
            spawn_id,
            guid
        );
        Some(guid)
    }

    fn spawn_gameobject(
        &self,
        pool_id: u32,
        spawn_id: u32,
        instance_id: u32,
        world: &World,
    ) -> Option<ObjectGuid> {
        if world.managers.gameobject_mgr.has_spawn(spawn_id) {
            return None;
        }

        let spawn = world
            .managers
            .gameobject_mgr
            .get_spawn_data_by_id(spawn_id)?;

        if !grid_is_loaded(world, spawn.map_id, instance_id, spawn.position) {
            return None;
        }

        let map = world
            .managers
            .map_mgr
            .get_or_create_map(spawn.map_id, instance_id);
        let guid = world.managers.gameobject_mgr.spawn_gameobject(&spawn)?;

        map.add_gameobject(guid, spawn.position);

        world
            .managers
            .gameobject_mgr
            .add_collision_model(guid, &world.managers.vmap_mgr);

        self.manager.mark_spawned(
            pool_id,
            instance_id,
            (PoolMemberType::GameObject, spawn_id),
            guid,
        );

        for player_guid in players_near(world, spawn.map_id, instance_id, spawn.position) {
            if let Some(create_msg) =
                world
                    .managers
                    .gameobject_mgr
                    .build_create_msg(guid, player_guid, world)
            {
                self.broadcast_mgr
                    .send_msg_to_player(player_guid, create_msg);
            }
        }

        tracing::debug!(
            "[POOL] Pool {} spawned gameobject {} ({:?})",
            pool_id,
            spawn_id,
            guid
        );
        Some(guid)
    }

    /// Remove one member from the world; members whose grid is not loaded have
    /// no object and only need their roster slot cleared (already done).
    fn despawn_member(&self, despawn: &PoolDespawn, instance_id: u32, world: &World) {
        let Some(guid) = despawn.guid else {
            return; // Not instantiated (grid not loaded)
        };

        match despawn.key.0 {
            PoolMemberType::Creature => self.despawn_creature(guid, instance_id, world),
            PoolMemberType::GameObject => self.despawn_gameobject(guid, instance_id, world),
            PoolMemberType::Pool => {}
        }
    }

    fn despawn_creature(&self, guid: ObjectGuid, instance_id: u32, world: &World) {
        let Some((position, map_id)) = world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.position, c.map_id))
        else {
            return;
        };

        for player_guid in world.session_mgr.get_all_sessions() {
            world.systems.visibility.remove_visible(player_guid, guid);
        }

        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        map.remove_creature(guid, position);

        self.send_destroy(guid, map_id, instance_id, position, world);

        world.systems.loot.remove_loot(guid);
        world.managers.creature_mgr.remove_creature(guid);
    }

    fn despawn_gameobject(&self, guid: ObjectGuid, instance_id: u32, world: &World) {
        let Some(position) = world.managers.gameobject_mgr.get_position(guid) else {
            return;
        };
        let map_id = world
            .managers
            .gameobject_mgr
            .with_gameobject(guid, |go| go.map_id)
            .unwrap_or(0);

        for player_guid in world.session_mgr.get_all_sessions() {
            world.systems.visibility.remove_visible(player_guid, guid);
        }

        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        map.remove_gameobject(guid, position);

        self.send_destroy(guid, map_id, instance_id, position, world);

        // Collision is looked up through the object, so drop it before the
        // object itself.
        world
            .managers
            .gameobject_mgr
            .remove_collision_model(guid, &world.managers.vmap_mgr);
        world.managers.gameobject_mgr.remove_gameobject(guid);
    }

    fn send_destroy(
        &self,
        guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        position: Position,
        world: &World,
    ) {
        let mut packet = WorldPacket::new(Opcode::SMSG_DESTROY_OBJECT);
        packet.write_guid_raw(guid.raw());

        let players = players_near(world, map_id, instance_id, position);
        self.broadcast_mgr.broadcast_to_players(&players, &packet);
    }
}

/// Is the grid covering `position` loaded on this map?
fn grid_is_loaded(world: &World, map_id: u32, instance_id: u32, position: Position) -> bool {
    use crate::map::grid_coords::world_to_grid;

    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return false;
    };

    let (grid_x, grid_y) = world_to_grid(position.x, position.y);
    let grid_mgr = map.grid_manager();
    let grid_mgr = grid_mgr.read();
    grid_mgr
        .get_grid_state(grid_x, grid_y)
        .map(|state| state.is_loaded())
        .unwrap_or(false)
}

/// Players within visibility range of a position.
fn players_near(
    world: &World,
    map_id: u32,
    instance_id: u32,
    position: Position,
) -> Vec<ObjectGuid> {
    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return Vec::new();
    };

    map.get_players_in_range(position, map.visibility_distance())
}
