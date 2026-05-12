//! Movement handlers

use anyhow::{anyhow, Result};
use tracing::{debug, info, trace};

use crate::shared::messages::character::SmsgLogoutCancelAck;
use crate::shared::messages::movement::SmsgForceMoveUnroot;
use crate::shared::messages::social::SmsgStandstateUpdate;
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::core::common::MovementInfo;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle MSG_MOVE_WORLDPORT_ACK - acknowledge far teleport
///
/// This is sent by the client after receiving SMSG_NEW_WORLD to confirm the map has loaded.
/// This handler completes the two-step teleport process:
/// 1. Area trigger handler sends TRANSFER_PENDING + NEW_WORLD (initiates teleport)
/// 2. This handler receives ACK and completes the teleport (map updates, initialization packets)
pub async fn handle_worldport_ack(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    info!("========================================");
    info!("[WORLDPORT-ACK] ★ MSG_MOVE_WORLDPORT_ACK RECEIVED ★");
    info!("========================================");

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    info!("[WORLDPORT-ACK] Player GUID: {:?}", player_guid);

    // Get pending teleport destination
    info!("[WORLDPORT-ACK] Retrieving pending teleport from session...");
    let pending = session.get_pending_teleport();
    info!("[WORLDPORT-ACK] Pending teleport: {:?}", pending);

    let (dest_map, dest_instance_id, dest_pos) = pending
        .ok_or_else(|| anyhow!("No pending teleport"))?;

    info!(
        "[WORLDPORT-ACK] Destination: map={} instance={} pos=({},{},{},{})",
        dest_map, dest_instance_id, dest_pos.x, dest_pos.y, dest_pos.z, dest_pos.o
    );

    session.clear_pending_teleport();
    info!("[WORLDPORT-ACK] Cleared pending teleport from session");

    // Get old map info
    info!("[WORLDPORT-ACK] Getting current player location...");
    let (old_map_id, old_instance_id) = world
        .managers
        .player_mgr
        .with_player(player_guid, |p| (p.map_id, p.instance_id))
        .ok_or_else(|| anyhow!("Player not found"))?;

    info!(
        "[WORLDPORT-ACK] Current location: map={} instance={}",
        old_map_id, old_instance_id
    );

    // Remove from old map
    info!("[WORLDPORT-ACK] Removing player from old map {} instance {}...", old_map_id, old_instance_id);
    let old_map = world
        .managers
        .map_mgr
        .get_or_create_map(old_map_id, old_instance_id);
    old_map.remove_player(player_guid);
    info!("[WORLDPORT-ACK] ✓ Player removed from old map");

    // Update player state
    info!("[WORLDPORT-ACK] Updating player state (map, instance, position)...");
    world.managers.player_mgr.with_player_mut(player_guid, |player| {
        player.map_id = dest_map;
        player.instance_id = dest_instance_id;
        player.movement.position = dest_pos;
    });
    info!("[WORLDPORT-ACK] ✓ Player state updated");

    // Add to new map
    info!("[WORLDPORT-ACK] Adding player to new map {} instance {}...", dest_map, dest_instance_id);
    let new_map = world
        .managers
        .map_mgr
        .get_or_create_map(dest_map, dest_instance_id);
    new_map.add_player(player_guid, dest_pos);
    info!("[WORLDPORT-ACK] ✓ Player added to new map");

    // Send initialization packets (critical for client to exit loading screen)
    use crate::shared::messages::login::{SmsgBindPointUpdate, SmsgInitWorldStates, SmsgSetRestStart};

    info!("[WORLDPORT-ACK] Sending initialization packets...");

    // 1. SMSG_SET_REST_START
    info!("[WORLDPORT-ACK] Sending SMSG_SET_REST_START...");
    session.send_msg(SmsgSetRestStart { time: 0 })?;
    info!("[WORLDPORT-ACK] ✓ SMSG_SET_REST_START sent");

    // 2. SMSG_BINDPOINTUPDATE (use player's homebind)
    let (bind_x, bind_y, bind_z, bind_map, bind_zone) = world
        .managers
        .player_mgr
        .with_player(player_guid, |p| {
            (p.homebind_x, p.homebind_y, p.homebind_z, p.homebind_map, p.homebind_zone)
        })
        .unwrap_or((dest_pos.x, dest_pos.y, dest_pos.z, dest_map, 0));

    info!(
        "[WORLDPORT-ACK] Sending SMSG_BINDPOINTUPDATE (map={} zone={})...",
        bind_map, bind_zone
    );
    session.send_msg(SmsgBindPointUpdate {
        x: bind_x,
        y: bind_y,
        z: bind_z,
        map_id: bind_map,
        zone_id: bind_zone,
    })?;
    info!("[WORLDPORT-ACK] ✓ SMSG_BINDPOINTUPDATE sent");

    // 3. SMSG_UPDATE_OBJECT - send item CREATE blocks then player's own object data.
    // Item blocks must come first (matching initial login order) so the client
    // knows about each item object before the player fields reference their GUIDs.
    info!("[WORLDPORT-ACK] Building player CREATE_OBJECT2 block...");

    use crate::shared::messages::update::{SmsgUpdateObject, UpdateBlockData};
    use crate::world::handlers::character::build_player_create_block_for_player;

    let player_block = {
        let player_ref = world
            .managers
            .player_mgr
            .get_player(player_guid)
            .ok_or_else(|| anyhow!("Player not found"))?;

        build_player_create_block_for_player(&player_ref, world)?
        // player_ref dropped here, releasing read lock
    };

    let mut item_blocks = Vec::new();
    world.systems.inventory.build_item_create_blocks(player_guid, &mut item_blocks);

    let mut update_object = SmsgUpdateObject::new();
    for block in item_blocks {
        update_object = update_object.add_block(UpdateBlockData::CreateObject2(block));
    }
    update_object = update_object.add_block(UpdateBlockData::CreateObject2(player_block));

    info!("[WORLDPORT-ACK] Sending SMSG_UPDATE_OBJECT with item + player CREATE_OBJECT2...");
    session.send_msg(update_object)?;
    info!("[WORLDPORT-ACK] ✓ SMSG_UPDATE_OBJECT sent - player should spawn now");

    // 4. SMSG_INIT_WORLD_STATES (critical - client needs this to exit loading)
    let zone = world
        .managers
        .player_mgr
        .with_player(player_guid, |p| p.zone_id)
        .unwrap_or(0);

    info!(
        "[WORLDPORT-ACK] Sending SMSG_INIT_WORLD_STATES (map={} zone={}) - CRITICAL PACKET...",
        dest_map, zone
    );
    session.send_msg(SmsgInitWorldStates::new(dest_map, zone))?;
    info!("[WORLDPORT-ACK] ✓ SMSG_INIT_WORLD_STATES sent - client should exit loading screen now");

    // Force immediate visibility update to send nearby creatures/objects
    info!("[WORLDPORT-ACK] Starting visibility update...");
    let current_tick = world.managers.map_mgr.current_tick();
    info!("[WORLDPORT-ACK] Current tick: {}", current_tick);

    // Reset visibility state so the new map's objects are treated as fresh.
    // visible_objects holds the old map's set — if we don't clear it, the delta
    // calculation sees all new-map creatures as "already known" and sends nothing.
    // objects_created holds the dedup guard — stale entries would block CREATE_OBJECT2.
    world.managers.player_mgr.with_player_mut(player_guid, |player| {
        player.visibility.visible_objects.clear();
        player.visibility.objects_created.clear();
        player.visibility.pending_appeared.clear();
        player.visibility.pending_disappeared.clear();
    });
    info!("[WORLDPORT-ACK] ✓ Cleared stale visibility state for new map");

    info!("[WORLDPORT-ACK] Marking player for force immediate visibility update...");
    world
        .systems
        .player
        .visibility()
        .mark_force_immediate(player_guid, world);
    info!("[WORLDPORT-ACK] ✓ Marked for immediate update");

    info!("[WORLDPORT-ACK] Calling update_player...");
    let updated = world
        .systems
        .player
        .visibility()
        .update_player(player_guid, current_tick, world)?;
    info!("[WORLDPORT-ACK] update_player returned: {}", updated);

    if updated {
        info!("[WORLDPORT-ACK] Flushing visibility notifications (sending CREATE_OBJECT2 packets)...");
        world
            .systems
            .player
            .visibility()
            .flush_pending_notifications(player_guid, world)
            .await?;
        info!("[WORLDPORT-ACK] ✓ Visibility packets sent successfully");
    } else {
        info!("[WORLDPORT-ACK] ⚠ No visibility update (grids may not be loaded yet)");
    }

    info!("========================================");
    info!(
        "[WORLDPORT-ACK] ★ TELEPORT COMPLETE ★ map {} instance {}",
        dest_map, dest_instance_id
    );
    info!("========================================");

    Ok(())
}

/// Handle all MSG_MOVE_* packets
pub async fn handle_movement(
    session: &WorldSession,
    opcode: Opcode,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    // Check if player is rooted (logging out)
    const UNIT_FLAG_DISABLE_MOVE: u32 = 0x00000004;
    const STAND_STATE_STAND: u8 = 0;

    let (is_rooted, unit_flags) = world.managers.player_mgr.with_player(player_guid, |player| {
        let rooted = (player.unit_flags & UNIT_FLAG_DISABLE_MOVE) != 0;
        (rooted, player.unit_flags)
    }).unwrap_or((false, 0));

    trace!(
        "[MOVEMENT] handle_movement for {}: opcode={:?}, is_rooted={}, unit_flags=0x{:08X}",
        player_guid, opcode, is_rooted, unit_flags
    );

    if is_rooted {
        // Movement during logout - auto-cancel logout
        info!(
            "[LOGOUT_TIMER] Movement detected during logout, cancelling logout for session {} (player: {})",
            session.id(),
            player_guid
        );

        // Cancel the logout
        session.cancel_logout_timer();
        session.send_msg(SmsgLogoutCancelAck)?;

        // Unroot and stand up
        let world_guid = WorldObjectGuid::from_low(player_guid.counter());
        session.send_msg(SmsgForceMoveUnroot { guid: world_guid })?;

        world.managers.player_mgr.with_player_mut(player_guid, |player| {
            player.stand_state = STAND_STATE_STAND;
            player.unit_flags &= !UNIT_FLAG_DISABLE_MOVE;
        });

        session.send_msg(SmsgStandstateUpdate { stand_state: STAND_STATE_STAND })?;
    }

    // Continue with normal movement processing
    let mut movement_info = MovementInfo::read_from_packet(packet)?;
    movement_info.mover_guid = player_guid;

    debug!(
        "[MOVE-IN] opcode={:?} guid={:?} orient={:.4} time={} flags=0x{:08X}",
        opcode, player_guid, movement_info.position.o, movement_info.time, movement_info.flags.value()
    );

    world
        .systems
        .player
        .movement()
        .handle_move(player_guid, opcode, movement_info, world).await

}
