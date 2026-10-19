//! Running a dungeon copy's encounter script.
//!
//! Each scripted copy gets its own script state, restored from the copy's save
//! when the copy loads and written back whenever the script changes its data,
//! so doors, boss states and progress survive leaving, unloading and restarts.
//! Callbacks fire exactly once per event: on load, when a creature or object
//! is created in the copy, when a creature enters combat, evades or dies,
//! when a player enters or leaves, and on a periodic update.

use crate::core::lua::LuaAction;
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

pub(crate) fn snapshot(
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
    snapshot.scope = crate::game::pvp::objective::scope_view(
        world,
        crate::game::pvp::scope::Scope::Copy {
            map_id,
            instance_id,
        },
    );
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
    killer: Option<ObjectGuid>,
    spawn_id: u32,
) {
    let Some(script) = script_for(world, map_id, instance_id) else {
        return;
    };
    let event = world
        .managers
        .bg_events
        .events_of(crate::game::game_event::EventSpawnKind::Creature, spawn_id)
        .first()
        .copied()
        .unwrap_or((255, 255));
    let snapshot = snapshot(world, map_id, instance_id);
    let actions = world
        .managers
        .lua_mgr
        .with_lua(|lua| script.on_creature_death(lua, &snapshot, entry, guid, killer, event));
    apply(world, map_id, instance_id, Some(guid), actions).await;
}

/// A creature in a scripted copy entered combat.
pub async fn on_creature_enter_combat(
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
        .with_lua(|lua| script.on_creature_enter_combat(lua, &snapshot, entry, guid));
    apply(world, map_id, instance_id, Some(guid), actions).await;
}

/// A creature in a scripted copy evaded (left combat and went home).
pub async fn on_creature_evade(
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
        .with_lua(|lua| script.on_creature_evade(lua, &snapshot, entry, guid));
    apply(world, map_id, instance_id, Some(guid), actions).await;
}

/// Combat transitions reported from synchronous creature AI.
#[derive(Debug, Clone, Copy)]
pub enum CreatureCombatEvent {
    EnterCombat,
    Evade,
}

/// Run a creature's combat callback on its copy's script without waiting, for
/// callers inside synchronous code.
pub fn queue_creature_combat_event(world: &World, guid: ObjectGuid, event: CreatureCombatEvent) {
    let Some((map_id, instance_id, entry)) = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| (c.map_id, c.instance_id, c.entry))
    else {
        return;
    };
    if script_for(world, map_id, instance_id).is_none() {
        return;
    }
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let world = world.clone();
    handle.spawn(async move {
        match event {
            CreatureCombatEvent::EnterCombat => {
                on_creature_enter_combat(&world, map_id, instance_id, entry, guid).await
            }
            CreatureCombatEvent::Evade => {
                on_creature_evade(&world, map_id, instance_id, entry, guid).await
            }
        }
    });
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
    let mut snapshot = snapshot(world, map_id, instance_id);
    snapshot.diff_ms = diff_ms;
    let actions = lua_mgr.with_lua(|lua| script.on_update(lua, &snapshot));
    apply(world, map_id, instance_id, None, actions).await;
}

/// Carry out a script's actions. Copy-level actions (data, stored guids,
/// timers, doors, world states, spawn events, announcements) apply to the
/// copy; anything else needs a creature to act through and is run by that
/// creature when the callback has one. Dungeon data is saved with the copy;
/// battleground copies never persist.
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
    let data_changed = actions
        .iter()
        .any(|a| matches!(a, LuaAction::SetInstanceData { .. }));
    crate::game::pvp::objective::apply(
        world,
        crate::game::pvp::scope::Scope::Copy {
            map_id,
            instance_id,
        },
        creature,
        None,
        actions,
    )
    .await;

    let battleground = crate::game::instance::lifecycle::map_entry(world, map_id)
        .is_some_and(|entry| entry.is_battleground());
    if data_changed && !battleground {
        let data = world
            .managers
            .lua_mgr
            .get_instance_state(map_id, instance_id)
            .data;
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
