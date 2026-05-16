//! Lua effect dummy script handler.
//!
//! Handles OnEffectDummy — called when a spell with SPELL_EFFECT_DUMMY (3)
//! hits a creature or game object target. Returns true if the script handled
//! the effect (suppresses default behavior).

use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::LuaGuid;
use crate::shared::protocol::ObjectGuid;
use mlua::{Function, Lua, Table, Value};

/// Lua effect dummy script handler.
///
/// Wraps a script table registered via RegisterEffectDummyScript(entry, table).
/// The `entry` is the creature or GO entry the dummy effect targets.
pub struct LuaEffectDummyScript {
    entry: u32,
}

impl LuaEffectDummyScript {
    pub fn new(entry: u32) -> Self {
        Self { entry }
    }

    pub fn entry(&self) -> u32 {
        self.entry
    }

    /// Called when a dummy spell effect hits the target.
    ///
    /// Returns `(handled, actions)` where:
    /// - `handled` = true suppresses default effect processing
    /// - `actions` = list of LuaActions to execute (e.g. kill credit, spawn)
    ///
    /// The Lua script may return either a bool (old style) or a table of actions.
    /// If a table is returned, `handled` is implicitly true.
    pub fn on_effect_dummy(
        &self,
        lua: &Lua,
        caster_guid: ObjectGuid,
        spell_id: u32,
        effect_index: u8,
        target_guid: ObjectGuid,
    ) -> (bool, Vec<LuaAction>) {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Effect dummy script for entry {} not found in registry",
                    self.entry
                );
                return (false, vec![]);
            }
        };

        if table.get::<Function>("OnEffectDummy").is_err() {
            return (false, vec![]);
        }

        let result: mlua::Result<Value> = (|| {
            let func: Function = table.get("OnEffectDummy")?;
            func.call((
                table.clone(),
                LuaGuid(caster_guid),
                spell_id,
                effect_index as u32,
                self.entry,
                LuaGuid(target_guid),
            ))
        })();

        match result {
            Ok(Value::Boolean(handled)) => (handled, vec![]),
            Ok(v @ Value::Table(_)) => {
                let actions = parse_actions(v);
                (!actions.is_empty(), actions)
            }
            Ok(_) => (false, vec![]),
            Err(e) => {
                tracing::error!(
                    "Error in effect dummy script {} OnEffectDummy spell={}: {}",
                    self.entry,
                    spell_id,
                    e
                );
                (false, vec![])
            }
        }
    }

    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("effectdummy_{}", self.entry);
        lua.named_registry_value::<Table>(&key).ok()
    }
}
