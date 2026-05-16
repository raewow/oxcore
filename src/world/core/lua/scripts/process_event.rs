//! Lua process event script handler.
//!
//! Handles OnProcessEventId — called when a game/script event fires
//! (e.g. altar click awakens a boss, creature death triggers sequence).

use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::{LuaGuid, PlayerSnapshot};
use crate::shared::protocol::ObjectGuid;
use mlua::{Function, Lua, Table, Value};

/// Lua process event script handler.
///
/// Wraps a script table registered via RegisterProcessEventScript(event_id, table).
pub struct LuaProcessEventScript {
    event_id: u32,
}

impl LuaProcessEventScript {
    pub fn new(event_id: u32) -> Self {
        Self { event_id }
    }

    pub fn event_id(&self) -> u32 {
        self.event_id
    }

    /// Called when the script event fires.
    ///
    /// `player` — the player that triggered the event (if applicable)
    /// `source_guid` — the object that sourced the event
    /// `is_start` — true if event starting, false if ending
    pub fn on_process_event(
        &self,
        lua: &Lua,
        player: &PlayerSnapshot,
        source_guid: ObjectGuid,
        is_start: bool,
    ) -> Vec<LuaAction> {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Process event script for event {} not found in registry",
                    self.event_id
                );
                return Vec::new();
            }
        };

        if table.get::<Function>("OnProcessEventId").is_err() {
            return Vec::new();
        }

        let input = match player.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create player table for process event {}: {}",
                    self.event_id,
                    e
                );
                return Vec::new();
            }
        };

        let result: mlua::Result<Value> = (|| {
            let func: Function = table.get("OnProcessEventId")?;
            func.call((
                table.clone(),
                input,
                self.event_id,
                LuaGuid(source_guid),
                is_start,
            ))
        })();

        match result {
            Ok(v) => parse_actions(v),
            Err(e) => {
                tracing::error!(
                    "Error in process event script {} OnProcessEventId: {}",
                    self.event_id,
                    e
                );
                Vec::new()
            }
        }
    }

    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("processevent_{}", self.event_id);
        lua.named_registry_value::<Table>(&key).ok()
    }
}
