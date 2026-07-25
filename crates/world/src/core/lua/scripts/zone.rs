//! Lua zone script handler.
//!
//! Zone scripts registered via `RegisterZoneScript(zone_id, table)` receive
//! callbacks as players move between zones and on a periodic tick. They keep
//! per-zone state through the same SetData/GetData pattern as instance scripts,
//! which world-event and outdoor-PvP scripts build on.

use super::super::super::common::ObjectGuid;
use super::super::actions::{parse_actions, LuaAction};
use super::super::snapshot::{LuaGuid, ZoneSnapshot};
use mlua::{Function, Lua, Table, Value};
use std::collections::HashMap;

/// Lua zone script handler.
///
/// Wraps a Lua script table registered via `RegisterZoneScript(zone_id, table)`.
pub struct LuaZoneScript {
    zone_id: u32,
}

impl LuaZoneScript {
    pub fn new(zone_id: u32) -> Self {
        Self { zone_id }
    }

    pub fn zone_id(&self) -> u32 {
        self.zone_id
    }

    /// Call `OnPlayerEnter` — a player moved into this zone.
    pub fn on_player_enter(
        &self,
        lua: &Lua,
        snapshot: &ZoneSnapshot,
        player_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback(lua, "OnPlayerEnter", snapshot, |table, input| {
            let func: Function = table.get("OnPlayerEnter")?;
            func.call((table.clone(), input, LuaGuid(player_guid)))
        })
    }

    /// Call `OnPlayerLeave` — a player moved out of this zone.
    pub fn on_player_leave(
        &self,
        lua: &Lua,
        snapshot: &ZoneSnapshot,
        player_guid: ObjectGuid,
    ) -> Vec<LuaAction> {
        self.call_callback(lua, "OnPlayerLeave", snapshot, |table, input| {
            let func: Function = table.get("OnPlayerLeave")?;
            func.call((table.clone(), input, LuaGuid(player_guid)))
        })
    }

    /// Call `Update` — the periodic zone tick.
    pub fn on_update(&self, lua: &Lua, snapshot: &ZoneSnapshot) -> Vec<LuaAction> {
        self.call_callback(lua, "Update", snapshot, |table, input| {
            let func: Function = table.get("Update")?;
            func.call((table.clone(), input))
        })
    }

    /// Get the script table from the Lua registry.
    fn get_script_table(&self, lua: &Lua) -> Option<Table> {
        let key = format!("zone_{}", self.zone_id);
        lua.named_registry_value::<Table>(&key).ok()
    }

    /// Check if a callback exists.
    pub fn has_callback(&self, lua: &Lua, callback: &str) -> bool {
        match self.get_script_table(lua) {
            Some(table) => table.get::<Function>(callback).is_ok(),
            None => false,
        }
    }

    /// Invoke a callback with the zone snapshot, returning the parsed actions.
    ///
    /// A missing callback is not an error: scripts implement only what they need.
    fn call_callback<F>(
        &self,
        lua: &Lua,
        callback: &str,
        snapshot: &ZoneSnapshot,
        f: F,
    ) -> Vec<LuaAction>
    where
        F: FnOnce(&Table, Table) -> mlua::Result<Value>,
    {
        let Some(table) = self.get_script_table(lua) else {
            tracing::warn!(
                "Zone script for zone {} not found in registry",
                self.zone_id
            );
            return Vec::new();
        };

        if table.get::<Function>(callback).is_err() {
            return Vec::new();
        }

        let input = match snapshot.to_lua_table(lua) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to create input table for zone {}: {}",
                    self.zone_id,
                    e
                );
                return Vec::new();
            }
        };

        match f(&table, input) {
            Ok(result) => parse_actions(result),
            Err(e) => {
                tracing::error!(
                    "Error in zone script {} callback {}: {}",
                    self.zone_id,
                    callback,
                    e
                );
                Vec::new()
            }
        }
    }
}

/// State stored per zone for data tracking.
#[derive(Debug, Clone, Default)]
pub struct ZoneScriptState {
    /// Zone data values (event states, capture progress, counters).
    pub data: HashMap<u32, u32>,
    /// GUIDs stored by data id (for tracking specific creatures or objects).
    pub guids: HashMap<u32, ObjectGuid>,
    /// Custom countdown timers, in milliseconds.
    pub timers: HashMap<u32, u32>,
    /// Players currently inside the zone.
    pub players: Vec<ObjectGuid>,
}

impl ZoneScriptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_data(&mut self, data_id: u32, value: u32) {
        self.data.insert(data_id, value);
    }

    pub fn get_data(&self, data_id: u32) -> u32 {
        self.data.get(&data_id).copied().unwrap_or(0)
    }

    pub fn set_guid(&mut self, data_id: u32, guid: ObjectGuid) {
        self.guids.insert(data_id, guid);
    }

    pub fn get_guid(&self, data_id: u32) -> Option<ObjectGuid> {
        self.guids.get(&data_id).copied()
    }

    pub fn set_timer(&mut self, timer_id: u32, duration_ms: u32) {
        self.timers.insert(timer_id, duration_ms);
    }

    pub fn is_timer_ready(&self, timer_id: u32) -> bool {
        self.timers.get(&timer_id).map(|&v| v == 0).unwrap_or(true)
    }

    pub fn update_timers(&mut self, diff_ms: u32) {
        for timer in self.timers.values_mut() {
            *timer = timer.saturating_sub(diff_ms);
        }
    }

    /// Record a player as present. Returns false if already tracked.
    pub fn add_player(&mut self, guid: ObjectGuid) -> bool {
        if self.players.contains(&guid) {
            return false;
        }
        self.players.push(guid);
        true
    }

    /// Drop a player. Returns false if they were not tracked.
    pub fn remove_player(&mut self, guid: ObjectGuid) -> bool {
        match self.players.iter().position(|p| *p == guid) {
            Some(idx) => {
                self.players.swap_remove(idx);
                true
            }
            None => false,
        }
    }

    pub fn player_count(&self) -> u32 {
        self.players.len() as u32
    }

    /// Build a `ZoneSnapshot` from the current state.
    pub fn to_snapshot(
        &self,
        map_id: u32,
        zone_id: u32,
        area_id: u32,
        diff_ms: u32,
    ) -> ZoneSnapshot {
        ZoneSnapshot {
            map_id,
            zone_id,
            area_id,
            player_count: self.player_count(),
            data: self.data.clone(),
            guids: self.guids.clone(),
            timers: self.timers.clone(),
            diff_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::HighGuid;

    fn guid(counter: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, counter)
    }

    #[test]
    fn data_defaults_to_zero_and_round_trips() {
        let mut state = ZoneScriptState::new();
        assert_eq!(state.get_data(5), 0);

        state.set_data(5, 42);
        assert_eq!(state.get_data(5), 42);
    }

    #[test]
    fn guids_round_trip() {
        let mut state = ZoneScriptState::new();
        assert_eq!(state.get_guid(1), None);

        state.set_guid(1, guid(7));
        assert_eq!(state.get_guid(1), Some(guid(7)));
    }

    #[test]
    fn unset_timer_reads_as_ready() {
        let mut state = ZoneScriptState::new();
        assert!(state.is_timer_ready(1));

        state.set_timer(1, 5000);
        assert!(!state.is_timer_ready(1));
    }

    #[test]
    fn timers_count_down_and_saturate_at_zero() {
        let mut state = ZoneScriptState::new();
        state.set_timer(1, 1000);

        state.update_timers(400);
        assert!(!state.is_timer_ready(1));

        state.update_timers(5000);
        assert!(state.is_timer_ready(1));
    }

    #[test]
    fn player_tracking_ignores_duplicates() {
        let mut state = ZoneScriptState::new();

        assert!(state.add_player(guid(1)));
        assert!(!state.add_player(guid(1)));
        assert_eq!(state.player_count(), 1);

        assert!(state.add_player(guid(2)));
        assert_eq!(state.player_count(), 2);
    }

    #[test]
    fn removing_untracked_player_reports_false() {
        let mut state = ZoneScriptState::new();
        assert!(!state.remove_player(guid(1)));

        state.add_player(guid(1));
        assert!(state.remove_player(guid(1)));
        assert_eq!(state.player_count(), 0);
    }

    #[test]
    fn snapshot_carries_state_and_tick_delta() {
        let mut state = ZoneScriptState::new();
        state.set_data(1, 10);
        state.set_guid(2, guid(3));
        state.set_timer(4, 500);
        state.add_player(guid(1));
        state.add_player(guid(2));

        let snap = state.to_snapshot(0, 1519, 1537, 250);

        assert_eq!(snap.map_id, 0);
        assert_eq!(snap.zone_id, 1519);
        assert_eq!(snap.area_id, 1537);
        assert_eq!(snap.player_count, 2);
        assert_eq!(snap.diff_ms, 250);
        assert_eq!(snap.data.get(&1), Some(&10));
        assert_eq!(snap.guids.get(&2), Some(&guid(3)));
        assert_eq!(snap.timers.get(&4), Some(&500));
    }
}
