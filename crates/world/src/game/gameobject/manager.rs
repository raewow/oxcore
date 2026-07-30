//! GameObjectManager - owns all gameobjects and templates

use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::gameobject::{GameObject, GameObjectTemplate};
use super::spawn::GameObjectSpawnData;
use super::types::{GOState, GameObjectType, LootState};
use crate::core::common::compress_update_packet_if_needed;
use crate::game::common::spawn_index::{build_grid_index, SpawnGridIndex};
use crate::map::grid_coords::world_to_grid;
use crate::map::pathfinding::vmap::{GameObjectModelSpawn, VMapManager};
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Tracks spawn state for grid loading
#[derive(Debug, Clone)]
struct SpawnState {
    /// Whether this spawn is currently active in the world
    spawned: bool,
}

/// Manages all gameobjects and their templates
pub struct GameObjectManager {
    /// Database pool for loading data
    world_db: Arc<MySqlPool>,
    /// Templates by entry ID
    templates: DashMap<u32, Arc<GameObjectTemplate>>,
    /// Active gameobjects by GUID
    gameobjects: DashMap<ObjectGuid, GameObject>,
    /// Spawn data by map_id -> list of spawns
    spawns_by_map: DashMap<u32, Vec<GameObjectSpawnData>>,
    /// map_id -> grid -> indices into `spawns_by_map[map_id]`. Built once by
    /// `load_spawns`; `spawns_by_map` is append-only afterwards.
    spawn_grid_index: DashMap<u32, SpawnGridIndex>,
    /// Track spawn states by spawn_id
    spawn_states: DashMap<u32, SpawnState>,
    /// Track which gameobject GUID belongs to which spawn_id
    guid_to_spawn: DashMap<ObjectGuid, u32>,
    /// GUID counter
    next_guid: AtomicU64,
    /// Current game patch for filtering spawns
    current_patch: std::sync::RwLock<u8>,
}

impl GameObjectManager {
    pub fn new(world_db: Arc<MySqlPool>) -> Self {
        Self {
            world_db,
            templates: DashMap::new(),
            gameobjects: DashMap::new(),
            spawns_by_map: DashMap::new(),
            spawn_grid_index: DashMap::new(),
            spawn_states: DashMap::new(),
            guid_to_spawn: DashMap::new(),
            next_guid: AtomicU64::new(1),
            current_patch: std::sync::RwLock::new(10),
        }
    }

    /// Set the current game patch for filtering spawns
    pub fn set_patch(&self, patch: u8) {
        if let Ok(mut guard) = self.current_patch.write() {
            *guard = patch;
        }
    }

    /// Get the current game patch
    pub fn get_patch(&self) -> u8 {
        self.current_patch.read().map(|g| *g).unwrap_or(10)
    }

    /// Get a template by entry
    pub fn get_template(&self, entry: u32) -> Option<Arc<GameObjectTemplate>> {
        self.templates.get(&entry).map(|r| Arc::clone(&r))
    }

    /// Get a gameobject by GUID
    pub fn get_gameobject(
        &self,
        guid: ObjectGuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, ObjectGuid, GameObject>> {
        self.gameobjects.get(&guid)
    }

    /// Iterate over all active gameobjects.
    pub fn iter_gameobjects(&self) -> dashmap::iter::Iter<'_, ObjectGuid, GameObject> {
        self.gameobjects.iter()
    }

    #[cfg(test)]
    pub fn add_template_for_test(&self, template: GameObjectTemplate) {
        self.templates.insert(template.entry, Arc::new(template));
    }

    #[cfg(test)]
    pub fn add_gameobject_for_test(&self, gameobject: GameObject) {
        self.gameobjects.insert(gameobject.guid, gameobject);
    }

    /// Batched immutable access
    pub fn with_gameobject<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&GameObject) -> R,
    {
        self.gameobjects.get(&guid).map(|go| f(&*go))
    }

    /// Batched mutable access
    pub fn with_gameobject_mut<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut GameObject) -> R,
    {
        self.gameobjects.get_mut(&guid).map(|mut go| f(&mut *go))
    }

    /// Get gameobject position
    pub fn get_position(&self, guid: ObjectGuid) -> Option<Position> {
        self.gameobjects.get(&guid).map(|go| go.position)
    }

    /// Get gameobject phase mask
    pub fn get_phase_mask(&self, guid: ObjectGuid) -> Option<u32> {
        self.gameobjects.get(&guid).map(|go| go.phase_mask)
    }

    /// Check if a spawn is already active
    pub fn has_spawn(&self, spawn_id: u32) -> bool {
        self.spawn_states
            .get(&spawn_id)
            .map(|s| s.spawned)
            .unwrap_or(false)
    }

    /// Get spawn data by spawn_id (for the pool system)
    pub fn get_spawn_data_by_id(&self, spawn_id: u32) -> Option<GameObjectSpawnData> {
        for map_spawns in self.spawns_by_map.iter() {
            if let Some(spawn) = map_spawns.value().iter().find(|s| s.spawn_id == spawn_id) {
                return Some(spawn.clone());
            }
        }
        None
    }

    /// Remove a gameobject
    pub fn remove_gameobject(&self, guid: ObjectGuid) -> Option<(ObjectGuid, GameObject)> {
        if let Some(spawn_id) = self.guid_to_spawn.remove(&guid) {
            if let Some(mut state) = self.spawn_states.get_mut(&spawn_id.1) {
                state.spawned = false;
            }
        }
        self.gameobjects.remove(&guid)
    }

    // ==================== VMap collision ====================

    /// Add a gameobject's collision model to the map's dynamic VMap tree.
    ///
    /// Called when the object enters the world, so line of sight, pathfinding,
    /// and height queries account for doors, gates, and bridges. Returns whether
    /// a model was inserted — most gameobjects have no collision geometry.
    ///
    /// Ported from `GameObject::AddToWorld` → `Map::InsertGameObjectModel`.
    pub fn add_collision_model(&self, guid: ObjectGuid, vmap_mgr: &VMapManager) -> bool {
        let Some((map_id, spawn, collides)) = self.with_gameobject(guid, |go| {
            (
                go.map_id,
                self.collision_model_spawn(go),
                Self::collision_enabled(go),
            )
        }) else {
            return false;
        };

        let Some(spawn) = spawn else {
            return false;
        };

        vmap_mgr.insert_gameobject_model(map_id, guid.raw(), spawn, collides)
    }

    /// Remove a gameobject's collision model from the dynamic VMap tree.
    ///
    /// Ported from `GameObject::RemoveFromWorld` → `Map::RemoveGameObjectModel`.
    pub fn remove_collision_model(&self, guid: ObjectGuid, vmap_mgr: &VMapManager) -> bool {
        // The object may already be gone from the map, so fall back to its GUID's
        // map id via the tracked entry when available.
        let map_id = self.with_gameobject(guid, |go| go.map_id);

        match map_id {
            Some(map_id) => vmap_mgr.remove_gameobject_model(map_id, guid.raw()),
            None => false,
        }
    }

    /// Sync a gameobject's collision with its current state.
    ///
    /// Call after changing `go_state` or `loot_state`: an open door must stop
    /// blocking line of sight, and a looted chest stops being solid.
    ///
    /// Ported from `GameObject::UpdateCollisionState`.
    pub fn update_collision_state(&self, guid: ObjectGuid, vmap_mgr: &VMapManager) -> bool {
        let Some((map_id, enabled)) =
            self.with_gameobject(guid, |go| (go.map_id, Self::collision_enabled(go)))
        else {
            return false;
        };

        vmap_mgr.set_gameobject_model_enabled(map_id, guid.raw(), enabled)
    }

    /// Rebuild a gameobject's collision model after its display id changed.
    ///
    /// Ported from `GameObject::UpdateModel`.
    pub fn update_collision_model(&self, guid: ObjectGuid, vmap_mgr: &VMapManager) -> bool {
        self.remove_collision_model(guid, vmap_mgr);
        self.add_collision_model(guid, vmap_mgr)
    }

    /// Whether this object should currently collide.
    ///
    /// Chests are solid until looted; everything else is solid while closed.
    fn collision_enabled(go: &GameObject) -> bool {
        match go.go_type {
            GameObjectType::Chest => go.loot_state == LootState::Ready,
            _ => go.go_state == GOState::Ready,
        }
    }

    /// Describe a gameobject's collision model, or `None` when it should not
    /// have one.
    ///
    /// Ported from `GameObjectModel::construct`: objects flagged line-of-sight
    /// safe and server-only objects (invisible triggers) get no collision.
    fn collision_model_spawn(&self, go: &GameObject) -> Option<GameObjectModelSpawn> {
        let template = self.get_template(go.entry)?;

        if template.is_los_ok() || template.is_server_only() {
            return None;
        }

        Some(GameObjectModelSpawn {
            display_id: go.display_id,
            position: go.position,
            scale: go.scale,
            always_break_los: template.can_always_break_los(),
        })
    }

    /// Load gameobject templates from database
    pub async fn load_templates(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, GameObjectTemplateRow>(
            r#"SELECT entry, `type`, displayId, name, icon, faction, flags, size,
                      data0, data1, data2, data3, data4, data5, data6, data7,
                      data8, data9, data10, data11, data12, data13, data14, data15,
                      data16, data17, data18, data19, data20, data21, data22, data23
               FROM gameobject_template
               WHERE patch = 0"#,
        )
        .fetch_all(&*self.world_db)
        .await
        .context("Failed to load gameobject templates")?;

        for row in rows {
            let template = GameObjectTemplate {
                entry: row.entry,
                go_type: row.go_type,
                display_id: row.display_id,
                name: row.name,
                icon_name: row.icon,
                cast_bar_caption: String::new(),
                faction: row.faction,
                flags: row.flags,
                size: row.size,
                data: [
                    row.data0, row.data1, row.data2, row.data3, row.data4, row.data5, row.data6,
                    row.data7, row.data8, row.data9, row.data10, row.data11, row.data12,
                    row.data13, row.data14, row.data15, row.data16, row.data17, row.data18,
                    row.data19, row.data20, row.data21, row.data22, row.data23,
                ],
            };
            self.templates.insert(template.entry, Arc::new(template));
        }

        tracing::info!("Loaded {} gameobject templates", self.templates.len());
        Ok(())
    }

    /// Load gameobject spawns from database
    pub async fn load_spawns(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, GameObjectSpawnRow>(
            r#"SELECT guid, id, map, position_x, position_y, position_z, orientation,
                      rotation0, rotation1, rotation2, rotation3,
                      spawntimesecsmin, animprogress, state,
                      patch_min, patch_max
               FROM gameobject"#,
        )
        .fetch_all(&*self.world_db)
        .await
        .context("Failed to load gameobject spawns")?;

        let current_patch = self.get_patch();
        let mut skipped_patch = 0;

        for row in rows {
            // Check patch compatibility
            if row.patch_min > current_patch || current_patch > row.patch_max {
                skipped_patch += 1;
                continue;
            }

            let spawn = GameObjectSpawnData {
                spawn_id: row.guid,
                entry: row.id,
                map_id: row.map,
                position: Position {
                    x: row.position_x,
                    y: row.position_y,
                    z: row.position_z,
                    o: row.orientation,
                },
                rotation0: row.rotation0,
                rotation1: row.rotation1,
                rotation2: row.rotation2,
                rotation3: row.rotation3,
                spawntimesecs: row.spawntimesecsmin as u32,
                animprogress: row.animprogress,
                state: row.state,
            };

            self.spawns_by_map
                .entry(spawn.map_id)
                .or_insert_with(Vec::new)
                .push(spawn);
        }

        self.rebuild_spawn_grid_index();

        let total: usize = self.spawns_by_map.iter().map(|e| e.value().len()).sum();
        tracing::info!(
            "Loaded {} gameobject spawns across {} maps (skipped {} for patch)",
            total,
            self.spawns_by_map.len(),
            skipped_patch
        );

        Ok(())
    }

    /// Rebuild the grid index from `spawns_by_map`. Call after any bulk change.
    fn rebuild_spawn_grid_index(&self) {
        self.spawn_grid_index.clear();
        for entry in self.spawns_by_map.iter() {
            let index = build_grid_index(entry.value(), |s| s.position);
            self.spawn_grid_index.insert(*entry.key(), index);
        }
    }

    /// Get spawns for a specific grid on a map
    pub fn get_spawns_for_grid(
        &self,
        map_id: u32,
        grid_x: u8,
        grid_y: u8,
    ) -> Vec<GameObjectSpawnData> {
        let Some(spawns) = self.spawns_by_map.get(&map_id) else {
            return Vec::new();
        };
        let Some(index) = self.spawn_grid_index.get(&map_id) else {
            // Index not built (spawns added outside load_spawns) — fall back to a
            // scan rather than silently spawning nothing.
            tracing::warn!("GameObject spawn grid index missing for map {}", map_id);
            return spawns
                .iter()
                .filter(|s| world_to_grid(s.position.x, s.position.y) == (grid_x, grid_y))
                .cloned()
                .collect();
        };

        debug_assert_eq!(
            index.values().map(Vec::len).sum::<usize>(),
            spawns.len(),
            "gameobject spawn grid index is stale for map {map_id}"
        );

        index
            .get(&(grid_x, grid_y))
            .map(|ids| {
                ids.iter()
                    .filter_map(|&i| spawns.get(i as usize).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Spawn a single gameobject from spawn data
    /// Returns the GUID of the spawned gameobject, or None if failed
    pub fn spawn_gameobject(&self, spawn: &GameObjectSpawnData) -> Option<ObjectGuid> {
        // Check if already spawned
        if self.has_spawn(spawn.spawn_id) {
            return None;
        }

        let template = self.get_template(spawn.entry)?;

        let counter = self.next_guid.fetch_add(1, Ordering::Relaxed);
        let guid = ObjectGuid::new_gameobject(spawn.entry, counter as u32);

        let mut gameobject = GameObject::new(
            guid,
            spawn.entry,
            spawn.spawn_id,
            spawn.position,
            spawn.map_id,
            &template,
            [
                spawn.rotation0,
                spawn.rotation1,
                spawn.rotation2,
                spawn.rotation3,
            ],
            spawn.state,
            spawn.animprogress,
        );
        gameobject.in_world = true;
        gameobject.spawntimesecs = spawn.spawntimesecs;

        self.gameobjects.insert(guid, gameobject);

        // Track spawn state
        self.spawn_states
            .insert(spawn.spawn_id, SpawnState { spawned: true });
        self.guid_to_spawn.insert(guid, spawn.spawn_id);

        Some(guid)
    }

    /// Build CREATE_OBJECT packet for a gameobject
    ///
    /// GameObjects use CreateObject (not CreateObject2) in the update packet.
    pub fn build_create_msg(
        &self,
        guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        world: &crate::World,
    ) -> Option<oxcore_shared::messages::update::SmsgUpdateObject> {
        use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
        use crate::core::common::position::Position as WorldPosition;
        use crate::game::common::object_type::ObjectTypeId;
        use crate::game::common::update_fields::*;
        use oxcore_shared::messages::update::*;

        let go = self.gameobjects.get(&guid)?;
        let quest_activate = go.go_type == GameObjectType::Chest
            && self.get_template(go.entry).is_some_and(|template| {
                world
                    .systems
                    .loot_manager
                    .player_needs_gameobject_quest_loot(template.data[1].max(0) as u32, |item_id| {
                        world
                            .systems
                            .quest
                            .player_has_quest_for_item(viewer_guid, item_id)
                    })
            });

        let world_guid = WorldObjectGuid::new_gameobject(go.entry, guid.counter());
        let world_position =
            WorldPosition::new(go.position.x, go.position.y, go.position.z, go.position.o);

        // OBJECT_FIELD_TYPE: TYPEMASK_OBJECT | TYPEMASK_GAMEOBJECT = 0x01 | 0x20 = 0x21
        let type_mask: u32 = 0x21;

        let block =
            CreateObjectBlock::new(world_guid, ObjectTypeId::GameObject, ObjectType::GameObject)
                .with_position(world_position)
                .add_flags(crate::game::common::object_type::update_flags::UPDATEFLAG_ALL)
                // Object fields
                .set_guid_field(OBJECT_FIELD_GUID, world_guid)
                .set_field(OBJECT_FIELD_TYPE, type_mask)
                .set_field(OBJECT_FIELD_ENTRY, go.entry)
                .set_float_field(OBJECT_FIELD_SCALE_X, go.scale)
                // GameObject fields
                .set_field(GAMEOBJECT_DISPLAYID, go.display_id)
                .set_field(GAMEOBJECT_FLAGS, go.flags)
                // Rotation (4 floats starting at GAMEOBJECT_ROTATION)
                .set_float_field(GAMEOBJECT_ROTATION, go.rotation[0])
                .set_float_field(GAMEOBJECT_ROTATION + 1, go.rotation[1])
                .set_float_field(GAMEOBJECT_ROTATION + 2, go.rotation[2])
                .set_float_field(GAMEOBJECT_ROTATION + 3, go.rotation[3])
                // State
                .set_field(GAMEOBJECT_STATE, go.go_state as u32)
                // Position in update fields (some clients read from here)
                .set_float_field(GAMEOBJECT_POS_X, go.position.x)
                .set_float_field(GAMEOBJECT_POS_Y, go.position.y)
                .set_float_field(GAMEOBJECT_POS_Z, go.position.z)
                .set_float_field(GAMEOBJECT_FACING, go.position.o)
                // Dynamic flags
                .set_field(
                    GAMEOBJECT_DYN_FLAGS,
                    if quest_activate {
                        crate::game::gameobject::types::go_dyn_flags::GO_DYNFLAG_LO_ACTIVATE
                    } else {
                        0
                    },
                )
                // Faction
                .set_field(GAMEOBJECT_FACTION, go.faction)
                // Type
                .set_field(GAMEOBJECT_TYPE_ID, go.go_type as u32)
                // Level
                .set_field(GAMEOBJECT_LEVEL, go.level)
                // Art/animation
                .set_field(GAMEOBJECT_ARTKIT, go.art_kit)
                .set_field(GAMEOBJECT_ANIMPROGRESS, go.anim_progress);

        Some(SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject(block)))
    }

    /// Look up a gameobject template for a query response.
    ///
    /// Returns the borrowed template rather than an encoded packet: the two protocols disagree
    /// about the body shape, so only the send path can encode it.
    pub fn gameobject_template_info(&self, entry: u32) -> Option<Arc<GameObjectTemplate>> {
        self.templates.get(&entry).map(|t| Arc::clone(&t))
    }

    /// Send nearby gameobjects to a player (called during login)
    pub fn send_nearby_gameobjects(
        &self,
        player_guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
        world: &crate::World,
    ) -> anyhow::Result<()> {
        use oxcore_shared::messages::update::SmsgUpdateObject;
        use oxcore_shared::messages::ToWorldPacket;

        const MAX_BLOCKS_PER_PACKET: usize = 50;

        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        let nearby = map.get_objects_in_range(position, map.visibility_distance());

        let gameobjects: Vec<_> = nearby.into_iter().filter(|g| g.is_game_object()).collect();

        if gameobjects.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "[GAMEOBJECT] Sending {} gameobjects to player {:?}",
            gameobjects.len(),
            player_guid
        );

        let mut current_msg = SmsgUpdateObject::new();
        let mut count = 0;
        let mut total_sent = 0;

        for guid in &gameobjects {
            if let Some(msg) = self.build_create_msg(*guid, player_guid, world) {
                for block in msg.blocks {
                    if count >= MAX_BLOCKS_PER_PACKET {
                        // Compression belongs to the vanilla encoding, so the recipient's
                        // broadcaster decides whether it applies.
                        send_update_to(world, player_guid, &current_msg)?;
                        total_sent += count;
                        current_msg = SmsgUpdateObject::new();
                        count = 0;
                    }
                    current_msg = current_msg.add_block(block);
                    count += 1;
                }
            }
        }

        if !current_msg.blocks.is_empty() {
            send_update_to(world, player_guid, &current_msg)?;
            total_sent += count;
        }

        // Send query responses proactively
        let mut unique_entries = std::collections::HashSet::new();
        for guid in &gameobjects {
            if let Some(go) = self.gameobjects.get(guid) {
                unique_entries.insert(go.entry);
            }
        }

        // Sent as messages so each client gets the body its protocol expects.
        for entry in unique_entries {
            if let Some(template) = self.gameobject_template_info(entry) {
                world.managers.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    oxcore_shared::messages::query::SmsgGameObjectQueryResponse {
                        entry,
                        guid: (0, 0),
                        template: Some(oxcore_shared::messages::query::GameObjectTemplateInfo {
                            go_type: template.go_type,
                            display_id: template.display_id,
                            name: &template.name,
                            icon_name: &template.icon_name,
                            data: &template.data,
                        }),
                    },
                );
            }
        }

        tracing::info!(
            "[GAMEOBJECT] Sent {} gameobject blocks to player {:?}",
            total_sent,
            player_guid
        );

        Ok(())
    }
}

// ============================================================
// Database row types (sqlx FromRow)
// ============================================================

#[derive(sqlx::FromRow, Debug)]
struct GameObjectTemplateRow {
    pub entry: u32,
    #[sqlx(rename = "type")]
    pub go_type: u32,
    #[sqlx(rename = "displayId")]
    pub display_id: u32,
    pub name: String,
    pub icon: String,
    pub faction: u32,
    pub flags: u32,
    pub size: f32,
    pub data0: i32,
    pub data1: i32,
    pub data2: i32,
    pub data3: i32,
    pub data4: i32,
    pub data5: i32,
    pub data6: i32,
    pub data7: i32,
    pub data8: i32,
    pub data9: i32,
    pub data10: i32,
    pub data11: i32,
    pub data12: i32,
    pub data13: i32,
    pub data14: i32,
    pub data15: i32,
    pub data16: i32,
    pub data17: i32,
    pub data18: i32,
    pub data19: i32,
    pub data20: i32,
    pub data21: i32,
    pub data22: i32,
    pub data23: i32,
}

#[derive(sqlx::FromRow, Debug)]
struct GameObjectSpawnRow {
    pub guid: u32,
    pub id: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub rotation0: f32,
    pub rotation1: f32,
    pub rotation2: f32,
    pub rotation3: f32,
    pub spawntimesecsmin: i32,
    pub animprogress: u8,
    pub state: u8,
    pub patch_min: u8,
    pub patch_max: u8,
}

/// Send one object update to a single player, letting their broadcaster pick the encoding.
///
/// A missing broadcaster is not an error: the player may have disconnected between the visibility
/// scan and the send.
fn send_update_to(
    world: &crate::World,
    player_guid: ObjectGuid,
    msg: &oxcore_shared::messages::update::SmsgUpdateObject,
) -> anyhow::Result<()> {
    if let Some(broadcaster) = world.managers.player_mgr.get_broadcaster(player_guid) {
        broadcaster.send_update_object(msg)?;
    }
    Ok(())
}
