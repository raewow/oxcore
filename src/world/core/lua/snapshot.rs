//! Lua snapshot types.
//!
//! Snapshots are readonly copies of game state passed to Lua scripts.
//! Scripts cannot modify game state directly - they must return actions.

use super::super::common::ObjectGuid;
use mlua::{Lua, Result as LuaResult, Table, UserData, UserDataMethods, Value};
use std::collections::HashMap;

/// A snapshot of creature state for Lua scripts.
#[derive(Debug, Clone)]
pub struct LuaCreatureSnapshot {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub map_id: u32,
    pub instance_id: u32,

    // Position
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
    pub spawn_x: f32,
    pub spawn_y: f32,
    pub spawn_z: f32,
    pub spawn_o: f32,

    // Stats
    pub health: u32,
    pub max_health: u32,
    pub power: u32,
    pub max_power: u32,
    pub power_type: u8,
    pub level: u8,

    // Combat state
    pub is_alive: bool,
    pub is_in_combat: bool,
    pub current_target: Option<ObjectGuid>,
    pub is_casting: bool,

    // AI state
    pub phase: u32,
    pub timers: HashMap<u32, u32>, // timer_id -> remaining_ms
    pub custom_data: HashMap<String, i64>,
    pub combat_time_ms: u32,

    // Threat list
    pub threat_list: Vec<ThreatEntry>,

    // Events since last update
    pub events: Vec<AIEvent>,

    // Time since last update
    pub diff_ms: u32,

    // Auras: (spell_id, remaining_ms, stacks)
    pub auras: Vec<(u32, u32, u8)>,

    // Instance data (if creature is in an instance with a script)
    pub instance_data: HashMap<u32, u32>,

    // Nearby creatures (for add coordination, entity queries)
    pub nearby_creatures: Vec<NearbyCreatureEntry>,

    // Summoned creatures
    pub summoned_creatures: Vec<ObjectGuid>,
}

impl LuaCreatureSnapshot {
    pub fn health_pct(&self) -> f32 {
        if self.max_health == 0 {
            0.0
        } else {
            self.health as f32 / self.max_health as f32
        }
    }

    pub fn power_pct(&self) -> f32 {
        if self.max_power == 0 {
            0.0
        } else {
            self.power as f32 / self.max_power as f32
        }
    }

    pub fn is_timer_ready(&self, timer_id: u32) -> bool {
        self.timers.get(&timer_id).map(|&v| v == 0).unwrap_or(true)
    }

    pub fn get_timer(&self, timer_id: u32) -> u32 {
        self.timers.get(&timer_id).copied().unwrap_or(0)
    }

    pub fn get_custom_data(&self, key: &str) -> i64 {
        self.custom_data.get(key).copied().unwrap_or(0)
    }

    pub fn has_aura(&self, spell_id: u32) -> bool {
        self.auras.iter().any(|(id, _, _)| *id == spell_id)
    }

    pub fn get_highest_threat_target(&self) -> Option<&ThreatEntry> {
        self.threat_list.first()
    }
}

impl Default for LuaCreatureSnapshot {
    fn default() -> Self {
        Self {
            guid: ObjectGuid::empty(),
            entry: 0,
            map_id: 0,
            instance_id: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            o: 0.0,
            spawn_x: 0.0,
            spawn_y: 0.0,
            spawn_z: 0.0,
            spawn_o: 0.0,
            health: 0,
            max_health: 0,
            power: 0,
            max_power: 0,
            power_type: 0,
            level: 1,
            is_alive: true,
            is_in_combat: false,
            current_target: None,
            is_casting: false,
            phase: 0,
            timers: HashMap::new(),
            custom_data: HashMap::new(),
            combat_time_ms: 0,
            threat_list: Vec::new(),
            events: Vec::new(),
            diff_ms: 0,
            auras: Vec::new(),
            instance_data: HashMap::new(),
            nearby_creatures: Vec::new(),
            summoned_creatures: Vec::new(),
        }
    }
}

/// Entry in the threat list.
#[derive(Debug, Clone)]
pub struct ThreatEntry {
    pub guid: ObjectGuid,
    pub threat: f32,
    pub distance: f32,
    pub is_player: bool,
    pub health_pct: f32,
    pub has_mana: bool,
}

/// Info about a nearby creature (for entity queries in scripts).
#[derive(Debug, Clone)]
pub struct NearbyCreatureEntry {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub distance: f32,
    pub is_alive: bool,
}

/// AI event that occurred since last update.
#[derive(Debug, Clone)]
pub enum AIEvent {
    DamageTaken {
        attacker: ObjectGuid,
        damage: u32,
        spell_id: Option<u32>,
    },
    SpellHit {
        caster: ObjectGuid,
        spell_id: u32,
    },
    HealingReceived {
        healer: ObjectGuid,
        amount: u32,
        spell_id: u32,
    },
    CombatStarted,
    CombatEnded,
    TargetKilled {
        victim: ObjectGuid,
    },
    Spawned,
    Died,
}

/// A snapshot of player state for gossip scripts.
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub guid: ObjectGuid,
    pub name: String,
    pub level: u8,
    pub class: u8,
    pub race: u8,
    pub faction: u32,
    pub gold: u32, // in copper
    pub health: u32,
    pub max_health: u32,
    pub map_id: u32,
    pub zone_id: u32,
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            guid: ObjectGuid::empty(),
            name: String::new(),
            level: 1,
            class: 0,
            race: 0,
            faction: 0,
            gold: 0,
            health: 0,
            max_health: 0,
            map_id: 0,
            zone_id: 0,
        }
    }
}

/// A snapshot of instance state.
#[derive(Debug, Clone)]
pub struct InstanceSnapshot {
    pub map_id: u32,
    pub instance_id: u32,
    pub difficulty: u8,
    pub player_count: u32,
    pub data: HashMap<u32, u32>,
    pub guids: HashMap<u32, ObjectGuid>,
    pub timers: HashMap<u32, u32>,
}

impl Default for InstanceSnapshot {
    fn default() -> Self {
        Self {
            map_id: 0,
            instance_id: 0,
            difficulty: 0,
            player_count: 0,
            data: HashMap::new(),
            guids: HashMap::new(),
            timers: HashMap::new(),
        }
    }
}

/// A snapshot of zone state.
#[derive(Debug, Clone)]
pub struct ZoneSnapshot {
    pub map_id: u32,
    pub zone_id: u32,
    pub area_id: u32,
    pub player_count: u32,
    pub data: HashMap<u32, u32>,
    pub guids: HashMap<u32, ObjectGuid>,
    pub timers: HashMap<u32, u32>,
    pub diff_ms: u32,
}

impl Default for ZoneSnapshot {
    fn default() -> Self {
        Self {
            map_id: 0,
            zone_id: 0,
            area_id: 0,
            player_count: 0,
            data: HashMap::new(),
            guids: HashMap::new(),
            timers: HashMap::new(),
            diff_ms: 0,
        }
    }
}

/// Generic snapshot type for Lua scripts.
#[derive(Debug, Clone)]
pub enum LuaSnapshot {
    Creature(LuaCreatureSnapshot),
    Player(PlayerSnapshot),
    Instance(InstanceSnapshot),
    Zone(ZoneSnapshot),
}

// ==================== Lua Conversions ====================

impl LuaCreatureSnapshot {
    /// Convert to a Lua table.
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let table = lua.create_table()?;

        // Basic info
        table.set("guid", LuaGuid(self.guid))?;
        table.set("entry", self.entry)?;
        table.set("map_id", self.map_id)?;
        table.set("instance_id", self.instance_id)?;

        // Position
        let position = lua.create_table()?;
        position.set("x", self.x)?;
        position.set("y", self.y)?;
        position.set("z", self.z)?;
        position.set("o", self.o)?;
        table.set("position", position)?;

        let spawn_position = lua.create_table()?;
        spawn_position.set("x", self.spawn_x)?;
        spawn_position.set("y", self.spawn_y)?;
        spawn_position.set("z", self.spawn_z)?;
        spawn_position.set("o", self.spawn_o)?;
        table.set("spawn_position", spawn_position)?;

        // Stats
        table.set("health", self.health)?;
        table.set("max_health", self.max_health)?;
        table.set("health_pct", self.health_pct())?;
        table.set("power", self.power)?;
        table.set("max_power", self.max_power)?;
        table.set("power_pct", self.power_pct())?;
        table.set("power_type", self.power_type)?;
        table.set("level", self.level)?;

        // Combat state
        table.set("is_alive", self.is_alive)?;
        table.set("is_in_combat", self.is_in_combat)?;
        table.set("is_casting", self.is_casting)?;
        if let Some(target) = self.current_target {
            table.set("current_target", LuaGuid(target))?;
        }

        // AI state
        table.set("phase", self.phase)?;
        table.set("combat_time_ms", self.combat_time_ms)?;
        table.set("diff_ms", self.diff_ms)?;

        // Timers
        let timers_table = lua.create_table()?;
        for (&id, &remaining) in &self.timers {
            timers_table.set(id, remaining)?;
        }
        table.set("timers", timers_table)?;

        // Custom data
        let custom_data_table = lua.create_table()?;
        for (key, &value) in &self.custom_data {
            custom_data_table.set(key.as_str(), value)?;
        }
        table.set("custom_data", custom_data_table)?;

        // Threat list
        let threat_table = lua.create_table()?;
        for (i, entry) in self.threat_list.iter().enumerate() {
            let entry_table = lua.create_table()?;
            entry_table.set("guid", LuaGuid(entry.guid))?;
            entry_table.set("threat", entry.threat)?;
            entry_table.set("distance", entry.distance)?;
            entry_table.set("is_player", entry.is_player)?;
            entry_table.set("health_pct", entry.health_pct)?;
            entry_table.set("has_mana", entry.has_mana)?;
            threat_table.set(i + 1, entry_table)?;
        }
        table.set("threat_list", threat_table)?;

        // Events
        let events_table = lua.create_table()?;
        for (i, event) in self.events.iter().enumerate() {
            let event_table = event.to_lua_table(lua)?;
            events_table.set(i + 1, event_table)?;
        }
        table.set("events", events_table)?;

        // Auras
        let auras_table = lua.create_table()?;
        for (i, &(spell_id, remaining_ms, stacks)) in self.auras.iter().enumerate() {
            let aura_table = lua.create_table()?;
            aura_table.set("spell_id", spell_id)?;
            aura_table.set("remaining_ms", remaining_ms)?;
            aura_table.set("stacks", stacks)?;
            auras_table.set(i + 1, aura_table)?;
        }
        table.set("auras", auras_table)?;

        // HasAura helper
        let auras_clone = self.auras.clone();
        let has_aura = lua.create_function(move |_, args: mlua::Variadic<Value>| {
            let spell_id: u32 = if args.len() >= 2 {
                match &args[1] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(false),
                }
            } else if args.len() == 1 {
                match &args[0] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(false),
                }
            } else {
                return Ok(false);
            };
            Ok(auras_clone.iter().any(|(id, _, _)| *id == spell_id))
        })?;
        table.set("HasAura", has_aura)?;

        // Nearby creatures
        let nearby_table = lua.create_table()?;
        for (i, nc) in self.nearby_creatures.iter().enumerate() {
            let nc_table = lua.create_table()?;
            nc_table.set("guid", LuaGuid(nc.guid))?;
            nc_table.set("entry", nc.entry)?;
            nc_table.set("distance", nc.distance)?;
            nc_table.set("is_alive", nc.is_alive)?;
            nearby_table.set(i + 1, nc_table)?;
        }
        table.set("nearby_creatures", nearby_table)?;

        // GetCreaturesByEntry helper
        let nearby_clone = self.nearby_creatures.clone();
        let get_creatures_by_entry = lua.create_function(move |lua, args: mlua::Variadic<Value>| {
            let entry: u32 = if args.len() >= 2 {
                match &args[1] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(Value::Table(lua.create_table()?)),
                }
            } else if args.len() == 1 {
                match &args[0] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(Value::Table(lua.create_table()?)),
                }
            } else {
                return Ok(Value::Table(lua.create_table()?));
            };

            let result = lua.create_table()?;
            let mut idx = 1;
            for nc in &nearby_clone {
                if nc.entry == entry {
                    let t = lua.create_table()?;
                    t.set("guid", LuaGuid(nc.guid))?;
                    t.set("entry", nc.entry)?;
                    t.set("distance", nc.distance)?;
                    t.set("is_alive", nc.is_alive)?;
                    result.set(idx, t)?;
                    idx += 1;
                }
            }
            Ok(Value::Table(result))
        })?;
        table.set("GetCreaturesByEntry", get_creatures_by_entry)?;

        // Instance data
        let instance_data_table = lua.create_table()?;
        for (&id, &value) in &self.instance_data {
            instance_data_table.set(id, value)?;
        }
        table.set("instance_data", instance_data_table)?;

        // GetInstanceData helper
        let instance_data_clone = self.instance_data.clone();
        let get_instance_data = lua.create_function(move |_, args: mlua::Variadic<Value>| {
            let data_id: u32 = if args.len() >= 2 {
                match &args[1] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(0u32),
                }
            } else if args.len() == 1 {
                match &args[0] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(0u32),
                }
            } else {
                return Ok(0u32);
            };
            Ok(instance_data_clone.get(&data_id).copied().unwrap_or(0))
        })?;
        table.set("GetInstanceData", get_instance_data)?;

        // Summoned creatures
        let summoned_table = lua.create_table()?;
        for (i, &guid) in self.summoned_creatures.iter().enumerate() {
            summoned_table.set(i + 1, LuaGuid(guid))?;
        }
        table.set("summoned_creatures", summoned_table)?;

        // Helper methods - use snapshot data captured in closure
        // Note: These methods support both dot and colon syntax:
        //   input.IsTimerReady(1) - first arg is timer_id
        //   input:IsTimerReady(1) - first arg is self (table), second is timer_id
        let timers_clone = self.timers.clone();
        let is_timer_ready = lua.create_function(move |_, args: mlua::Variadic<Value>| {
            // Handle both input:IsTimerReady(id) and input.IsTimerReady(id)
            let timer_id: u32 = if args.len() >= 2 {
                // Colon syntax: first arg is self (table), second is timer_id
                match &args[1] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(true), // Invalid arg, return true (ready)
                }
            } else if args.len() == 1 {
                // Dot syntax: first arg is timer_id
                match &args[0] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(true),
                }
            } else {
                return Ok(true);
            };
            Ok(timers_clone.get(&timer_id).map(|&v| v == 0).unwrap_or(true))
        })?;
        table.set("IsTimerReady", is_timer_ready)?;

        let timers_clone2 = self.timers.clone();
        let get_timer = lua.create_function(move |_, args: mlua::Variadic<Value>| {
            let timer_id: u32 = if args.len() >= 2 {
                match &args[1] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(0),
                }
            } else if args.len() == 1 {
                match &args[0] {
                    Value::Integer(n) => *n as u32,
                    Value::Number(n) => *n as u32,
                    _ => return Ok(0),
                }
            } else {
                return Ok(0);
            };
            Ok(timers_clone2.get(&timer_id).copied().unwrap_or(0))
        })?;
        table.set("GetTimer", get_timer)?;

        let custom_data_clone = self.custom_data.clone();
        let get_custom_data = lua.create_function(move |_, args: mlua::Variadic<Value>| {
            let key: String = if args.len() >= 2 {
                match &args[1] {
                    Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
                    _ => return Ok(0),
                }
            } else if args.len() == 1 {
                match &args[0] {
                    Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
                    _ => return Ok(0),
                }
            } else {
                return Ok(0);
            };
            Ok(custom_data_clone.get(&key).copied().unwrap_or(0))
        })?;
        table.set("GetCustomData", get_custom_data)?;

        let threat_list_clone = self.threat_list.clone();
        let get_highest_threat_target =
            lua.create_function(move |lua, _args: mlua::Variadic<Value>| {
                if let Some(entry) = threat_list_clone.first() {
                    let t = lua.create_table()?;
                    t.set("guid", LuaGuid(entry.guid))?;
                    t.set("threat", entry.threat)?;
                    t.set("distance", entry.distance)?;
                    t.set("is_player", entry.is_player)?;
                    t.set("health_pct", entry.health_pct)?;
                    Ok(Value::Table(t))
                } else {
                    Ok(Value::Nil)
                }
            })?;
        table.set("GetHighestThreatTarget", get_highest_threat_target)?;

        Ok(table)
    }
}

impl AIEvent {
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let table = lua.create_table()?;

        match self {
            AIEvent::DamageTaken {
                attacker,
                damage,
                spell_id,
            } => {
                table.set("type", "DAMAGE_TAKEN")?;
                table.set("attacker", LuaGuid(*attacker))?;
                table.set("damage", *damage)?;
                if let Some(spell) = spell_id {
                    table.set("spell_id", *spell)?;
                }
            }
            AIEvent::SpellHit { caster, spell_id } => {
                table.set("type", "SPELL_HIT")?;
                table.set("caster", LuaGuid(*caster))?;
                table.set("spell_id", *spell_id)?;
            }
            AIEvent::HealingReceived {
                healer,
                amount,
                spell_id,
            } => {
                table.set("type", "HEALING_RECEIVED")?;
                table.set("healer", LuaGuid(*healer))?;
                table.set("amount", *amount)?;
                table.set("spell_id", *spell_id)?;
            }
            AIEvent::CombatStarted => {
                table.set("type", "COMBAT_STARTED")?;
            }
            AIEvent::CombatEnded => {
                table.set("type", "COMBAT_ENDED")?;
            }
            AIEvent::TargetKilled { victim } => {
                table.set("type", "TARGET_KILLED")?;
                table.set("victim", LuaGuid(*victim))?;
            }
            AIEvent::Spawned => {
                table.set("type", "SPAWNED")?;
            }
            AIEvent::Died => {
                table.set("type", "DIED")?;
            }
        }

        Ok(table)
    }
}

impl PlayerSnapshot {
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let table = lua.create_table()?;

        table.set("guid", LuaGuid(self.guid))?;
        table.set("name", self.name.as_str())?;
        table.set("level", self.level)?;
        table.set("class", self.class)?;
        table.set("race", self.race)?;
        table.set("faction", self.faction)?;
        table.set("gold", self.gold)?;
        table.set("health", self.health)?;
        table.set("max_health", self.max_health)?;
        table.set("map_id", self.map_id)?;
        table.set("zone_id", self.zone_id)?;

        Ok(table)
    }
}

impl InstanceSnapshot {
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let table = lua.create_table()?;

        table.set("map_id", self.map_id)?;
        table.set("instance_id", self.instance_id)?;
        table.set("difficulty", self.difficulty)?;
        table.set("player_count", self.player_count)?;

        let data_table = lua.create_table()?;
        for (&id, &value) in &self.data {
            data_table.set(id, value)?;
        }
        table.set("data", data_table)?;

        let timers_table = lua.create_table()?;
        for (&id, &remaining) in &self.timers {
            timers_table.set(id, remaining)?;
        }
        table.set("timers", timers_table)?;

        let data_clone = self.data.clone();
        let get_data = lua.create_function(move |_, data_id: u32| {
            Ok(data_clone.get(&data_id).copied().unwrap_or(0))
        })?;
        table.set("GetData", get_data)?;

        let guids_clone = self.guids.clone();
        let get_guid = lua.create_function(move |_, data_id: u32| {
            Ok(guids_clone.get(&data_id).map(|g| LuaGuid(*g)))
        })?;
        table.set("GetGuid", get_guid)?;

        let timers_clone = self.timers.clone();
        let is_timer_ready = lua.create_function(move |_, timer_id: u32| {
            Ok(timers_clone.get(&timer_id).map(|&v| v == 0).unwrap_or(true))
        })?;
        table.set("IsTimerReady", is_timer_ready)?;

        Ok(table)
    }
}

impl ZoneSnapshot {
    pub fn to_lua_table(&self, lua: &Lua) -> LuaResult<Table> {
        let table = lua.create_table()?;

        table.set("map_id", self.map_id)?;
        table.set("zone_id", self.zone_id)?;
        table.set("area_id", self.area_id)?;
        table.set("player_count", self.player_count)?;
        table.set("diff_ms", self.diff_ms)?;

        let data_clone = self.data.clone();
        let get_data = lua.create_function(move |_, data_id: u32| {
            Ok(data_clone.get(&data_id).copied().unwrap_or(0))
        })?;
        table.set("GetData", get_data)?;

        let guids_clone = self.guids.clone();
        let get_guid = lua.create_function(move |_, data_id: u32| {
            Ok(guids_clone.get(&data_id).map(|g| LuaGuid(*g)))
        })?;
        table.set("GetGuid", get_guid)?;

        let timers_clone = self.timers.clone();
        let is_timer_ready = lua.create_function(move |_, timer_id: u32| {
            Ok(timers_clone.get(&timer_id).map(|&v| v == 0).unwrap_or(true))
        })?;
        table.set("IsTimerReady", is_timer_ready)?;

        Ok(table)
    }
}

// ==================== GUID UserData ====================

/// Wrapper for ObjectGuid to expose to Lua.
#[derive(Debug, Clone, Copy)]
pub struct LuaGuid(pub ObjectGuid);

impl UserData for LuaGuid {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("raw", |_, this| Ok(this.0.raw()));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("IsEmpty", |_, this, ()| Ok(this.0.is_empty()));

        methods.add_meta_method(mlua::MetaMethod::Eq, |_, this, other: LuaGuid| {
            Ok(this.0 == other.0)
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, ()| {
            Ok(format!("GUID(0x{:016X})", this.0.raw()))
        });
    }
}

impl mlua::FromLua for LuaGuid {
    fn from_lua(value: Value, _lua: &Lua) -> LuaResult<Self> {
        match value {
            Value::UserData(ud) => ud.borrow::<LuaGuid>().map(|g| *g),
            Value::Integer(n) => Ok(LuaGuid(ObjectGuid::from_raw(n as u64))),
            Value::Number(n) => Ok(LuaGuid(ObjectGuid::from_raw(n as u64))),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("LuaGuid"),
                message: Some(String::from("expected GUID userdata or integer")),
            }),
        }
    }
}
