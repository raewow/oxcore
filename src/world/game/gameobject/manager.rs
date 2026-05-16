//! GameObjectManager - owns all gameobjects and templates

use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::gameobject::{GameObject, GameObjectTemplate};
use super::spawn::GameObjectSpawnData;
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::core::common::compress_update_packet_if_needed;
use crate::world::map::grid_coords::world_to_grid;

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

    /// Remove a gameobject
    pub fn remove_gameobject(&self, guid: ObjectGuid) -> Option<(ObjectGuid, GameObject)> {
        if let Some(spawn_id) = self.guid_to_spawn.remove(&guid) {
            if let Some(mut state) = self.spawn_states.get_mut(&spawn_id.1) {
                state.spawned = false;
            }
        }
        self.gameobjects.remove(&guid)
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

        let total: usize = self.spawns_by_map.iter().map(|e| e.value().len()).sum();
        tracing::info!(
            "Loaded {} gameobject spawns across {} maps (skipped {} for patch)",
            total,
            self.spawns_by_map.len(),
            skipped_patch
        );

        Ok(())
    }

    /// Get spawns for a specific grid on a map
    pub fn get_spawns_for_grid(
        &self,
        map_id: u32,
        grid_x: u8,
        grid_y: u8,
    ) -> Vec<GameObjectSpawnData> {
        let mut result = Vec::new();

        if let Some(spawns) = self.spawns_by_map.get(&map_id) {
            for spawn in spawns.iter() {
                let (spawn_gx, spawn_gy) = world_to_grid(spawn.position.x, spawn.position.y);
                if spawn_gx == grid_x && spawn_gy == grid_y {
                    result.push(spawn.clone());
                }
            }
        }

        result
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

        let gameobject = GameObject::new(
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
        _world: &crate::world::World,
    ) -> Option<crate::shared::messages::update::SmsgUpdateObject> {
        use crate::shared::messages::update::*;
        use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
        use crate::world::core::common::position::Position as WorldPosition;
        use crate::world::game::common::object_type::ObjectTypeId;
        use crate::world::game::common::update_fields::*;

        let go = self.gameobjects.get(&guid)?;

        let world_guid = WorldObjectGuid::new_gameobject(go.entry, guid.counter());
        let world_position =
            WorldPosition::new(go.position.x, go.position.y, go.position.z, go.position.o);

        // OBJECT_FIELD_TYPE: TYPEMASK_OBJECT | TYPEMASK_GAMEOBJECT = 0x01 | 0x20 = 0x21
        let type_mask: u32 = 0x21;

        let block =
            CreateObjectBlock::new(world_guid, ObjectTypeId::GameObject, ObjectType::GameObject)
                .with_position(world_position)
                .add_flags(crate::world::game::common::object_type::update_flags::UPDATEFLAG_ALL)
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
                .set_field(GAMEOBJECT_DYN_FLAGS, 0)
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

    /// Build SMSG_GAMEOBJECT_QUERY_RESPONSE packet for a gameobject entry
    pub fn build_gameobject_query_packet(
        &self,
        entry: u32,
    ) -> Option<crate::shared::protocol::WorldPacket> {
        use crate::shared::protocol::Opcode;

        let template = self.templates.get(&entry)?;

        let mut packet =
            crate::shared::protocol::WorldPacket::new(Opcode::SMSG_GAMEOBJECT_QUERY_RESPONSE);
        packet.write_u32(entry);
        packet.write_u32(template.go_type);
        packet.write_u32(template.display_id);
        packet.write_cstring(&template.name);
        packet.write_u8(0); // name2
        packet.write_u8(0); // name3
        packet.write_u8(0); // name4
        packet.write_cstring(&template.icon_name);
        // 1.12.1: No castBarCaption, no unk1 string, no size float
        // (castBarCaption/unk1 were added in 2.x, size is "not in Zero")
        // data fields (24 ints - client reads all 24 in 1.12.1)
        for i in 0..24 {
            packet.write_i32(template.data[i]);
        }

        Some(packet)
    }

    /// Send nearby gameobjects to a player (called during login)
    pub fn send_nearby_gameobjects(
        &self,
        player_guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
        world: &crate::world::World,
    ) -> anyhow::Result<()> {
        use crate::shared::messages::update::SmsgUpdateObject;
        use crate::shared::messages::ToWorldPacket;

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
            if let Some(msg) = self.build_create_msg(*guid, world) {
                for block in msg.blocks {
                    if count >= MAX_BLOCKS_PER_PACKET {
                        let packet = current_msg.to_world_packet();
                        let compressed = compress_update_packet_if_needed(packet)?;
                        world
                            .managers
                            .broadcast_mgr
                            .send_to_player(player_guid, compressed);
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
            let packet = current_msg.to_world_packet();
            let compressed = compress_update_packet_if_needed(packet)?;
            world
                .managers
                .broadcast_mgr
                .send_to_player(player_guid, compressed);
            total_sent += count;
        }

        // Send query responses proactively
        let mut unique_entries = std::collections::HashSet::new();
        for guid in &gameobjects {
            if let Some(go) = self.gameobjects.get(guid) {
                unique_entries.insert(go.entry);
            }
        }

        for entry in unique_entries {
            if let Some(query_packet) = self.build_gameobject_query_packet(entry) {
                world
                    .managers
                    .broadcast_mgr
                    .send_to_player(player_guid, query_packet);
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
