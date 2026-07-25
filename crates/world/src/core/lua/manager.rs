//! Lua script manager.
//!
//! Central coordinator for the Lua scripting system. Manages the Lua VM,
//! script loading, and provides access to script handlers.

use super::api::{create_sandbox, register_api_functions, RegistryStats, ScriptRegistry};
use super::error::{LuaError, LuaResult};
use super::loader::{default_scripts_path, load_all_scripts, LoadResult};
use super::scripts::{
    InstanceScriptState, LuaAreaTriggerScript, LuaCreatureAI, LuaEffectDummyScript,
    LuaGameObjectScript, LuaGossipScript, LuaInstanceAI, LuaProcessEventScript, LuaZoneScript,
    ZoneScriptState,
};
use dashmap::DashMap;
use mlua::Lua;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Lua script manager.
///
/// This is the main entry point for the Lua scripting system.
/// It manages the Lua VM, script registry, and provides access to script handlers.
pub struct LuaScriptManager {
    /// The Lua VM (wrapped in RwLock for thread-safe access).
    lua: RwLock<Lua>,
    /// Script registry tracking registered scripts.
    registry: Arc<ScriptRegistry>,
    /// Path to the scripts directory.
    scripts_dir: PathBuf,
    /// Whether the manager has been initialized.
    initialized: RwLock<bool>,
    /// Per-instance state storage, keyed by (map_id, instance_id).
    instance_states: DashMap<(u32, u32), InstanceScriptState>,
    /// Per-zone state storage, keyed by zone_id.
    zone_states: DashMap<u32, ZoneScriptState>,
}

impl LuaScriptManager {
    /// Create a new script manager.
    ///
    /// Does not load scripts yet - call `initialize()` to load scripts.
    pub fn new(data_dir: &Path) -> Self {
        let scripts_dir = default_scripts_path(data_dir);

        Self {
            lua: RwLock::new(Lua::new()),
            registry: Arc::new(ScriptRegistry::new()),
            scripts_dir,
            initialized: RwLock::new(false),
            instance_states: DashMap::new(),
            zone_states: DashMap::new(),
        }
    }

    /// Create a new script manager with a custom scripts directory.
    pub fn with_scripts_dir(scripts_dir: PathBuf) -> Self {
        Self {
            lua: RwLock::new(Lua::new()),
            registry: Arc::new(ScriptRegistry::new()),
            scripts_dir,
            initialized: RwLock::new(false),
            instance_states: DashMap::new(),
            zone_states: DashMap::new(),
        }
    }

    /// Initialize the script manager by loading all scripts.
    ///
    /// This should be called once during server startup.
    pub fn initialize(&self) -> LuaResult<LoadResult> {
        let mut initialized = self.initialized.write();
        if *initialized {
            return Err(LuaError::AlreadyInitialized);
        }

        let result = self.load_scripts()?;
        *initialized = true;

        tracing::debug!(
            "Lua scripting system initialized: {} scripts loaded ({} failed)",
            result.loaded,
            result.failed
        );

        Ok(result)
    }

    /// Reload all scripts.
    ///
    /// This clears the current Lua state and reloads all scripts from disk.
    /// Used for hot-reloading during development.
    pub fn reload(&self) -> LuaResult<LoadResult> {
        tracing::info!("Reloading Lua scripts...");

        // Clear the registry
        self.registry.clear();

        // Create a new Lua VM
        {
            let mut lua = self.lua.write();
            *lua = Lua::new();
        }

        // Reload scripts
        let result = self.load_scripts()?;

        tracing::info!(
            "Lua scripts reloaded: {} scripts loaded ({} failed)",
            result.loaded,
            result.failed
        );

        if !result.errors.is_empty() {
            for error in &result.errors {
                tracing::error!("Script error: {}", error);
            }
        }

        Ok(result)
    }

    /// Load scripts from the scripts directory.
    fn load_scripts(&self) -> LuaResult<LoadResult> {
        let lua = self.lua.read();

        // Create sandbox environment
        let sandbox = create_sandbox(&lua).map_err(LuaError::Runtime)?;

        // Register API functions
        register_api_functions(&lua, &sandbox, self.registry.clone()).map_err(LuaError::Runtime)?;

        // Load all scripts
        load_all_scripts(&lua, &sandbox, &self.scripts_dir, self.registry.clone())
    }

    /// Check if the manager has been initialized.
    pub fn is_initialized(&self) -> bool {
        *self.initialized.read()
    }

    /// Get statistics about registered scripts.
    pub fn stats(&self) -> RegistryStats {
        self.registry.stats()
    }

    /// Get the scripts directory path.
    pub fn scripts_dir(&self) -> &Path {
        &self.scripts_dir
    }

    // ==================== Script Handler Access ====================

    /// Check if a creature AI script exists for the given entry.
    pub fn has_creature_ai(&self, entry: u32) -> bool {
        self.registry.creature_ai.read().contains_key(&entry)
    }

    /// Get a creature AI script handler.
    pub fn get_creature_ai(&self, entry: u32) -> Option<LuaCreatureAI> {
        if self.has_creature_ai(entry) {
            Some(LuaCreatureAI::new(entry))
        } else {
            None
        }
    }

    /// Check if an instance script exists for the given map ID.
    pub fn has_instance_script(&self, map_id: u32) -> bool {
        self.registry.instance.read().contains_key(&map_id)
    }

    /// Get an instance script handler.
    pub fn get_instance_ai(&self, map_id: u32) -> Option<LuaInstanceAI> {
        if self.has_instance_script(map_id) {
            Some(LuaInstanceAI::new(map_id))
        } else {
            None
        }
    }

    /// Get or create instance state for a specific instance.
    pub fn get_instance_state(&self, map_id: u32, instance_id: u32) -> InstanceScriptState {
        self.instance_states
            .entry((map_id, instance_id))
            .or_insert_with(InstanceScriptState::new)
            .clone()
    }

    /// Update instance state.
    pub fn set_instance_state(&self, map_id: u32, instance_id: u32, state: InstanceScriptState) {
        self.instance_states.insert((map_id, instance_id), state);
    }

    /// Set instance data value.
    pub fn set_instance_data(&self, map_id: u32, instance_id: u32, data_id: u32, value: u32) {
        self.instance_states
            .entry((map_id, instance_id))
            .or_insert_with(InstanceScriptState::new)
            .set_data(data_id, value);
    }

    /// Get instance data value.
    pub fn get_instance_data(&self, map_id: u32, instance_id: u32, data_id: u32) -> u32 {
        self.instance_states
            .get(&(map_id, instance_id))
            .map(|s| s.get_data(data_id))
            .unwrap_or(0)
    }

    /// Check if a zone script exists for the given zone ID.
    pub fn has_zone_script(&self, zone_id: u32) -> bool {
        self.registry.zone.read().contains_key(&zone_id)
    }

    /// Get a zone script handler.
    pub fn get_zone_script(&self, zone_id: u32) -> Option<LuaZoneScript> {
        if self.has_zone_script(zone_id) {
            Some(LuaZoneScript::new(zone_id))
        } else {
            None
        }
    }

    /// Get (a copy of) the state for a zone, creating it if absent.
    pub fn get_zone_state(&self, zone_id: u32) -> ZoneScriptState {
        self.zone_states
            .entry(zone_id)
            .or_insert_with(ZoneScriptState::new)
            .clone()
    }

    /// Mutate a zone's state in place.
    ///
    /// Preferred over `get_zone_state` + `set_zone_state` because it holds the
    /// entry lock across the change, so concurrent zone transitions cannot lose
    /// each other's updates.
    pub fn with_zone_state_mut<F, R>(&self, zone_id: u32, f: F) -> R
    where
        F: FnOnce(&mut ZoneScriptState) -> R,
    {
        let mut entry = self
            .zone_states
            .entry(zone_id)
            .or_insert_with(ZoneScriptState::new);
        f(entry.value_mut())
    }

    /// Replace a zone's state.
    pub fn set_zone_state(&self, zone_id: u32, state: ZoneScriptState) {
        self.zone_states.insert(zone_id, state);
    }

    /// Set a zone data value.
    pub fn set_zone_data(&self, zone_id: u32, data_id: u32, value: u32) {
        self.with_zone_state_mut(zone_id, |state| state.set_data(data_id, value));
    }

    /// Get a zone data value (0 when unset).
    pub fn get_zone_data(&self, zone_id: u32, data_id: u32) -> u32 {
        self.zone_states
            .get(&zone_id)
            .map(|s| s.get_data(data_id))
            .unwrap_or(0)
    }

    /// Zone ids that currently have tracked state, i.e. the zones worth ticking.
    pub fn active_zone_states(&self) -> Vec<u32> {
        self.zone_states.iter().map(|e| *e.key()).collect()
    }

    /// Register a zone script table directly (used by tests).
    /// This is the Rust equivalent of calling `RegisterZoneScript(zone_id, script_table)` in Lua.
    pub fn register_zone_script_table(&self, zone_id: u32, script_table: mlua::Table) {
        let key = format!("zone_{}", zone_id);
        let lua = self.lua.read();
        let _ = lua.set_named_registry_value(&key, script_table);
        self.registry.zone.write().insert(
            zone_id,
            super::api::ScriptEntry {
                name: key,
                file_path: String::new(),
            },
        );
    }

    /// Check if a gossip script exists for the given entry.
    pub fn has_gossip_script(&self, entry: u32) -> bool {
        self.registry.gossip.read().contains_key(&entry)
    }

    /// Get a gossip script handler.
    pub fn get_gossip_script(&self, entry: u32) -> Option<LuaGossipScript> {
        if self.has_gossip_script(entry) {
            Some(LuaGossipScript::new(entry))
        } else {
            None
        }
    }

    /// Register a gossip script table directly (used by tests).
    /// This is the Rust equivalent of calling `RegisterGossipScript(entry, script_table)` in Lua.
    pub fn register_gossip_script_table(&self, entry: u32, script_table: mlua::Table) {
        let key = format!("gossip_{}", entry);
        let lua = self.lua.read();
        let _ = lua.set_named_registry_value(&key, script_table);
        self.registry.gossip.write().insert(
            entry,
            super::api::ScriptEntry {
                name: key,
                file_path: String::new(),
            },
        );
    }

    /// Check if an area trigger script exists for the given trigger ID.
    pub fn has_area_trigger_script(&self, trigger_id: u32) -> bool {
        self.registry.area_trigger.read().contains_key(&trigger_id)
    }

    /// Get an area trigger script handler.
    pub fn get_area_trigger_script(&self, trigger_id: u32) -> Option<LuaAreaTriggerScript> {
        if self.has_area_trigger_script(trigger_id) {
            Some(LuaAreaTriggerScript::new(trigger_id))
        } else {
            None
        }
    }

    /// Check if a game object script exists for the given GO entry.
    pub fn has_game_object_script(&self, go_entry: u32) -> bool {
        self.registry.game_object.read().contains_key(&go_entry)
    }

    /// Get a game object script handler.
    pub fn get_game_object_script(&self, go_entry: u32) -> Option<LuaGameObjectScript> {
        if self.has_game_object_script(go_entry) {
            Some(LuaGameObjectScript::new(go_entry))
        } else {
            None
        }
    }

    /// Check if an effect dummy script exists for the given entry.
    pub fn has_effect_dummy_script(&self, entry: u32) -> bool {
        self.registry.effect_dummy.read().contains_key(&entry)
    }

    /// Get an effect dummy script handler.
    pub fn get_effect_dummy_script(&self, entry: u32) -> Option<LuaEffectDummyScript> {
        if self.has_effect_dummy_script(entry) {
            Some(LuaEffectDummyScript::new(entry))
        } else {
            None
        }
    }

    /// Check if a process event script exists for the given event ID.
    pub fn has_process_event_script(&self, event_id: u32) -> bool {
        self.registry.process_event.read().contains_key(&event_id)
    }

    /// Get a process event script handler.
    pub fn get_process_event_script(&self, event_id: u32) -> Option<LuaProcessEventScript> {
        if self.has_process_event_script(event_id) {
            Some(LuaProcessEventScript::new(event_id))
        } else {
            None
        }
    }

    // ==================== Direct Lua Access ====================

    /// Execute a callback with read access to the Lua VM.
    ///
    /// This is the primary way to call script callbacks. The callback receives
    /// a reference to the Lua VM and can call script functions.
    pub fn with_lua<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Lua) -> R,
    {
        let lua = self.lua.read();
        f(&lua)
    }

    /// Execute a callback with write access to the Lua VM.
    ///
    /// This should be used sparingly, mainly for operations that modify
    /// the Lua state (like hot-reloading).
    pub fn with_lua_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Lua) -> R,
    {
        let mut lua = self.lua.write();
        f(&mut lua)
    }

    // ==================== Utility Methods ====================

    /// Get a list of all registered creature AI entries.
    pub fn get_creature_ai_entries(&self) -> Vec<u32> {
        self.registry.creature_ai.read().keys().copied().collect()
    }

    /// Get a list of all registered instance script map IDs.
    pub fn get_instance_script_maps(&self) -> Vec<u32> {
        self.registry.instance.read().keys().copied().collect()
    }

    /// Get a list of all registered zone script zone IDs.
    pub fn get_zone_script_zones(&self) -> Vec<u32> {
        self.registry.zone.read().keys().copied().collect()
    }

    /// Get a list of all registered gossip script entries.
    pub fn get_gossip_script_entries(&self) -> Vec<u32> {
        self.registry.gossip.read().keys().copied().collect()
    }
}

impl std::fmt::Debug for LuaScriptManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaScriptManager")
            .field("scripts_dir", &self.scripts_dir)
            .field("initialized", &*self.initialized.read())
            .field("stats", &self.stats())
            .finish()
    }
}
