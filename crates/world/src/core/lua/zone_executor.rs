//! Drives Lua zone scripts.
//!
//! Zone scripts registered with `RegisterZoneScript(zone_id, table)` are invoked
//! from three places:
//!
//! - `on_player_enter_zone` / `on_player_leave_zone`, when a player crosses a
//!   zone boundary (or logs in / out).
//! - `update_zone_scripts`, a periodic tick for zones that have live state.
//!
//! Player membership is tracked in the per-zone script state so `Update` can
//! report a player count and scripts can drive world events off it.
//!
//! Tracks player membership per zone so `Update` can report a player count.

use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;
use std::time::Duration;

use super::gossip_executor::execute_gossip_actions;
use crate::World;

/// Called when a player enters `zone_id`.
///
/// Registers the player with the zone's state and fires `OnPlayerEnter`.
pub async fn on_player_enter_zone(
    player_guid: ObjectGuid,
    zone_id: u32,
    world: &World,
) -> Result<()> {
    if zone_id == 0 {
        return Ok(());
    }

    let lua_mgr = &world.managers.lua_mgr;
    let Some(script) = lua_mgr.get_zone_script(zone_id) else {
        return Ok(());
    };

    // Track membership before the callback so the snapshot's player count
    // includes the player who just arrived.
    lua_mgr.with_zone_state_mut(zone_id, |state| state.add_player(player_guid));

    let snapshot = zone_snapshot(zone_id, world, 0);
    let actions = lua_mgr.with_lua(|lua| script.on_player_enter(lua, &snapshot, player_guid));

    if !actions.is_empty() {
        execute_gossip_actions(actions, player_guid, ObjectGuid::empty(), world).await?;
    }

    Ok(())
}

/// Called when a player leaves `zone_id`.
///
/// Fires `OnPlayerLeave`, then deregisters the player.
pub async fn on_player_leave_zone(
    player_guid: ObjectGuid,
    zone_id: u32,
    world: &World,
) -> Result<()> {
    if zone_id == 0 {
        return Ok(());
    }

    let lua_mgr = &world.managers.lua_mgr;
    let Some(script) = lua_mgr.get_zone_script(zone_id) else {
        // Still drop membership: a script may have been unregistered by a reload
        // while the player was standing in the zone.
        lua_mgr.with_zone_state_mut(zone_id, |state| state.remove_player(player_guid));
        return Ok(());
    };

    let snapshot = zone_snapshot(zone_id, world, 0);
    let actions = lua_mgr.with_lua(|lua| script.on_player_leave(lua, &snapshot, player_guid));

    // Remove after the callback so the script can still see the leaving player.
    lua_mgr.with_zone_state_mut(zone_id, |state| state.remove_player(player_guid));

    if !actions.is_empty() {
        execute_gossip_actions(actions, player_guid, ObjectGuid::empty(), world).await?;
    }

    Ok(())
}

/// Move a player from one zone to the other, firing both callbacks.
///
/// `old_zone` of 0 means the player had no previous zone (fresh login).
pub async fn on_player_zone_change(
    player_guid: ObjectGuid,
    old_zone: u32,
    new_zone: u32,
    world: &World,
) -> Result<()> {
    if old_zone == new_zone {
        return Ok(());
    }

    on_player_leave_zone(player_guid, old_zone, world).await?;
    on_player_enter_zone(player_guid, new_zone, world).await?;

    Ok(())
}

/// Tick every zone that has live script state.
///
/// Advances the zone's script timers and fires `Update`. Only zones already
/// carrying state are visited, so a server with no zone scripts does no work.
pub async fn update_zone_scripts(diff: Duration, world: &World) -> Result<()> {
    let lua_mgr = &world.managers.lua_mgr;
    let diff_ms = diff.as_millis() as u32;

    for zone_id in lua_mgr.active_zone_states() {
        let Some(script) = lua_mgr.get_zone_script(zone_id) else {
            continue;
        };

        lua_mgr.with_zone_state_mut(zone_id, |state| state.update_timers(diff_ms));

        let snapshot = zone_snapshot(zone_id, world, diff_ms);
        let actions = lua_mgr.with_lua(|lua| script.on_update(lua, &snapshot));

        if !actions.is_empty() {
            // Zone-wide actions are not tied to one player; address them at the
            // first tracked player so player-scoped actions still have a target.
            let target = snapshot_first_player(zone_id, world);
            execute_gossip_actions(actions, target, ObjectGuid::empty(), world).await?;
        }
    }

    Ok(())
}

/// Build the snapshot handed to a zone callback.
fn zone_snapshot(zone_id: u32, world: &World, diff_ms: u32) -> super::snapshot::ZoneSnapshot {
    let state = world.managers.lua_mgr.get_zone_state(zone_id);

    // Resolve the map from a player in the zone; zone ids do not carry one.
    let map_id = state
        .players
        .first()
        .and_then(|guid| world.managers.player_mgr.with_player(*guid, |p| p.map_id))
        .unwrap_or(0);

    state.to_snapshot(map_id, zone_id, zone_id, diff_ms)
}

/// First player tracked in a zone, or an empty GUID when the zone is empty.
fn snapshot_first_player(zone_id: u32, world: &World) -> ObjectGuid {
    world
        .managers
        .lua_mgr
        .get_zone_state(zone_id)
        .players
        .first()
        .copied()
        .unwrap_or_else(ObjectGuid::empty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::HighGuid;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });

        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn guid(counter: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, counter)
    }

    /// Register a zone script that records each callback into a global log table.
    fn install_recording_script(world: &World, zone_id: u32) {
        world.managers.lua_mgr.with_lua(|lua| {
            lua.load(
                r#"
                _G.calls = {}
                _G.zone_script = {
                    OnPlayerEnter = function(self, zone, player)
                        table.insert(_G.calls, "enter:" .. zone.zone_id .. ":" .. zone.player_count)
                    end,
                    OnPlayerLeave = function(self, zone, player)
                        table.insert(_G.calls, "leave:" .. zone.zone_id .. ":" .. zone.player_count)
                    end,
                    Update = function(self, zone)
                        table.insert(_G.calls, "update:" .. zone.zone_id .. ":" .. zone.diff_ms)
                    end,
                }
                "#,
            )
            .exec()
            .expect("script should load");

            let table: mlua::Table = lua.globals().get("zone_script").expect("table exists");
            world
                .managers
                .lua_mgr
                .register_zone_script_table(zone_id, table);
        });
    }

    fn recorded_calls(world: &World) -> Vec<String> {
        world.managers.lua_mgr.with_lua(|lua| {
            let calls: mlua::Table = lua.globals().get("calls").expect("calls table exists");
            calls
                .sequence_values::<String>()
                .map(|v| v.expect("string entry"))
                .collect()
        })
    }

    #[tokio::test]
    async fn enter_and_leave_invoke_the_registered_callbacks() {
        let world = test_world();
        install_recording_script(&world, 1519);

        on_player_enter_zone(guid(1), 1519, &world)
            .await
            .expect("enter should succeed");
        on_player_leave_zone(guid(1), 1519, &world)
            .await
            .expect("leave should succeed");

        // The entering player is counted before OnPlayerEnter runs, and the
        // leaving player is still counted during OnPlayerLeave.
        assert_eq!(
            recorded_calls(&world),
            vec!["enter:1519:1".to_string(), "leave:1519:1".to_string()]
        );
    }

    #[tokio::test]
    async fn player_count_tracks_membership() {
        let world = test_world();
        install_recording_script(&world, 1519);

        on_player_enter_zone(guid(1), 1519, &world).await.unwrap();
        on_player_enter_zone(guid(2), 1519, &world).await.unwrap();

        assert_eq!(
            world.managers.lua_mgr.get_zone_state(1519).player_count(),
            2
        );

        on_player_leave_zone(guid(1), 1519, &world).await.unwrap();
        assert_eq!(
            world.managers.lua_mgr.get_zone_state(1519).player_count(),
            1
        );
    }

    #[tokio::test]
    async fn zone_change_fires_leave_then_enter() {
        let world = test_world();
        install_recording_script(&world, 1519);
        install_recording_script(&world, 1537);

        on_player_enter_zone(guid(1), 1519, &world).await.unwrap();
        on_player_zone_change(guid(1), 1519, 1537, &world)
            .await
            .unwrap();

        assert_eq!(
            recorded_calls(&world),
            vec![
                "enter:1519:1".to_string(),
                "leave:1519:1".to_string(),
                "enter:1537:1".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn zone_change_to_the_same_zone_is_a_no_op() {
        let world = test_world();
        install_recording_script(&world, 1519);

        on_player_zone_change(guid(1), 1519, 1519, &world)
            .await
            .unwrap();

        assert!(recorded_calls(&world).is_empty());
    }

    #[tokio::test]
    async fn update_ticks_zones_with_live_state() {
        let world = test_world();
        install_recording_script(&world, 1519);

        // No state yet, so nothing is ticked.
        update_zone_scripts(Duration::from_millis(50), &world)
            .await
            .unwrap();
        assert!(recorded_calls(&world).is_empty());

        on_player_enter_zone(guid(1), 1519, &world).await.unwrap();
        update_zone_scripts(Duration::from_millis(50), &world)
            .await
            .unwrap();

        assert_eq!(
            recorded_calls(&world),
            vec!["enter:1519:1".to_string(), "update:1519:50".to_string()]
        );
    }

    #[tokio::test]
    async fn unscripted_zone_is_ignored() {
        let world = test_world();
        install_recording_script(&world, 1519);

        on_player_enter_zone(guid(1), 4444, &world).await.unwrap();
        on_player_leave_zone(guid(1), 4444, &world).await.unwrap();

        assert!(recorded_calls(&world).is_empty());
        assert_eq!(
            world.managers.lua_mgr.get_zone_state(4444).player_count(),
            0
        );
    }

    #[tokio::test]
    async fn zone_id_zero_is_ignored() {
        let world = test_world();
        install_recording_script(&world, 0);

        on_player_enter_zone(guid(1), 0, &world).await.unwrap();
        on_player_leave_zone(guid(1), 0, &world).await.unwrap();

        assert!(recorded_calls(&world).is_empty());
    }

    #[tokio::test]
    async fn zone_timers_count_down_on_update() {
        let world = test_world();
        install_recording_script(&world, 1519);
        on_player_enter_zone(guid(1), 1519, &world).await.unwrap();

        world
            .managers
            .lua_mgr
            .with_zone_state_mut(1519, |state| state.set_timer(1, 100));
        assert!(!world
            .managers
            .lua_mgr
            .get_zone_state(1519)
            .is_timer_ready(1));

        update_zone_scripts(Duration::from_millis(150), &world)
            .await
            .unwrap();

        assert!(world
            .managers
            .lua_mgr
            .get_zone_state(1519)
            .is_timer_ready(1));
    }

    #[tokio::test]
    async fn zone_data_round_trips_through_the_manager() {
        let world = test_world();

        assert_eq!(world.managers.lua_mgr.get_zone_data(1519, 1), 0);
        world.managers.lua_mgr.set_zone_data(1519, 1, 5);
        assert_eq!(world.managers.lua_mgr.get_zone_data(1519, 1), 5);
        assert!(world.managers.lua_mgr.active_zone_states().contains(&1519));
    }
}
