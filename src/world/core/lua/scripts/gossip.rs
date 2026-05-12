//! Lua gossip/zone script handler.
//!
//! Handles calling Lua callbacks for gossip scripts (OnGossipHello, OnGossipSelect,
//! OnQuestAccept, OnQuestRewarded) and parsing the returned action tables.

use super::super::super::common::ObjectGuid;
use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::{LuaGuid, PlayerSnapshot};
use mlua::{Function, Lua, Table, Value};

/// Lua gossip script handler.
///
/// Wraps a Lua script table registered via RegisterGossipScript(entry, table).
pub struct LuaGossipScript {
    entry: u32,
}

impl LuaGossipScript {
    pub fn new(entry: u32) -> Self {
        Self { entry }
    }

    pub fn entry(&self) -> u32 {
        self.entry
    }

    /// Called when a player opens gossip with this NPC.
    pub fn on_gossip_hello(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        npc_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnGossipHello", player, |_lua, table, input| {
            let func: Function = table.get("OnGossipHello")?;
            func.call((table.clone(), input, LuaGuid(npc_guid)))
        })
    }

    /// Called when a player selects a gossip option.
    pub fn on_gossip_select(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        npc_guid: ObjectGuid,
        menu_id: u32,
        option_id: u32,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnGossipSelect", player, |_lua, table, input| {
            let func: Function = table.get("OnGossipSelect")?;
            func.call((table.clone(), input, LuaGuid(npc_guid), menu_id, option_id))
        })
    }

    /// Called when a player accepts a quest from this NPC.
    pub fn on_quest_accept(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        npc_guid: ObjectGuid,
        quest_id: u32,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnQuestAccept", player, |_lua, table, input| {
            let func: Function = table.get("OnQuestAccept")?;
            func.call((table.clone(), input, LuaGuid(npc_guid), quest_id))
        })
    }

    /// Called when a player turns in a quest at this NPC.
    pub fn on_quest_rewarded(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        npc_guid: ObjectGuid,
        quest_id: u32,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnQuestRewarded", player, |_lua, table, input| {
            let func: Function = table.get("OnQuestRewarded")?;
            func.call((table.clone(), input, LuaGuid(npc_guid), quest_id))
        })
    }

    /// Check if a specific callback is defined in the script.
    pub fn has_callback(&self, lua: &Lua, callback: &str) -> bool {
        if let Some(table) = self.get_script_table(lua) {
            table.get::<Function>(callback).is_ok()
        } else {
            false
        }
    }

    /// Get the script table from Lua registry.
    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("gossip_{}", self.entry);
        lua.named_registry_value::<Table>(&key).ok()
    }

    /// Call a gossip callback with player snapshot and extra arguments.
    fn call_callback_with_extra<F>(
        &self,
        lua: &Lua,
        callback: &str,
        player: &PlayerSnapshot,
        f: F,
    ) -> Vec<LuaAction>
    where
        F: FnOnce(&Lua, &Table, Table) -> mlua::Result<Value>,
    {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Gossip script for entry {} not found in registry",
                    self.entry
                );
                return Vec::new();
            }
        };

        // Return empty if callback not defined — allows partial scripts
        if table.get::<Function>(callback).is_err() {
            return Vec::new();
        }

        let input = match player.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create player table for gossip script {}: {}",
                    self.entry,
                    e
                );
                return Vec::new();
            }
        };

        match f(lua, &table, input) {
            Ok(result) => parse_actions(result),
            Err(e) => {
                tracing::error!(
                    "Error in gossip script {} callback {}: {}",
                    self.entry,
                    callback,
                    e
                );
                Vec::new()
            }
        }
    }
}
