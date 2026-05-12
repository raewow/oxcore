//! Lua area trigger script handler.
//!
//! Handles calling OnAreaTrigger when a player enters a scripted zone.

use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::PlayerSnapshot;
use mlua::{Function, Lua, Table, Value};

/// Lua area trigger script handler.
///
/// Wraps a script table registered via RegisterAreaTriggerScript(trigger_id, table).
pub struct LuaAreaTriggerScript {
    trigger_id: u32,
}

impl LuaAreaTriggerScript {
    pub fn new(trigger_id: u32) -> Self {
        Self { trigger_id }
    }

    pub fn trigger_id(&self) -> u32 {
        self.trigger_id
    }

    /// Called when a player enters this area trigger zone.
    pub fn on_area_trigger(&self, lua: &Lua, player: &PlayerSnapshot) -> Vec<LuaAction> {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Area trigger script for trigger {} not found in registry",
                    self.trigger_id
                );
                return Vec::new();
            }
        };

        if table.get::<Function>("OnAreaTrigger").is_err() {
            return Vec::new();
        }

        let input = match player.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create player table for area trigger {}: {}",
                    self.trigger_id,
                    e
                );
                return Vec::new();
            }
        };

        let result: mlua::Result<Value> = (|| {
            let func: Function = table.get("OnAreaTrigger")?;
            func.call((table.clone(), input))
        })();

        match result {
            Ok(v) => parse_actions(v),
            Err(e) => {
                tracing::error!(
                    "Error in area trigger script {} OnAreaTrigger: {}",
                    self.trigger_id,
                    e
                );
                Vec::new()
            }
        }
    }

    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("at_{}", self.trigger_id);
        lua.named_registry_value::<Table>(&key).ok()
    }
}
