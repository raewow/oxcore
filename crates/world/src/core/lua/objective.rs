//! Calling objective callbacks on a scope's script.
//!
//! Battleground and open-world PvP objectives use one callback set whether
//! their script is registered for a map (`RegisterInstanceScript`, one state
//! per copy) or for a zone (`RegisterZoneScript`). Every callback receives the
//! scope's input table first, then its own arguments, and returns actions.

use mlua::{Function, IntoLuaMulti, Lua, Table, Value};

use super::actions::{parse_actions, LuaAction};

/// The script that owns a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeScript {
    /// Registered for a map; serves every copy of it.
    Instance(u32),
    /// Registered for a zone.
    Zone(u32),
}

/// What a callback returned.
#[derive(Debug, Default)]
pub struct CallbackResult {
    pub actions: Vec<LuaAction>,
    /// The callback claimed the event (returned actions, `true`, or a table
    /// with `handled = true`), so default handling should not run.
    pub handled: bool,
}

impl ScopeScript {
    fn registry_key(&self) -> String {
        match self {
            ScopeScript::Instance(map_id) => format!("instance_{}", map_id),
            ScopeScript::Zone(zone_id) => format!("zone_{}", zone_id),
        }
    }

    fn table(&self, lua: &Lua) -> Option<Table> {
        lua.named_registry_value::<Table>(&self.registry_key()).ok()
    }

    pub fn has_callback(&self, lua: &Lua, callback: &str) -> bool {
        self.table(lua)
            .is_some_and(|table| table.get::<Function>(callback).is_ok())
    }

    /// A field of the script table, such as `WatchedAuras` or `ScoreColumns`.
    pub fn field<T: mlua::FromLua>(&self, lua: &Lua, name: &str) -> Option<T> {
        self.table(lua).and_then(|table| table.get::<T>(name).ok())
    }

    /// A list of numbers declared on the script table (e.g. `WatchedAuras = {23333}`).
    pub fn number_list(&self, lua: &Lua, name: &str) -> Vec<u32> {
        self.field::<Table>(lua, name)
            .map(|list| list.sequence_values::<u32>().filter_map(Result::ok).collect())
            .unwrap_or_default()
    }

    /// Call `callback(self, input, args...)`.
    pub fn call(
        &self,
        lua: &Lua,
        callback: &str,
        input: Table,
        args: impl IntoLuaMulti,
    ) -> CallbackResult {
        let Some(table) = self.table(lua) else {
            return CallbackResult::default();
        };
        let Ok(func) = table.get::<Function>(callback) else {
            return CallbackResult::default();
        };
        let mut all_args = args.into_lua_multi(lua).unwrap_or_default();
        all_args.push_front(Value::Table(input));
        all_args.push_front(Value::Table(table.clone()));
        match func.call::<Value>(all_args) {
            Ok(result) => interpret(result),
            Err(e) => {
                tracing::error!("Error in {:?} callback {}: {}", self, callback, e);
                CallbackResult::default()
            }
        }
    }

    /// Call a callback that answers with a value instead of actions.
    pub fn query(&self, lua: &Lua, callback: &str, input: Table, args: impl IntoLuaMulti) -> Option<Value> {
        let table = self.table(lua)?;
        let func = table.get::<Function>(callback).ok()?;
        let mut all_args = args.into_lua_multi(lua).ok()?;
        all_args.push_front(Value::Table(input));
        all_args.push_front(Value::Table(table.clone()));
        match func.call::<Value>(all_args) {
            Ok(Value::Nil) => None,
            Ok(value) => Some(value),
            Err(e) => {
                tracing::error!("Error in {:?} callback {}: {}", self, callback, e);
                None
            }
        }
    }
}

fn interpret(result: Value) -> CallbackResult {
    match result {
        Value::Boolean(handled) => CallbackResult {
            actions: Vec::new(),
            handled,
        },
        Value::Table(table) => {
            let flagged = table.get::<bool>("handled").unwrap_or(false);
            let actions = parse_actions(Value::Table(table));
            CallbackResult {
                handled: flagged || !actions.is_empty(),
                actions,
            }
        }
        _ => CallbackResult::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn register(lua: &Lua, source: &str) -> ScopeScript {
        let table: Table = lua.load(source).eval().expect("script table");
        lua.set_named_registry_value("zone_1", table).unwrap();
        ScopeScript::Zone(1)
    }

    #[test]
    fn callbacks_get_self_input_then_their_arguments() {
        let lua = Lua::new();
        let script = register(
            &lua,
            r#"{ OnAreaTrigger = function(self, input, player, trigger)
                    return { { action = "SET_DATA", data_id = trigger, value = input.zone_id } }
                 end }"#,
        );
        let input = lua.create_table().unwrap();
        input.set("zone_id", 7).unwrap();
        let result = script.call(&lua, "OnAreaTrigger", input, (5u64, 3646u32));
        assert!(result.handled);
        assert!(matches!(
            result.actions.as_slice(),
            [LuaAction::SetInstanceData { data_id: 3646, value: 7 }]
        ));
    }

    #[test]
    fn handled_can_be_claimed_without_actions() {
        let lua = Lua::new();
        let script = register(
            &lua,
            r#"{ A = function() return true end,
                 B = function() return { handled = true } end,
                 C = function() return {} end }"#,
        );
        for (name, expected) in [("A", true), ("B", true), ("C", false), ("Missing", false)] {
            let input = lua.create_table().unwrap();
            assert_eq!(script.call(&lua, name, input, ()).handled, expected, "{name}");
        }
    }

    #[test]
    fn the_battleground_and_world_pvp_scripts_load() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts");
        let mgr = crate::core::lua::LuaScriptManager::with_scripts_dir(dir);
        let result = mgr.initialize().expect("scripts load");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        for map in [489, 529, 30] {
            assert!(mgr.has_instance_script(map), "map {map}");
        }
        assert!(mgr.has_zone_script(1377));
        mgr.with_lua(|lua| {
            assert_eq!(ScopeScript::Instance(489).field::<u32>(lua, "ScoreColumns"), Some(2));
            assert_eq!(ScopeScript::Instance(30).field::<u32>(lua, "ScoreColumns"), Some(7));
            assert_eq!(
                ScopeScript::Instance(489).number_list(lua, "WatchedAuras"),
                vec![23333, 23335]
            );
            assert!(ScopeScript::Zone(1377).has_callback(lua, "OnAreaTrigger"));
        });
    }

    #[test]
    fn declared_lists_are_read_from_the_table() {
        let lua = Lua::new();
        let script = register(&lua, r#"{ WatchedAuras = { 23333, 23335 }, ScoreColumns = 2 }"#);
        assert_eq!(script.number_list(&lua, "WatchedAuras"), vec![23333, 23335]);
        assert_eq!(script.field::<u32>(&lua, "ScoreColumns"), Some(2));
    }
}
