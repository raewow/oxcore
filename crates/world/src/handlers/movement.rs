//! Movement handlers

use anyhow::{anyhow, Result};
use tracing::{debug, info, trace};

use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::core::common::MovementInfo;
use crate::core::session::WorldSession;
use crate::game::creature::movement::packet_sender::{MovementChangeType, MovementFlagChange};
use crate::World;
use oxcore_shared::messages::character::SmsgLogoutCancelAck;
use oxcore_shared::messages::movement::SmsgForceMoveUnroot;
use oxcore_shared::messages::social::SmsgStandstateUpdate;
use oxcore_shared::protocol::{Opcode, WorldPacket};

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

    let (dest_map, dest_instance_id, dest_pos) =
        pending.ok_or_else(|| anyhow!("No pending teleport"))?;

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
    info!(
        "[WORLDPORT-ACK] Removing player from old map {} instance {}...",
        old_map_id, old_instance_id
    );
    let old_map = world
        .managers
        .map_mgr
        .get_or_create_map(old_map_id, old_instance_id);
    old_map.remove_player(player_guid);
    info!("[WORLDPORT-ACK] ✓ Player removed from old map");

    // Update player state
    info!("[WORLDPORT-ACK] Updating player state (map, instance, position)...");
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.map_id = dest_map;
            player.instance_id = dest_instance_id;
            player.movement.position = dest_pos;
        });
    info!("[WORLDPORT-ACK] ✓ Player state updated");

    // Add to new map
    info!(
        "[WORLDPORT-ACK] Adding player to new map {} instance {}...",
        dest_map, dest_instance_id
    );
    let new_map = world
        .managers
        .map_mgr
        .get_or_create_map(dest_map, dest_instance_id);
    new_map.add_player(player_guid, dest_pos);
    info!("[WORLDPORT-ACK] ✓ Player added to new map");

    // Send initialization packets (critical for client to exit loading screen)
    use oxcore_shared::messages::login::{
        SmsgBindPointUpdate, SmsgInitWorldStates, SmsgSetRestStart,
    };

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
            (
                p.homebind_x,
                p.homebind_y,
                p.homebind_z,
                p.homebind_map,
                p.homebind_zone,
            )
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

    use crate::handlers::character::build_player_create_block_for_player;
    use oxcore_shared::messages::update::{SmsgUpdateObject, UpdateBlockData};

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
    world
        .systems
        .inventory
        .build_item_create_blocks(player_guid, &mut item_blocks);

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
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
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
    let updated =
        world
            .systems
            .player
            .visibility()
            .update_player(player_guid, current_tick, world)?;
    info!("[WORLDPORT-ACK] update_player returned: {}", updated);

    if updated {
        info!(
            "[WORLDPORT-ACK] Flushing visibility notifications (sending CREATE_OBJECT2 packets)..."
        );
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

/// Validate incoming movement info before applying it (`WorldSession::VerifyMovementInfo`).
///
/// Rejects positions that aren't finite/in-bounds, and for on-transport movement
/// rejects oversized transport offsets or offsets that produce an invalid absolute
/// world coordinate.
pub fn verify_movement_info(movement_info: &MovementInfo) -> bool {
    use crate::core::common::MoveFlags;

    if !is_valid_map_coord(
        movement_info.position.x,
        movement_info.position.y,
        movement_info.position.z,
        movement_info.position.o,
    ) {
        return false;
    }

    if movement_info.flags.has_flag(MoveFlags::ONTRANSPORT) {
        let t = movement_info.transport_position.unwrap_or_default();

        // Transports are size-limited. Also received at zeppelin/lift leave with
        // absolute continent coords, which can be safely skipped.
        if t.x.abs() > 250.0 || t.y.abs() > 250.0 || t.z.abs() > 100.0 {
            return false;
        }

        if !is_valid_map_coord(
            movement_info.position.x + t.x,
            movement_info.position.y + t.y,
            movement_info.position.z + t.z,
            movement_info.position.o + t.o,
        ) {
            return false;
        }
    }

    true
}

/// Equivalent of `MaNGOS::IsValidMapCoord` for a single coordinate set.
///
/// Coordinates must be finite and within the map coordinate bound (MAX_MAP_COORD
/// is ~64*533.33 ≈ 17066.666); orientation must be finite.
fn is_valid_map_coord(x: f32, y: f32, z: f32, o: f32) -> bool {
    const MAX_MAP_COORD: f32 = 64.0 * 533.3333_f32;
    x.is_finite()
        && y.is_finite()
        && z.is_finite()
        && o.is_finite()
        && x.abs() <= MAX_MAP_COORD
        && y.abs() <= MAX_MAP_COORD
        && z.abs() <= MAX_MAP_COORD
}

/// Handle CMSG_SET_ACTIVE_MOVER (`WorldSession::HandleSetActiveMoverOpcode`).
///
/// Records which unit the client believes it is controlling. Without pet
/// possession / mind control, the only legal mover is the player itself; a
/// mismatch is logged and the client is corrected back to the player GUID.
pub fn handle_set_active_mover(
    session: &WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let guid_raw = packet
        .read_guid_raw()
        .ok_or_else(|| anyhow!("SetActiveMover: missing guid"))?;

    // Empty guid: client is releasing the active mover.
    if guid_raw == 0 {
        session.set_client_mover_guid(None);
        return Ok(());
    }

    let requested = WorldObjectGuid::from_low((guid_raw & 0xFFFF_FFFF) as u32);

    if requested.counter() != player_guid.counter() {
        debug!(
            "[SET_ACTIVE_MOVER] Incorrect mover guid {:?}; correcting to player {:?}",
            requested, player_guid
        );
        // Correct the client's view to the player (mover stays the player).
        session.set_client_mover_guid(Some(player_guid));
        return Ok(());
    }

    session.set_client_mover_guid(Some(requested));
    Ok(())
}

/// Handle CMSG_MOVE_NOT_ACTIVE_MOVER (`WorldSession::HandleMoveNotActiveMoverOpcode`).
///
/// The client gives up control of the active mover. We clear the active-mover
/// GUID, validate the trailing movement info, apply the relocation and rebroadcast
/// a heartbeat/stop to observers.
pub async fn handle_move_not_active_mover(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let old_mover_raw = packet
        .read_packed_guid_raw()
        .or_else(|| packet.read_guid_raw())
        .ok_or_else(|| anyhow!("NotActiveMover: missing guid"))?;
    let _ = old_mover_raw;

    session.set_client_mover_guid(None);

    let mut movement_info = MovementInfo::read_from_packet(packet)?;
    movement_info.mover_guid = player_guid;

    // Do not accept packets sent before the reject window or that fail validation.
    if movement_info.time <= session.move_reject_time() {
        return Ok(());
    }
    if !verify_movement_info(&movement_info) {
        return Ok(());
    }

    // Choose the rebroadcast opcode based on whether the player is still moving.
    let opcode = rebroadcast_opcode(&movement_info);
    world
        .systems
        .player
        .movement()
        .handle_move(player_guid, opcode, movement_info, world)
        .await
}

/// Handle the logout flow's CMSG_FORCE_MOVE_ROOT_ACK.
///
/// Generic forced-root changes still require the movement-controller queue. The
/// existing logout path sends a counter-zero root packet, which can be tracked
/// safely on the session and acknowledged independently.
pub fn handle_move_root_ack(
    session: &WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;
    let guid_raw = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("MoveRootAck: missing guid"))?;
    let counter = packet
        .read_u32()
        .ok_or_else(|| anyhow!("MoveRootAck: missing counter"))?;

    let mover = WorldObjectGuid::from_low((guid_raw & 0xFFFF_FFFF) as u32);
    if counter != 0 || session.get_mover_from_guid(mover) != Some(player_guid) {
        return Ok(());
    }

    session.take_pending_root_ack();
    Ok(())
}

/// Handle CMSG_MOVE_KNOCK_BACK_ACK for a server-initiated player knockback.
pub async fn handle_move_knock_back_ack(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;
    let guid_raw = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("KnockBackAck: missing guid"))?;
    let counter = packet
        .read_u32()
        .ok_or_else(|| anyhow!("KnockBackAck: missing counter"))?;
    let mut movement_info = MovementInfo::read_from_packet(packet)?;

    let mover = WorldObjectGuid::from_low((guid_raw & 0xFFFF_FFFF) as u32);
    if session.get_mover_from_guid(mover) != Some(player_guid) {
        return Ok(());
    }

    let pending = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| match player.movement.pending_knockback {
            Some(pending) if pending.counter == counter => {
                player.movement.pending_knockback = None;
                Some(pending)
            }
            _ => None,
        })
        .flatten();
    let Some(pending) = pending else {
        return Ok(());
    };

    if !verify_movement_info(&movement_info) {
        return Ok(());
    }

    movement_info.mover_guid = player_guid;
    movement_info.jump_cos_angle = Some(pending.cos_angle);
    movement_info.jump_sin_angle = Some(pending.sin_angle);
    movement_info.jump_xy_speed = Some(pending.horizontal_speed);
    movement_info.jump_velocity = Some(pending.vertical_speed);
    world
        .systems
        .player
        .movement()
        .handle_move(player_guid, Opcode::MSG_MOVE_KNOCK_BACK, movement_info, world)
        .await
}

/// Handle CMSG_MOVE_SPLINE_DONE for a server-scripted player spline.
pub async fn handle_move_spline_done(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;
    let mut movement_info = MovementInfo::read_from_packet(packet)?;
    let spline_id = packet
        .read_u32()
        .ok_or_else(|| anyhow!("MoveSplineDone: missing spline id"))?;
    let _facing = packet.read_f32();

    let matched = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| match player.movement.pending_spline {
            Some(pending) if pending.id == spline_id => {
                player.movement.pending_spline = None;
                true
            }
            _ => false,
        })
        .unwrap_or(false);
    if !matched || !verify_movement_info(&movement_info) {
        return Ok(());
    }

    movement_info.mover_guid = player_guid;
    let opcode = rebroadcast_opcode(&movement_info);
    world
        .systems
        .player
        .movement()
        .handle_move(player_guid, opcode, movement_info, world)
        .await
}

/// Handle CMSG_MOUNTSPECIAL_ANIM (`WorldSession::HandleMountSpecialAnimOpcode`).
///
/// Broadcasts SMSG_MOUNTSPECIAL_ANIM (the mount's special animation) to nearby
/// players so they see the animation.
pub fn handle_mount_special_anim(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let mut data = WorldPacket::new(Opcode::SMSG_MOUNTSPECIAL_ANIM);
    data.write_guid_raw(player_guid.raw());

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_exclude_self(player_guid, &data);
    Ok(())
}

/// Handle CMSG_MOVE_TIME_SKIPPED (`WorldSession::HandleMoveTimeSkippedOpcode`).
///
/// The client reports it skipped `lag` ms of movement time. We rebroadcast the
/// skip to observers so their extrapolation of this mover stays in sync.
pub fn handle_move_time_skipped(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let guid_raw = packet
        .read_guid_raw()
        .ok_or_else(|| anyhow!("MoveTimeSkipped: missing guid"))?;
    let lag = packet
        .read_u32()
        .ok_or_else(|| anyhow!("MoveTimeSkipped: missing lag"))?;

    // Only the player is a valid mover (no pet/possession), so resolve and bail
    // if the guid isn't controllable by this client.
    let mover = WorldObjectGuid::from_low((guid_raw & 0xFFFF_FFFF) as u32);
    if session.get_mover_from_guid(mover).is_none() {
        return Ok(());
    }

    let mut data = WorldPacket::new(Opcode::MSG_MOVE_TIME_SKIPPED);
    data.write_packed_guid_raw(player_guid.counter() as u64);
    data.write_u32(lag);

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_exclude_self(player_guid, &data);
    Ok(())
}

/// Move types that carry a per-type speed/turn rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpeedMoveType {
    Walk,
    Run,
    RunBack,
    Swim,
    SwimBack,
    TurnRate,
}

impl SpeedMoveType {
    /// Map a `CMSG_FORCE_*_CHANGE_ACK` opcode to its move type.
    fn from_ack_opcode(opcode: Opcode) -> Option<Self> {
        Some(match opcode {
            Opcode::CMSG_FORCE_RUN_SPEED_CHANGE_ACK => Self::Run,
            Opcode::CMSG_FORCE_RUN_BACK_SPEED_CHANGE_ACK => Self::RunBack,
            Opcode::CMSG_FORCE_SWIM_SPEED_CHANGE_ACK => Self::Swim,
            Opcode::CMSG_FORCE_WALK_SPEED_CHANGE_ACK => Self::Walk,
            Opcode::CMSG_FORCE_SWIM_BACK_SPEED_CHANGE_ACK => Self::SwimBack,
            Opcode::CMSG_FORCE_TURN_RATE_CHANGE_ACK => Self::TurnRate,
            _ => return None,
        })
    }

    /// The pending-change type this move type is queued under.
    fn change_type(self) -> Option<MovementChangeType> {
        Some(match self {
            Self::Walk => MovementChangeType::SpeedChangeWalk,
            Self::Run => MovementChangeType::SpeedChangeRun,
            Self::RunBack => MovementChangeType::SpeedChangeRunBack,
            Self::Swim => MovementChangeType::SpeedChangeSwim,
            Self::SwimBack => MovementChangeType::SpeedChangeSwimBack,
            Self::TurnRate => MovementChangeType::RateChangeTurn,
        })
    }

    /// The MSG_MOVE_SET_* opcode used to inform observers of the new value.
    fn observer_opcode(self) -> Opcode {
        match self {
            Self::Walk => Opcode::MSG_MOVE_SET_WALK_SPEED,
            Self::Run => Opcode::MSG_MOVE_SET_RUN_SPEED,
            Self::RunBack => Opcode::MSG_MOVE_SET_RUN_BACK_SPEED,
            Self::Swim => Opcode::MSG_MOVE_SET_SWIM_SPEED,
            Self::SwimBack => Opcode::MSG_MOVE_SET_SWIM_BACK_SPEED,
            Self::TurnRate => Opcode::MSG_MOVE_SET_TURN_RATE,
        }
    }
}

/// Handle the `CMSG_FORCE_*_CHANGE_ACK` family
/// (`WorldSession::HandleForceSpeedChangeAckOpcodes`).
///
/// The client acknowledges a server-forced speed/turn-rate change. The ack must match
/// a change this server actually queued — same counter, same move type, same speed
/// (within 0.01) — otherwise it is dropped. Only then is the authoritative value
/// applied and nearby observers informed via the matching MSG_MOVE_SET_* opcode.
///
/// Not ported: the anticheat `OnWrongAckData` reporting on a mismatched ack.
pub fn handle_force_speed_change_ack(
    session: &WorldSession,
    opcode: Opcode,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let Some(move_type) = SpeedMoveType::from_ack_opcode(opcode) else {
        return Ok(());
    };

    // Wire format (1.12): packed guid, movement counter (u32), movement info, speed (f32).
    let guid_raw = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("SpeedChangeAck: missing guid"))?;
    let movement_counter = packet
        .read_u32()
        .ok_or_else(|| anyhow!("SpeedChangeAck: missing counter"))?;
    let mut movement_info = MovementInfo::read_from_packet(packet)?;
    let new_speed = packet
        .read_f32()
        .ok_or_else(|| anyhow!("SpeedChangeAck: missing speed"))?;

    movement_info.mover_guid = player_guid;

    // Only the player is a valid mover (no pet/possession).
    let mover = WorldObjectGuid::from_low((guid_raw & 0xFFFF_FFFF) as u32);
    if session.get_mover_from_guid(mover).is_none() {
        return Ok(());
    }

    // Verify the client is replying with a change this server actually sent.
    let Some(change_type) = move_type.change_type() else {
        return Ok(());
    };
    let matched = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player.movement.find_pending_speed_change(
                new_speed,
                movement_counter,
                change_type,
            )
        })
        .unwrap_or(false);

    if !matched {
        tracing::warn!(
            "[MOVEMENT] {:?} sent {:?} with counter {} that matches no pending change",
            player_guid,
            opcode,
            movement_counter
        );
        return Ok(());
    }

    let position_valid =
        movement_info.time > session.move_reject_time() && verify_movement_info(&movement_info);

    // Apply the authoritative speed and (if valid) the relocation in one lock.
    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            match move_type {
                SpeedMoveType::Walk => player.movement.walk_speed = new_speed,
                SpeedMoveType::Run => player.movement.run_speed = new_speed,
                SpeedMoveType::Swim => player.movement.swim_speed = new_speed,
                SpeedMoveType::TurnRate => player.movement.turn_rate = new_speed,
                // rcore does not track run-back / swim-back speeds; observers are
                // still notified below.
                SpeedMoveType::RunBack | SpeedMoveType::SwimBack => {}
            }
            if position_valid {
                player.movement.position = movement_info.position;
                player.movement.flags = movement_info.flags.value();
                player.movement.movement_flags = movement_info.flags.value();
                player.movement.timestamp = movement_info.time;
                player.movement.last_movement_time = movement_info.time;
            }
        });

    // Inform observers: packed guid + movement info + new speed.
    // write_to_packet emits the packed mover guid first, matching the C++ layout.
    let mut data = WorldPacket::new(move_type.observer_opcode());
    movement_info.write_to_packet(&mut data);
    data.write_f32(new_speed);

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_exclude_self(player_guid, &data);

    Ok(())
}

/// Movement-flag toggles the client can acknowledge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlagChange {
    WaterWalk,
    FeatherFall,
}

impl FlagChange {
    fn from_ack_opcode(opcode: Opcode) -> Option<Self> {
        Some(match opcode {
            Opcode::CMSG_MOVE_WATER_WALK_ACK => Self::WaterWalk,
            Opcode::CMSG_MOVE_FEATHER_FALL_ACK => Self::FeatherFall,
            _ => return None,
        })
    }

    fn observer_opcode(self) -> Opcode {
        match self {
            Self::WaterWalk => Opcode::MSG_MOVE_WATER_WALK,
            Self::FeatherFall => Opcode::MSG_MOVE_FEATHER_FALL,
        }
    }

    /// The pending-change flag this toggle is queued under.
    fn pending_flag(self) -> MovementFlagChange {
        match self {
            Self::WaterWalk => MovementFlagChange::WaterWalking,
            Self::FeatherFall => MovementFlagChange::SafeFall,
        }
    }
}

/// Handle the `CMSG_MOVE_*_ACK` flag-toggle family
/// (`WorldSession::HandleMovementFlagChangeToggleAck`).
///
/// The client acknowledges a server-forced water-walk / feather-fall toggle. The ack
/// must match a change this server queued (same counter, same flag) or it is dropped;
/// the authoritative flag comes from the queued change, not from the client's payload.
///
/// Not ported: anticheat tests, and the hover ack (the client has no hover ack opcode).
pub fn handle_movement_flag_change_ack(
    session: &WorldSession,
    opcode: Opcode,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    let Some(change) = FlagChange::from_ack_opcode(opcode) else {
        return Ok(());
    };

    // Wire format (1.12): packed guid, movement counter (u32), movement info, apply (u32).
    let guid_raw = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("FlagChangeAck: missing guid"))?;
    let movement_counter = packet
        .read_u32()
        .ok_or_else(|| anyhow!("FlagChangeAck: missing counter"))?;
    let mut movement_info = MovementInfo::read_from_packet(packet)?;
    let _client_apply = packet.read_u32().unwrap_or(0) != 0;

    movement_info.mover_guid = player_guid;

    let mover = WorldObjectGuid::from_low((guid_raw & 0xFFFF_FFFF) as u32);
    if session.get_mover_from_guid(mover).is_none() {
        return Ok(());
    }

    // Verify the client is replying with a toggle this server actually sent, and take
    // the applied value from the queued change rather than trusting the client's byte.
    let apply = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            player
                .movement
                .find_pending_flag_change(movement_counter, change.pending_flag())
        })
        .flatten();

    let Some(apply) = apply else {
        tracing::warn!(
            "[MOVEMENT] {:?} sent {:?} with counter {} that matches no pending change",
            player_guid,
            opcode,
            movement_counter
        );
        return Ok(());
    };

    let position_valid =
        movement_info.time > session.move_reject_time() && verify_movement_info(&movement_info);

    world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            if let FlagChange::WaterWalk = change {
                player.movement.water_walking = apply;
            }
            if position_valid {
                player.movement.position = movement_info.position;
                player.movement.flags = movement_info.flags.value();
                player.movement.movement_flags = movement_info.flags.value();
                player.movement.timestamp = movement_info.time;
                player.movement.last_movement_time = movement_info.time;
            }
        });

    // Observers: packed guid + movement info (no apply byte, matching C++).
    let mut data = WorldPacket::new(change.observer_opcode());
    movement_info.write_to_packet(&mut data);

    world
        .managers
        .broadcast_mgr
        .broadcast_nearby_exclude_self(player_guid, &data);

    Ok(())
}

/// Pick the observer rebroadcast opcode for a movement info: heartbeat while
/// moving, stop otherwise (mirrors the `MOVEFLAG_MASK_MOVING ? HEARTBEAT : STOP`
/// choice in several handlers).
fn rebroadcast_opcode(movement_info: &MovementInfo) -> Opcode {
    use crate::core::common::MoveFlags;

    // MOVEFLAG_MASK_MOVING = forward|backward|strafe l/r|falling far|turn l/r|pitch up/down
    const MASK_MOVING: u32 = 0x0000_0001
        | 0x0000_0002
        | 0x0000_0004
        | 0x0000_0008
        | 0x0000_4000
        | 0x0000_0010
        | 0x0000_0020
        | 0x0000_0040
        | 0x0000_0080;

    if movement_info.flags.has_flag(MoveFlags::from(MASK_MOVING)) {
        Opcode::MSG_MOVE_HEARTBEAT
    } else {
        Opcode::MSG_MOVE_STOP
    }
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

    let (is_rooted, unit_flags) = world
        .managers
        .player_mgr
        .with_player(player_guid, |player| {
            let rooted = (player.unit_flags & UNIT_FLAG_DISABLE_MOVE) != 0;
            (rooted, player.unit_flags)
        })
        .unwrap_or((false, 0));

    trace!(
        "[MOVEMENT] handle_movement for {}: opcode={:?}, is_rooted={}, unit_flags=0x{:08X}",
        player_guid,
        opcode,
        is_rooted,
        unit_flags
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

        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.stand_state = STAND_STATE_STAND;
                player.unit_flags &= !UNIT_FLAG_DISABLE_MOVE;
            });

        session.send_msg(SmsgStandstateUpdate {
            stand_state: STAND_STATE_STAND,
        })?;
    }

    // Continue with normal movement processing
    let mut movement_info = MovementInfo::read_from_packet(packet)?;
    movement_info.mover_guid = player_guid;

    // Do not accept packets sent before the reject window (anticheat backoff).
    if movement_info.time <= session.move_reject_time() {
        return Ok(());
    }

    // Ignore client movement while a teleport is awaiting its ACK; the worldport/
    // teleport-ack handlers own position during that window.
    if session.get_pending_teleport().is_some() {
        return Ok(());
    }

    // Reject physically impossible / out-of-bounds movement info.
    if !verify_movement_info(&movement_info) {
        return Ok(());
    }

    debug!(
        "[MOVE-IN] opcode={:?} guid={:?} orient={:.4} time={} flags=0x{:08X}",
        opcode,
        player_guid,
        movement_info.position.o,
        movement_info.time,
        movement_info.flags.value()
    );

    world
        .systems
        .player
        .movement()
        .handle_move(player_guid, opcode, movement_info, world)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::movement::MoveType;
    use crate::game::player::movement::MovementControllerSender;
    use oxcore_shared::database::Databases;
    use oxcore_shared::protocol::{HighGuid, ObjectGuid};
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn test_world() -> World {
        let pool = || {
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        let databases = Arc::new(Databases {
            world: pool(),
            character: pool(),
            auth: pool(),
            logs: pool(),
        });

        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    /// Register a session-backed player so handlers can look it up.
    fn add_test_player(
        world: &mut World,
    ) -> (Arc<WorldSession>, mpsc::UnboundedReceiver<WorldPacket>) {
        use crate::game::player::player::Player;
        use crate::game::player::PlayerBroadcaster;

        let (tx, rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(1, 1, "Tester".to_string(), 0, tx));
        let player_guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
        session.set_player_guid(Some(player_guid));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), player_guid);
        session.set_client_mover_guid(Some(player_guid));

        let mut player = Player::new(player_guid, "Tester".to_string(), 0, 0, 0, 10, 1, 1, 0);
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(
            session.packet_tx(),
            player_guid,
        )));
        world.managers.player_mgr.add_player(player, 1);
        (session, rx)
    }

    #[tokio::test]
    async fn set_active_mover_accepts_the_player_and_corrects_other_movers() {
        let world = test_world();
        let (tx, _rx) = mpsc::unbounded_channel();
        let session = WorldSession::new(1, 1, "Tester".to_string(), 0, tx);
        let player_guid = ObjectGuid::new_without_entry(HighGuid::Player, 42);
        session.set_player_guid(Some(player_guid));

        let mut player_packet = WorldPacket::new(Opcode::CMSG_SET_ACTIVE_MOVER);
        player_packet.write_guid_raw(player_guid.raw());
        handle_set_active_mover(&session, &mut player_packet, &world).unwrap();
        assert_eq!(session.client_mover_guid(), Some(player_guid));

        let mut other_packet = WorldPacket::new(Opcode::CMSG_SET_ACTIVE_MOVER);
        other_packet.write_guid_raw(ObjectGuid::new_without_entry(HighGuid::Player, 7).raw());
        handle_set_active_mover(&session, &mut other_packet, &world).unwrap();
        assert_eq!(session.client_mover_guid(), Some(player_guid));

        let mut release_packet = WorldPacket::new(Opcode::CMSG_SET_ACTIVE_MOVER);
        release_packet.write_guid_raw(0);
        handle_set_active_mover(&session, &mut release_packet, &world).unwrap();
        assert_eq!(session.client_mover_guid(), Some(player_guid));

        session.set_pending_root_ack(true);
        let mut root_ack = WorldPacket::new(Opcode::CMSG_FORCE_MOVE_ROOT_ACK);
        root_ack.write_packed_guid_raw(player_guid.raw());
        root_ack.write_u32(0);
        handle_move_root_ack(&session, &mut root_ack, &world).unwrap();
        assert!(!session.take_pending_root_ack());
    }

    #[test]
    fn forced_movement_ack_opcodes_map_to_their_observer_messages() {
        let speed_cases = [
            (
                Opcode::CMSG_FORCE_RUN_SPEED_CHANGE_ACK,
                Opcode::MSG_MOVE_SET_RUN_SPEED,
            ),
            (
                Opcode::CMSG_FORCE_WALK_SPEED_CHANGE_ACK,
                Opcode::MSG_MOVE_SET_WALK_SPEED,
            ),
            (
                Opcode::CMSG_FORCE_TURN_RATE_CHANGE_ACK,
                Opcode::MSG_MOVE_SET_TURN_RATE,
            ),
        ];
        for (ack, observer) in speed_cases {
            assert_eq!(
                SpeedMoveType::from_ack_opcode(ack).map(SpeedMoveType::observer_opcode),
                Some(observer)
            );
        }

        assert_eq!(
            FlagChange::from_ack_opcode(Opcode::CMSG_MOVE_WATER_WALK_ACK)
                .map(FlagChange::observer_opcode),
            Some(Opcode::MSG_MOVE_WATER_WALK)
        );
        assert_eq!(
            FlagChange::from_ack_opcode(Opcode::CMSG_MOVE_FEATHER_FALL_ACK)
                .map(FlagChange::observer_opcode),
            Some(Opcode::MSG_MOVE_FEATHER_FALL)
        );
    }

    /// Movement-info body as the client sends it: no leading guid, unlike
    /// `MovementInfo::write_to_packet`, which prefixes the packed mover guid.
    fn write_client_movement_info(packet: &mut WorldPacket) {
        packet.write_u32(0); // flags
        packet.write_u32(1); // client timestamp
        packet.write_f32(0.0); // x
        packet.write_f32(0.0); // y
        packet.write_f32(0.0); // z
        packet.write_f32(0.0); // orientation
        packet.write_u32(0); // fall time
    }

    /// Build a speed ack packet the way the 1.12 client lays it out.
    fn speed_ack_packet(player_guid: ObjectGuid, counter: u32, speed: f32) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::CMSG_FORCE_RUN_SPEED_CHANGE_ACK);
        packet.write_packed_guid_raw(player_guid.raw());
        packet.write_u32(counter);
        write_client_movement_info(&mut packet);
        packet.write_f32(speed);
        packet
    }

    #[tokio::test]
    async fn speed_ack_is_dropped_unless_it_matches_a_queued_change() {
        let mut world = test_world();
        let (session, _rx) = add_test_player(&mut world);
        let player_guid = session.player_guid().expect("player guid");

        // Queue a forced run-speed change of 2x (14.0 yards/sec).
        assert!(MovementControllerSender::add_speed_change_to_controller(
            &world,
            player_guid,
            MoveType::Run,
            2.0,
        ));
        let counter = world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p.movement.movement_counter)
            .expect("player");

        // An ack with a counter we never issued changes nothing and leaves the
        // pending change queued.
        let mut wrong = speed_ack_packet(player_guid, counter + 99, 14.0);
        handle_force_speed_change_ack(
            &session,
            Opcode::CMSG_FORCE_RUN_SPEED_CHANGE_ACK,
            &mut wrong,
            &world,
        )
        .unwrap();
        assert!(world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p.movement.has_pending_movement_change())
            .unwrap());

        // The matching ack applies the speed and clears the queue.
        let mut correct = speed_ack_packet(player_guid, counter, 14.0);
        handle_force_speed_change_ack(
            &session,
            Opcode::CMSG_FORCE_RUN_SPEED_CHANGE_ACK,
            &mut correct,
            &world,
        )
        .unwrap();

        let (run_speed, still_pending) = world
            .managers
            .player_mgr
            .with_player(player_guid, |p| {
                (
                    p.movement.run_speed,
                    p.movement.has_pending_movement_change(),
                )
            })
            .unwrap();
        assert_eq!(run_speed, 14.0);
        assert!(!still_pending);
    }

    #[tokio::test]
    async fn flag_ack_takes_the_applied_value_from_the_queued_change() {
        let mut world = test_world();
        let (session, _rx) = add_test_player(&mut world);
        let player_guid = session.player_guid().expect("player guid");

        assert!(
            MovementControllerSender::add_movement_flag_change_to_controller(
                &world,
                player_guid,
                MovementFlagChange::WaterWalking,
                true,
            )
        );
        let counter = world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p.movement.movement_counter)
            .expect("player");

        // Client claims the flag was cleared; the queued change says applied, and the
        // server trusts its own record.
        let mut packet = WorldPacket::new(Opcode::CMSG_MOVE_WATER_WALK_ACK);
        packet.write_packed_guid_raw(player_guid.raw());
        packet.write_u32(counter);
        write_client_movement_info(&mut packet);
        packet.write_u32(0);

        handle_movement_flag_change_ack(
            &session,
            Opcode::CMSG_MOVE_WATER_WALK_ACK,
            &mut packet,
            &world,
        )
        .unwrap();

        assert!(world
            .managers
            .player_mgr
            .with_player(player_guid, |p| p.movement.water_walking)
            .unwrap());
    }
}
