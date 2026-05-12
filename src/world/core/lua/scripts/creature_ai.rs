//! Lua creature AI script handler.
//!
//! Handles calling Lua callbacks for creature AI (OnEnterCombat, OnUpdate, etc.)
//! and parsing the returned action tables.

use super::super::super::common::ObjectGuid;
use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::{LuaCreatureSnapshot, LuaGuid};
use mlua::{Function, Lua, Table, Value};
use std::collections::HashMap;

/// Lua creature AI script handler.
///
/// Wraps a Lua script table and provides methods to call its callbacks.
pub struct LuaCreatureAI {
    entry: u32,
}

impl LuaCreatureAI {
    pub fn new(entry: u32) -> Self {
        Self { entry }
    }

    pub fn entry(&self) -> u32 {
        self.entry
    }

    /// Call OnEnterCombat callback.
    pub fn on_enter_combat(&self, lua: &Lua, snapshot: &LuaCreatureSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "OnEnterCombat", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnEnterCombat")?;
            func.call((table.clone(), input))
        })
    }

    /// Call OnUpdate callback (called every tick).
    pub fn on_update(&self, lua: &Lua, snapshot: &LuaCreatureSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "OnUpdate", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnUpdate")?;
            func.call((table.clone(), input))
        })
    }

    /// Call OnDeath callback.
    pub fn on_death(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        killer_guid: Option<ObjectGuid>,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnDeath", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnDeath")?;
            let killer = killer_guid.map(LuaGuid);
            func.call((table.clone(), input, killer))
        })
    }

    /// Call OnKill callback.
    pub fn on_kill(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        victim_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnKill", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnKill")?;
            func.call((table.clone(), input, LuaGuid(victim_guid)))
        })
    }

    /// Call OnSpellHit callback.
    pub fn on_spell_hit(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        spell_id: u32,
        caster_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnSpellHit", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnSpellHit")?;
            func.call((table.clone(), input, spell_id, LuaGuid(caster_guid)))
        })
    }

    /// Call OnDamageTaken callback.
    pub fn on_damage_taken(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        attacker_guid: ObjectGuid,
        damage: u32,
        spell_id: Option<u32>,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "OnDamageTaken", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnDamageTaken")?;
            func.call((
                table.clone(),
                input,
                LuaGuid(attacker_guid),
                damage,
                spell_id,
            ))
        })
    }

    /// Call OnEvade callback.
    pub fn on_evade(&self, lua: &Lua, snapshot: &LuaCreatureSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "OnEvade", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnEvade")?;
            func.call((table.clone(), input))
        })
    }

    /// Call OnReset callback.
    pub fn on_reset(&self, lua: &Lua, snapshot: &LuaCreatureSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "OnReset", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnReset")?;
            func.call((table.clone(), input))
        })
    }

    /// Call OnSpawn callback.
    pub fn on_spawn(&self, lua: &Lua, snapshot: &LuaCreatureSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "OnSpawn", snapshot, |_lua, table, input| {
            let func: Function = table.get("OnSpawn")?;
            func.call((table.clone(), input))
        })
    }

    /// Call HealedBy callback.
    pub fn on_healed_by(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        healer_guid: ObjectGuid,
        amount: u32,
        spell_id: Option<u32>,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "HealedBy", snapshot, |_lua, table, input| {
            let func: Function = table.get("HealedBy")?;
            func.call((table.clone(), input, LuaGuid(healer_guid), amount, spell_id))
        })
    }

    /// Call SpellHitTarget callback.
    pub fn on_spell_hit_target(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        target_guid: ObjectGuid,
        spell_id: u32,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "SpellHitTarget", snapshot, |_lua, table, input| {
            let func: Function = table.get("SpellHitTarget")?;
            func.call((table.clone(), input, LuaGuid(target_guid), spell_id))
        })
    }

    /// Call MovementInform callback.
    pub fn on_movement_inform(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        movement_type: u32,
        point_id: u32,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "MovementInform", snapshot, |_lua, table, input| {
            let func: Function = table.get("MovementInform")?;
            func.call((table.clone(), input, movement_type, point_id))
        })
    }

    /// Call JustReachedHome callback.
    pub fn on_just_reached_home(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
    ) -> Vec<LuaAction> {
        self.call_callback(lua, "JustReachedHome", snapshot, |_lua, table, input| {
            let func: Function = table.get("JustReachedHome")?;
            func.call((table.clone(), input))
        })
    }

    /// Call JustRespawned callback.
    pub fn on_just_respawned(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
    ) -> Vec<LuaAction> {
        self.call_callback(lua, "JustRespawned", snapshot, |_lua, table, input| {
            let func: Function = table.get("JustRespawned")?;
            func.call((table.clone(), input))
        })
    }

    /// Call MoveInLineOfSight callback.
    pub fn on_move_in_line_of_sight(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        unit_guid: ObjectGuid,
        is_hostile: bool,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(
            lua,
            "MoveInLineOfSight",
            snapshot,
            |_lua, table, input| {
                let func: Function = table.get("MoveInLineOfSight")?;
                func.call((table.clone(), input, LuaGuid(unit_guid), is_hostile))
            },
        )
    }

    /// Call JustSummoned callback.
    pub fn on_just_summoned(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        summoned_guid: ObjectGuid,
        entry: u32,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(lua, "JustSummoned", snapshot, |_lua, table, input| {
            let func: Function = table.get("JustSummoned")?;
            func.call((table.clone(), input, LuaGuid(summoned_guid), entry))
        })
    }

    /// Call SummonedCreatureJustDied callback.
    pub fn on_summoned_creature_just_died(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        summoned_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(
            lua,
            "SummonedCreatureJustDied",
            snapshot,
            |_lua, table, input| {
                let func: Function = table.get("SummonedCreatureJustDied")?;
                func.call((table.clone(), input, LuaGuid(summoned_guid)))
            },
        )
    }

    /// Call SummonedCreatureDespawn callback.
    pub fn on_summoned_creature_despawn(
        &self,
        lua: &Lua,
        snapshot: &LuaCreatureSnapshot,
        summoned_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback_with_extra(
            lua,
            "SummonedCreatureDespawn",
            snapshot,
            |_lua, table, input| {
                let func: Function = table.get("SummonedCreatureDespawn")?;
                func.call((table.clone(), input, LuaGuid(summoned_guid)))
            },
        )
    }

    /// Get the script table from Lua registry.
    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("creature_ai_{}", self.entry);
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
        snapshot: &LuaCreatureSnapshot,
        f: F,
    ) -> Vec<LuaAction>
    where
        F: FnOnce(&Lua, &Table, Table) -> mlua::Result<Value>,
    {
        let table = match self.get_script_table(lua) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Creature AI script for entry {} not found in registry",
                    self.entry
                );
                return Vec::new();
            }
        };

        // Check if callback exists
        if table.get::<Function>(callback).is_err() {
            return Vec::new();
        }

        // Create input table from snapshot
        let input = match snapshot.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create input table for creature {}: {}",
                    self.entry,
                    e
                );
                return Vec::new();
            }
        };

        // Call the callback
        match f(lua, &table, input) {
            Ok(result) => parse_actions(result),
            Err(e) => {
                tracing::error!(
                    "Error in creature AI script {} callback {}: {}",
                    self.entry,
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
        snapshot: &LuaCreatureSnapshot,
        f: F,
    ) -> Vec<LuaAction>
    where
        F: FnOnce(&Lua, &Table, Table) -> mlua::Result<Value>,
    {
        // Same implementation, just different signature for clarity
        self.call_callback(lua, callback, snapshot, f)
    }
}

/// State stored per creature instance for timer and custom data tracking.
#[derive(Debug, Clone, Default)]
pub struct CreatureScriptState {
    pub phase: u32,
    pub timers: HashMap<u32, u32>,
    pub custom_data: HashMap<String, i64>,
    pub combat_time_ms: u32,
    pub summoned_creatures: Vec<ObjectGuid>,
}

impl CreatureScriptState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update timers by the elapsed time.
    pub fn update_timers(&mut self, diff_ms: u32) {
        for timer in self.timers.values_mut() {
            *timer = timer.saturating_sub(diff_ms);
        }
    }

    /// Check if a timer is ready (reached 0).
    pub fn is_timer_ready(&self, timer_id: u32) -> bool {
        self.timers.get(&timer_id).map(|&v| v == 0).unwrap_or(true)
    }

    /// Set a timer.
    pub fn set_timer(&mut self, timer_id: u32, duration_ms: u32) {
        self.timers.insert(timer_id, duration_ms);
    }

    /// Set phase.
    pub fn set_phase(&mut self, phase: u32) {
        self.phase = phase;
    }

    /// Set custom data.
    pub fn set_custom_data(&mut self, key: String, value: i64) {
        self.custom_data.insert(key, value);
    }

    /// Get custom data.
    pub fn get_custom_data(&self, key: &str) -> i64 {
        self.custom_data.get(key).copied().unwrap_or(0)
    }

    /// Update combat time.
    pub fn add_combat_time(&mut self, diff_ms: u32) {
        self.combat_time_ms = self.combat_time_ms.saturating_add(diff_ms);
    }

    /// Reset for new combat.
    pub fn reset_combat(&mut self) {
        self.combat_time_ms = 0;
    }

    /// Add a summoned creature.
    pub fn add_summoned(&mut self, guid: ObjectGuid) {
        if !self.summoned_creatures.contains(&guid) {
            self.summoned_creatures.push(guid);
        }
    }

    /// Remove a summoned creature.
    pub fn remove_summoned(&mut self, guid: ObjectGuid) {
        self.summoned_creatures.retain(|&g| g != guid);
    }

    /// Get all summoned creatures.
    pub fn get_summoned(&self) -> &[ObjectGuid] {
        &self.summoned_creatures
    }
}
