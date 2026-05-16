//! Lua instance script handler.
//!
//! Handles calling Lua callbacks for instance scripts (OnCreatureCreate, OnCreatureDeath, etc.)
//! and parsing the returned action tables.
//!
//! Instance scripts manage encounter state (boss kills, door states, etc.)
//! using SetData/GetData pattern from MaNGOS.

use super::super::super::common::ObjectGuid;
use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::{InstanceSnapshot, LuaGuid};
use mlua::{Function, Lua, Table, Value};
use std::collections::HashMap;

/// Lua instance script handler.
///
/// Wraps a Lua script table registered via RegisterInstanceScript(map_id, table).
pub struct LuaInstanceAI {
    map_id: u32,
}

impl LuaInstanceAI {
    pub fn new(map_id: u32) -> Self {
        Self { map_id }
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    /// Call OnCreatureCreate callback - when a creature spawns in the instance.
    pub fn on_creature_create(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnCreatureCreate", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnCreatureCreate")?;
            func.call((table.clone(), input, creature_entry, LuaGuid(creature_guid)))
        })
    }

    /// Call OnCreatureDeath callback - when a creature dies in the instance.
    pub fn on_creature_death(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnCreatureDeath", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnCreatureDeath")?;
            func.call((table.clone(), input, creature_entry, LuaGuid(creature_guid)))
        })
    }

    /// Call OnCreatureEnterCombat callback.
    pub fn on_creature_enter_combat(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(
            lua,
            "OnCreatureEnterCombat",
            snapshot,
            |_lua, table, input| {
                let func: Function = table.get("OnCreatureEnterCombat")?;
                func.call((table.clone(), input, creature_entry, LuaGuid(creature_guid)))
            },
        )
    }

    /// Call OnCreatureEvade callback.
    pub fn on_creature_evade(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnCreatureEvade", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnCreatureEvade")?;
            func.call((table.clone(), input, creature_entry, LuaGuid(creature_guid)))
        })
    }

    /// Call OnGameObjectCreate callback.
    pub fn on_gameobject_create(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        go_entry: u32,
        go_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnGameObjectCreate", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnGameObjectCreate")?;
            func.call((table.clone(), input, go_entry, LuaGuid(go_guid)))
        })
    }

    /// Call OnPlayerEnter callback.
    pub fn on_player_enter(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        player_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnPlayerEnter", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnPlayerEnter")?;
            func.call((table.clone(), input, LuaGuid(player_guid)))
        })
    }

    /// Call OnPlayerLeave callback.
    pub fn on_player_leave(
        &self,
        lua: &Lua,
        snapshot: &InstanceSnapshot,
        player_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnPlayerLeave", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnPlayerLeave")?;
            func.call((table.clone(), input, LuaGuid(player_guid)))
        })
    }

    /// Call Update callback (periodic tick).
    pub fn on_update(&self, lua: &Lua, snapshot: &InstanceSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "Update", snapshot, |_lua, table, input| {
            let func: Function = table.get("Update")?;
            func.call((table.clone(), input))
        })
    }

    /// Call OnLoad callback (instance loaded/created).
    pub fn on_load(&self, lua: &Lua, snapshot: &InstanceSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "OnLoad", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnLoad")?;
            func.call((table.clone(), input))
        })
    }

    /// Get the script table from Lua registry.
    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("instance_{}", self.map_id);
        lua.named_registry_value::<Table>(&key).ok()
    }

    /// Check if a callback exists.
    pub fn has_callback(&self, lua: &Lua, callback: &str) -> bool {
        if let Some(table) = self.get_script_table(lua) {
            table.get::<Function>(callback).is_ok()
        } else {
            false
        }
    }

    /// Call a callback with just the snapshot.
    fn call_callback<F>(
        &self,
        lua: &Lua,
        callback: &str,
        snapshot: &InstanceSnapshot,
        f: F,
    ) -> Vec<LuaAction>
    where
        F: FnOnce(&Lua, &Table, Table) -> mlua::Result<Value>,
    {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Instance script for map {} not found in registry",
                    self.map_id
                );
                return Vec::new();
            }
        };

        if table.get::<Function>(callback).is_err() {
            return Vec::new();
        }

        let input = match snapshot.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create input table for instance {}: {}",
                    self.map_id,
                    e
                );
                return Vec::new();
            }
        };

        match f(lua, &table, input) {
            Ok(result) => parse_actions(result),
            Err(e) => {
                tracing::error!(
                    "Error in instance script {} callback {}: {}",
                    self.map_id,
                    callback,
                    e
                );
                Vec::new()
            }
        }
    }

    /// Call a callback with extra arguments beyond the snapshot.
    fn call_callback_with_extra<F>(
        &self,
        lua: &Lua,
        callback: &str,
        snapshot: &InstanceSnapshot,
        f: F,
    ) -> Vec<LuaAction>
    where
        F: FnOnce(&Lua, &Table, Table) -> mlua::Result<Value>,
    {
        self.call_callback(lua, callback, snapshot, f)
    }
}

/// State stored per instance for data tracking.
#[derive(Debug, Clone, Default)]
pub struct InstanceScriptState {
    /// Instance data values (encounter states, counters, etc.)
    pub data: HashMap<u32, u32>,
    /// GUIDs stored by data ID (for tracking specific creatures/GOs)
    pub guids: HashMap<u32, ObjectGuid>,
    /// Custom timers
    pub timers: HashMap<u32, u32>,
}

impl InstanceScriptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_data(&mut self, data_id: u32, value: u32) {
        self.data.insert(data_id, value);
    }

    pub fn get_data(&self, data_id: u32) -> u32 {
        self.data.get(&data_id).copied().unwrap_or(0)
    }

    pub fn set_guid(&mut self, data_id: u32, guid: ObjectGuid) {
        self.guids.insert(data_id, guid);
    }

    pub fn get_guid(&self, data_id: u32) -> Option<ObjectGuid> {
        self.guids.get(&data_id).copied()
    }

    pub fn update_timers(&mut self, diff_ms: u32) {
        for timer in self.timers.values_mut() {
            *timer = timer.saturating_sub(diff_ms);
        }
    }

    pub fn set_timer(&mut self, timer_id: u32, duration_ms: u32) {
        self.timers.insert(timer_id, duration_ms);
    }

    pub fn is_timer_ready(&self, timer_id: u32) -> bool {
        self.timers.get(&timer_id).map(|&v| v == 0).unwrap_or(true)
    }

    /// Build an InstanceSnapshot from current state.
    pub fn to_snapshot(
        &self,
        map_id: u32,
        instance_id: u32,
    ) -> super::super::snapshot::InstanceSnapshot {
        super::super::snapshot::InstanceSnapshot {
            map_id,
            instance_id,
            difficulty: 0,
            player_count: 0,
            data: self.data.clone(),
            guids: self.guids.clone(),
            timers: self.timers.clone(),
        }
    }
}
