//! Running a dungeon copy's encounter script.
//!
//! Each scripted copy gets its own script state, restored from the copy's save
//! when the copy loads and written back whenever the script changes its data,
//! so doors, boss states and progress survive leaving, unloading and restarts.
//! Callbacks fire exactly once per event: on load, when a creature or object
//! is created in the copy, when a creature dies, when a player enters or
//! leaves, and on a periodic update.

use crate::core::lua::{CreatureScriptState, LuaAction};
use crate::game::common::update_fields::GAMEOBJECT_STATE;
use crate::game::gameobject::GOState;
use crate::World;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::protocol::ObjectGuid;

fn script_for(
    world: &World,
    map_id: u32,
    instance_id: u32,
) -> Option<crate::core::lua::scripts::LuaInstanceAI> {
    if instance_id == 0 {
        return None;
    }
    world.managers.lua_mgr.get_instance_ai(map_id)
}

fn snapshot(
    world: &World,
    map_id: u32,
    instance_id: u32,
) -> crate::core::lua::snapshot::InstanceSnapshot {
    let mut snapshot = world
        .managers
        .lua_mgr
        .get_instance_state(map_id, instance_id)
        .to_snapshot(map_id, instance_id);
    snapshot.player_count = world
        .managers
        .map_mgr
        .get_map(map_id, instance_id)
        .map(|map| map.player_count() as u32)
        .unwrap_or(0);
    snapshot
}

/// Create a copy's script state (restored from its save) and run `OnLoad`, once.
/// Safe to call repeatedly; only the first call for a copy does anything.
pub async fn ensure_loaded(world: &World, map_id: u32, instance_id: u32) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    let lua_mgr = &world.managers.lua_mgr;
    if lua_mgr.has_instance_state(map_id, instance_id) {
        return;
    }
    let mut state = crate::core::lua::scripts::InstanceScriptState::new();
    state.data = world
        .managers
        .instance_mgr
        .saved_script_data(map_id, instance_id);
    lua_mgr.set_instance_state(map_id, instance_id, state);

    let snapshot = snapshot(world, map_id, instance_id);
    let actions = lua_mgr.with_lua(|lua| script.on_load(lua, &snapshot));
    apply(world, map_id, instance_id, None, actions).await;
}

/// A creature was created in a scripted copy.
pub async fn on_creature_create(
    world: &World,
    map_id: u32,
    instance_id: u32,
    entry: u32,
    guid: ObjectGuid,
) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.on_creature_create(lua, &snapshot, entry, guid));
    apply(world, map_id, instance_id, Some(guid), actions).await;
}

/// A gameobject was created in a scripted copy.
pub async fn on_gameobject_create(
    world: &World,
    map_id: u32,
    instance_id: u32,
    entry: u32,
    guid: ObjectGuid,
) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.on_gameobject_create(lua, &snapshot, entry, guid));
    apply(world, map_id, instance_id, None, actions).await;
}

/// A creature died in a scripted copy.
pub async fn on_creature_death(
    world: &World,
    map_id: u32,
    instance_id: u32,
    entry: u32,
    guid: ObjectGuid,
) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.on_creature_death(lua, &snapshot, entry, guid));
    apply(world, map_id, instance_id, Some(guid), actions).await;
}

/// A player arrived in a scripted copy.
pub async fn on_player_enter(world: &World, map_id: u32, instance_id: u32, player: ObjectGuid) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    ensure_loaded(world, map_id, instance_id).await;
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.on_player_enter(lua, &snapshot, player));
    apply(world, map_id, instance_id, None, actions).await;
}

/// A player left a scripted copy.
pub async fn on_player_leave(world: &World, map_id: u32, instance_id: u32, player: ObjectGuid) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    if !world
        .managers
        .lua_mgr
        .has_instance_state(map_id, instance_id)
    {
        return;
    }
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.on_player_leave(lua, &snapshot, player));
    apply(world, map_id, instance_id, None, actions).await;
}

/// Periodic update of an occupied scripted copy.
pub async fn update(world: &World, map_id: u32, instance_id: u32, diff_ms: u32) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    let lua_mgr = &world.managers.lua_mgr;
    if !lua_mgr.has_instance_state(map_id, instance_id) {
        return;
    }
    let mut state = lua_mgr.get_instance_state(map_id, instance_id);
    state.update_timers(diff_ms);
    lua_mgr.set_instance_state(map_id, instance_id, state);
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = lua_mgr.with_lua(|lua| script.on_update(lua, &snapshot));
    apply(world, map_id, instance_id, None, actions).await;
}

/// Carry out a script's actions. Instance-level actions (data, stored guids,
/// timers, doors) apply to the copy; anything else needs a creature to act
/// through and is run by that creature when the callback has one.
async fn apply(
    world: &World,
    map_id: u32,
    instance_id: u32,
    creature: Option<ObjectGuid>,
    actions: Vec<LuaAction>,
) {
    if actions.is_empty() {
        return;
    }
    let lua_mgr = &world.managers.lua_mgr;
    let mut data_changed = false;
    let mut creature_actions = Vec::new();

    for action in actions {
        match action {
            LuaAction::SetInstanceData { data_id, value } => {
                lua_mgr.set_instance_data(map_id, instance_id, data_id, value);
                data_changed = true;
            }
            LuaAction::SetInstanceGuid { data_id, guid } => {
                let mut state = lua_mgr.get_instance_state(map_id, instance_id);
                state.set_guid(data_id, guid);
                lua_mgr.set_instance_state(map_id, instance_id, state);
            }
            LuaAction::SetTimer {
                timer_id,
                duration_ms,
            } => {
                let mut state = lua_mgr.get_instance_state(map_id, instance_id);
                state.set_timer(timer_id, duration_ms);
                lua_mgr.set_instance_state(map_id, instance_id, state);
            }
            LuaAction::OpenDoor { guid } => set_door(world, guid, true),
            LuaAction::CloseDoor { guid } => set_door(world, guid, false),
            LuaAction::OpenDoorByData { data_id } => {
                if let Some(guid) = lua_mgr.get_instance_state(map_id, instance_id).get_guid(data_id) {
                    set_door(world, guid, true);
                }
            }
            LuaAction::CloseDoorByData { data_id } => {
                if let Some(guid) = lua_mgr.get_instance_state(map_id, instance_id).get_guid(data_id) {
                    set_door(world, guid, false);
                }
            }
            other => creature_actions.push(other),
        }
    }

    if !creature_actions.is_empty() {
        match creature {
            Some(creature) => {
                let mut state = CreatureScriptState::default();
                let ai_actions =
                    crate::core::lua::bridge::lua_actions_to_ai_actions(creature_actions, &mut state);
                crate::game::creature::ai::executor::execute_actions(world, creature, ai_actions);
            }
            None => tracing::debug!(
                "Instance script {}:{} returned {} actions with no creature to run them",
                map_id,
                instance_id,
                creature_actions.len()
            ),
        }
    }

    if data_changed {
        let data = lua_mgr.get_instance_state(map_id, instance_id).data;
        if let Err(e) = world
            .managers
            .instance_mgr
            .save_script_data(&world.databases, map_id, instance_id, data)
            .await
        {
            tracing::warn!(
                "Failed to save script data of {}:{}: {}",
                map_id,
                instance_id,
                e
            );
        }
    }
}

/// Save a copy's script data without waiting, for callers inside synchronous code
/// (creature AI setting instance data).
pub fn queue_save(world: &World, map_id: u32, instance_id: u32) {
    if instance_id == 0 {
        return;
    }
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let world = world.clone();
    let data = world
        .managers
        .lua_mgr
        .get_instance_state(map_id, instance_id)
        .data;
    handle.spawn(async move {
        if let Err(e) = world
            .managers
            .instance_mgr
            .save_script_data(&world.databases, map_id, instance_id, data)
            .await
        {
            tracing::warn!(
                "Failed to save script data of {}:{}: {}",
                map_id,
                instance_id,
                e
            );
        }
    });
}

/// Open or close a door: change its state, its collision in its own copy, and
/// what nearby clients see.
pub fn set_door(world: &World, guid: ObjectGuid, open: bool) {
    let state = if open { GOState::Active } else { GOState::Ready };
    let Some((map_id, instance_id, position)) = world
        .managers
        .gameobject_mgr
        .with_gameobject_mut(guid, |go| {
            go.go_state = state;
            (go.map_id, go.instance_id, go.position)
        })
    else {
        return;
    };
    world
        .managers
        .gameobject_mgr
        .update_collision_state(guid, &world.managers.vmap_mgr);

    let Some(map) = world.managers.map_mgr.get_map(map_id, instance_id) else {
        return;
    };
    let viewers = map.get_players_in_range(position, map.visibility_distance());
    for viewer in viewers {
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(guid, ObjectType::GameObject)
                .set_field(GAMEOBJECT_STATE, state as u32),
        ));
        world.managers.broadcast_mgr.send_msg_to_player(viewer, update);
    }
}
