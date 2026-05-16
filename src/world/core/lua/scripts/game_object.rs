//! Lua game object script handler.
//!
//! Handles OnGameObjectHello (player right-clicks a GO) and
//! OnGameObjectOpen (player opens/loots a GO).

use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::{LuaGuid, PlayerSnapshot};
use crate::shared::protocol::ObjectGuid;
use mlua::{Function, Lua, Table, Value};

/// Lua game object script handler.
///
/// Wraps a script table registered via RegisterGameObjectScript(go_entry, table).
pub struct LuaGameObjectScript {
    go_entry: u32,
}

impl LuaGameObjectScript {
    pub fn new(go_entry: u32) -> Self {
        Self { go_entry }
    }

    pub fn go_entry(&self) -> u32 {
        self.go_entry
    }

    /// Called when a player right-clicks/uses this game object (GOHello).
    pub fn on_gameobject_hello(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        go_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback(lua, "OnGameObjectHello", player, go_guid)
    }

    /// Called when a player opens/loots this game object (GOOpen).
    pub fn on_gameobject_open(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        go_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback(lua, "OnGameObjectOpen", player, go_guid)
    }

    /// Called when a player completes a quest at this game object (QuestRewarded).
    pub fn on_quest_rewarded(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        go_guid: ObjectGuid,
        quest_id: u32,
    ) -> Vec<LuaAction> {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => return Vec::new(),
        };

        if table.get::<Function>("OnQuestRewarded").is_err() {
            return Vec::new();
        }

        let input = match player.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create player table for GO quest rewarded script {}: {}",
                    self.go_entry,
                    e
                );
                return Vec::new();
            }
        };

        let result: mlua::Result<Value> = (|| {
            let func: Function = table.get("OnQuestRewarded")?;
            func.call((table.clone(), input, LuaGuid(go_guid), quest_id))
        })();

        match result {
            Ok(v) => parse_actions(v),
            Err(e) => {
                tracing::error!(
                    "Error in GO script {} OnQuestRewarded quest={}: {}",
                    self.go_entry,
                    quest_id,
                    e
                );
                Vec::new()
            }
        }
    }

    /// Check if a specific callback is defined in the script.
    pub fn has_callback(&self, lua: &Lua, callback: &str) -> bool {
        if let Some(table) = self.get_script_table(lua) {
            table.get::<Function>(callback).is_ok()
        } else {
            false
        }
    }

    fn call_callback(
        &self,
        lua: &Lua,
        callback: &str,
        player: &PlayerSnapshot,
        go_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Game object script for entry {} not found in registry",
                    self.go_entry
                );
                return Vec::new();
            }
        };

        if table.get::<Function>(callback).is_err() {
            return Vec::new();
        }

        let input = match player.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create player table for GO script {}: {}",
                    self.go_entry,
                    e
                );
                return Vec::new();
            }
        };

        let result: mlua::Result<Value> = (|| {
            let func: Function = table.get(callback)?;
            func.call((table.clone(), input, LuaGuid(go_guid)))
        })();

        match result {
            Ok(v) => parse_actions(v),
            Err(e) => {
                tracing::error!(
                    "Error in GO script {} callback {}: {}",
                    self.go_entry,
                    callback,
                    e
                );
                Vec::new()
            }
        }
    }

    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("go_{}", self.go_entry);
        lua.named_registry_value::<Table>(&key).ok()
    }
}
