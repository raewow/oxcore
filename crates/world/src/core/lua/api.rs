//! Lua API module.
//!
//! This module provides the Lua API functions that scripts can call.
//! It sets up the sandboxed environment and registers all global functions.

use mlua::{Function, Lua, Result as MluaResult, Table, Value};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Script metadata parsed from file headers.
#[derive(Debug, Clone, Default)]
pub struct ScriptMetadata {
    pub script_type: String,
    pub entry: Option<u32>,
    pub map_id: Option<u32>,
    pub zone_id: Option<u32>,
    pub name: String,
    pub priority: u32,
}

/// Registered script tables, keyed by name.
pub struct ScriptRegistry {
    /// Creature AI scripts by entry ID.
    pub creature_ai: RwLock<HashMap<u32, ScriptEntry>>,
    /// Instance scripts by map ID.
    pub instance: RwLock<HashMap<u32, ScriptEntry>>,
    /// Zone scripts by zone ID.
    pub zone: RwLock<HashMap<u32, ScriptEntry>>,
    /// Gossip scripts by NPC entry ID.
    pub gossip: RwLock<HashMap<u32, ScriptEntry>>,
    /// Area trigger scripts by trigger ID.
    pub area_trigger: RwLock<HashMap<u32, ScriptEntry>>,
    /// Game object scripts by GO entry ID.
    pub game_object: RwLock<HashMap<u32, ScriptEntry>>,
    /// Effect dummy scripts by creature/GO entry ID.
    pub effect_dummy: RwLock<HashMap<u32, ScriptEntry>>,
    /// Process event scripts by event ID.
    pub process_event: RwLock<HashMap<u32, ScriptEntry>>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self {
            creature_ai: RwLock::new(HashMap::new()),
            instance: RwLock::new(HashMap::new()),
            zone: RwLock::new(HashMap::new()),
            gossip: RwLock::new(HashMap::new()),
            area_trigger: RwLock::new(HashMap::new()),
            game_object: RwLock::new(HashMap::new()),
            effect_dummy: RwLock::new(HashMap::new()),
            process_event: RwLock::new(HashMap::new()),
        }
    }

    pub fn clear(&self) {
        self.creature_ai.write().clear();
        self.instance.write().clear();
        self.zone.write().clear();
        self.gossip.write().clear();
        self.area_trigger.write().clear();
        self.game_object.write().clear();
        self.effect_dummy.write().clear();
        self.process_event.write().clear();
    }

    pub fn stats(&self) -> RegistryStats {
        RegistryStats {
            creature_ai: self.creature_ai.read().len(),
            instance: self.instance.read().len(),
            zone: self.zone.read().len(),
            gossip: self.gossip.read().len(),
            area_trigger: self.area_trigger.read().len(),
            game_object: self.game_object.read().len(),
            effect_dummy: self.effect_dummy.read().len(),
            process_event: self.process_event.read().len(),
        }
    }
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A registered script entry.
#[derive(Debug, Clone)]
pub struct ScriptEntry {
    pub name: String,
    pub file_path: String,
}

/// Statistics about registered scripts.
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    pub creature_ai: usize,
    pub instance: usize,
    pub zone: usize,
    pub gossip: usize,
    pub area_trigger: usize,
    pub game_object: usize,
    pub effect_dummy: usize,
    pub process_event: usize,
}

impl std::fmt::Display for RegistryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} creature AI, {} instance, {} zone, {} gossip, {} area trigger, {} game object, {} effect dummy, {} process event",
            self.creature_ai, self.instance, self.zone, self.gossip,
            self.area_trigger, self.game_object, self.effect_dummy, self.process_event
        )
    }
}

/// Create a sandboxed Lua environment.
///
/// This creates a new Lua globals table with only safe functions exposed.
pub fn create_sandbox(lua: &Lua) -> MluaResult<Table> {
    let sandbox = lua.create_table()?;

    // Basic Lua functions (safe)
    let globals = lua.globals();

    // Copy safe globals
    let safe_globals = [
        "assert", "error", "ipairs", "next", "pairs", "pcall", "print", "select", "tonumber",
        "tostring", "type", "unpack", "xpcall", "_VERSION",
    ];

    for name in safe_globals {
        if let Ok(value) = globals.get::<Value>(name) {
            sandbox.set(name, value)?;
        }
    }

    // Safe table library
    if let Ok(table_lib) = globals.get::<Table>("table") {
        let safe_table = lua.create_table()?;
        let table_funcs = ["concat", "insert", "remove", "sort", "unpack"];
        for name in table_funcs {
            if let Ok(func) = table_lib.get::<Function>(name) {
                safe_table.set(name, func)?;
            }
        }
        sandbox.set("table", safe_table)?;
    }

    // Safe string library
    if let Ok(string_lib) = globals.get::<Table>("string") {
        let safe_string = lua.create_table()?;
        let string_funcs = [
            "byte", "char", "find", "format", "gmatch", "gsub", "len", "lower", "match", "rep",
            "reverse", "sub", "upper",
        ];
        for name in string_funcs {
            if let Ok(func) = string_lib.get::<Function>(name) {
                safe_string.set(name, func)?;
            }
        }
        sandbox.set("string", safe_string)?;
    }

    // Safe math library
    if let Ok(math_lib) = globals.get::<Table>("math") {
        let safe_math = lua.create_table()?;
        let math_funcs = [
            "abs",
            "acos",
            "asin",
            "atan",
            "atan2",
            "ceil",
            "cos",
            "deg",
            "exp",
            "floor",
            "fmod",
            "huge",
            "log",
            "log10",
            "max",
            "min",
            "modf",
            "pi",
            "pow",
            "rad",
            "random",
            "randomseed",
            "sin",
            "sqrt",
            "tan",
        ];
        for name in math_funcs {
            if let Ok(value) = math_lib.get::<Value>(name) {
                safe_math.set(name, value)?;
            }
        }
        sandbox.set("math", safe_math)?;
    }

    // Self-reference for _G
    sandbox.set("_G", sandbox.clone())?;

    Ok(sandbox)
}

/// Register script API functions in the sandbox.
///
/// This adds the registration functions that scripts call to register themselves.
pub fn register_api_functions(
    lua: &Lua,
    sandbox: &Table,
    registry: Arc<ScriptRegistry>,
) -> MluaResult<()> {
    // RegisterCreatureAI(entry, script_table)
    let registry_clone = registry.clone();
    let register_creature_ai =
        lua.create_function(move |lua, (entry, script_table): (u32, Table)| {
            // Store the script table in Lua registry for later access
            let key = format!("creature_ai_{}", entry);
            lua.set_named_registry_value(&key, script_table)?;

            // Record in our registry
            registry_clone.creature_ai.write().insert(
                entry,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(), // Set by loader
                },
            );

            tracing::debug!("Registered creature AI for entry {}", entry);
            Ok(())
        })?;
    sandbox.set("RegisterCreatureAI", register_creature_ai)?;

    // RegisterInstanceScript(map_id, script_table)
    let registry_clone = registry.clone();
    let register_instance =
        lua.create_function(move |lua, (map_id, script_table): (u32, Table)| {
            let key = format!("instance_{}", map_id);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.instance.write().insert(
                map_id,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered instance script for map {}", map_id);
            Ok(())
        })?;
    sandbox.set("RegisterInstanceScript", register_instance)?;

    // RegisterZoneScript(zone_id, script_table)
    let registry_clone = registry.clone();
    let register_zone =
        lua.create_function(move |lua, (zone_id, script_table): (u32, Table)| {
            let key = format!("zone_{}", zone_id);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.zone.write().insert(
                zone_id,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered zone script for zone {}", zone_id);
            Ok(())
        })?;
    sandbox.set("RegisterZoneScript", register_zone)?;

    // RegisterGossipScript(entry, script_table)
    let registry_clone = registry.clone();
    let register_gossip =
        lua.create_function(move |lua, (entry, script_table): (u32, Table)| {
            let key = format!("gossip_{}", entry);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.gossip.write().insert(
                entry,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered gossip script for entry {}", entry);
            Ok(())
        })?;
    sandbox.set("RegisterGossipScript", register_gossip)?;

    // RegisterAreaTriggerScript(trigger_id, script_table)
    let registry_clone = registry.clone();
    let register_area_trigger =
        lua.create_function(move |lua, (trigger_id, script_table): (u32, Table)| {
            let key = format!("at_{}", trigger_id);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.area_trigger.write().insert(
                trigger_id,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered area trigger script for trigger {}", trigger_id);
            Ok(())
        })?;
    sandbox.set("RegisterAreaTriggerScript", register_area_trigger)?;

    // RegisterGameObjectScript(go_entry, script_table)
    let registry_clone = registry.clone();
    let register_game_object =
        lua.create_function(move |lua, (go_entry, script_table): (u32, Table)| {
            let key = format!("go_{}", go_entry);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.game_object.write().insert(
                go_entry,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered game object script for entry {}", go_entry);
            Ok(())
        })?;
    sandbox.set("RegisterGameObjectScript", register_game_object)?;

    // RegisterEffectDummyScript(creature_or_go_entry, script_table)
    let registry_clone = registry.clone();
    let register_effect_dummy =
        lua.create_function(move |lua, (entry, script_table): (u32, Table)| {
            let key = format!("effectdummy_{}", entry);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.effect_dummy.write().insert(
                entry,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered effect dummy script for entry {}", entry);
            Ok(())
        })?;
    sandbox.set("RegisterEffectDummyScript", register_effect_dummy)?;

    // RegisterProcessEventScript(event_id, script_table)
    let registry_clone = registry.clone();
    let register_process_event =
        lua.create_function(move |lua, (event_id, script_table): (u32, Table)| {
            let key = format!("processevent_{}", event_id);
            lua.set_named_registry_value(&key, script_table)?;

            registry_clone.process_event.write().insert(
                event_id,
                ScriptEntry {
                    name: key.clone(),
                    file_path: String::new(),
                },
            );

            tracing::debug!("Registered process event script for event {}", event_id);
            Ok(())
        })?;
    sandbox.set("RegisterProcessEventScript", register_process_event)?;

    Ok(())
}

/// Parse script metadata from file content.
///
/// Looks for a Lua block comment at the start of the file:
/// ```lua
/// --[[
///     @script_type: creature_ai
///     @entry: 11502
///     @name: boss_ragnaros
/// ]]
/// ```
pub fn parse_metadata(content: &str) -> ScriptMetadata {
    let mut metadata = ScriptMetadata::default();
    metadata.priority = 100; // Default priority

    // Find the first block comment
    if let Some(start) = content.find("--[[") {
        if let Some(end) = content[start..].find("]]") {
            let block = &content[start + 4..start + end];

            for line in block.lines() {
                let line = line.trim();
                if line.starts_with('@') {
                    if let Some((key, value)) = line[1..].split_once(':') {
                        let key = key.trim().to_lowercase();
                        let value = value.trim();

                        match key.as_str() {
                            "script_type" => metadata.script_type = value.to_string(),
                            "entry" => metadata.entry = value.parse().ok(),
                            "map_id" => metadata.map_id = value.parse().ok(),
                            "zone_id" => metadata.zone_id = value.parse().ok(),
                            "name" => metadata.name = value.to_string(),
                            "priority" => {
                                if let Ok(p) = value.parse() {
                                    metadata.priority = p;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    metadata
}
