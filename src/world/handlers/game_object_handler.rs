//! Game object interaction handler
//!
//! Handles CMSG_GAMEOBJ_USE (0x00B1) — player right-clicks / uses a game object.
//! Routes to Lua OnGameObjectHello script if registered for the GO's entry.

use anyhow::Result;
use tracing::debug;

use crate::shared::protocol::WorldPacket;
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle CMSG_GAMEOBJ_USE (0x00B1)
///
/// Sent when the player right-clicks / activates a game object.
/// Packet format: GUID (8 bytes, unpacked)
pub async fn handle_gameobj_use(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let go_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read GO GUID"))?;

    debug!(
        "CMSG_GAMEOBJ_USE: player={:?}, go={:?}",
        player_guid, go_guid
    );

    // Look up the GO's template entry from its GUID
    let go_entry = world
        .managers
        .gameobject_mgr
        .get_gameobject(go_guid)
        .map(|go| go.entry)
        .unwrap_or(0);

    if go_entry == 0 {
        debug!(
            "CMSG_GAMEOBJ_USE: GO {:?} not found or has no entry",
            go_guid
        );
        return Ok(());
    }

    let quest_items =
        world
            .systems
            .quest
            .prepare_gameobject_quest_menu(player_guid, go_entry, world);
    if !quest_items.is_empty() {
        world
            .systems
            .quest
            .send_gameobject_quest_list(player_guid, go_guid, go_entry, world)?;
        return Ok(());
    }

    // Check for a Lua script handling OnGameObjectHello
    if let Some(script) = world.managers.lua_mgr.get_game_object_script(go_entry) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_gameobject_hello(lua, &player_snap, go_guid));
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, go_guid, world).await?;
        }
    }

    Ok(())
}

/// Handle game object open/loot interaction.
///
/// Called internally when a player opens a chest or door (not directly from a packet).
/// Routes to Lua OnGameObjectOpen script if registered.
pub async fn handle_gameobj_open(
    player_guid: crate::shared::protocol::ObjectGuid,
    go_guid: crate::shared::protocol::ObjectGuid,
    world: &World,
) -> Result<()> {
    let go_entry = world
        .managers
        .gameobject_mgr
        .get_gameobject(go_guid)
        .map(|go| go.entry)
        .unwrap_or(0);

    if go_entry == 0 {
        return Ok(());
    }

    if let Some(script) = world.managers.lua_mgr.get_game_object_script(go_entry) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_gameobject_open(lua, &player_snap, go_guid));
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, go_guid, world).await?;
        }
    }

    Ok(())
}
