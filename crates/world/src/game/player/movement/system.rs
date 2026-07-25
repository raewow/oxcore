//! Movement system - stateless movement processing logic

use anyhow::{anyhow, Result};
use std::time::Duration;

use crate::core::common::{MoveFlags, MovementInfo};
use crate::World;
use oxcore_shared::messages::movement::{spline_flags, SmsgMonsterMove};
use oxcore_shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};

use super::state::PendingKnockback;
use super::state::PendingSpline;
use super::validator;

/// Re-apply the root flag when the client omits it.
///
/// A rooted client that sends a movement packet without MOVEFLAG_ROOT would otherwise
/// clear its own root server-side, since incoming flags are stored verbatim.
pub(super) fn preserve_root_flag(
    player: &crate::game::player::player::Player,
    info: &mut MovementInfo,
) {
    let was_rooted = MoveFlags::from(player.movement.movement_flags).has_flag(MoveFlags::ROOT);

    if was_rooted && !info.flags.has_flag(MoveFlags::ROOT) {
        info.flags.set_flag(MoveFlags::ROOT);
    }
}

/// Movement system (stateless - operates on Player.movement via PlayerManager)
pub struct MovementSystem;

fn knockback_observer_packet(
    movement_info: &MovementInfo,
    pending: PendingKnockback,
) -> WorldPacket {
    let mut packet = WorldPacket::new(Opcode::MSG_MOVE_KNOCK_BACK);
    movement_info.write_to_packet(&mut packet);
    packet.write_f32(pending.cos_angle);
    packet.write_f32(pending.sin_angle);
    packet.write_f32(pending.horizontal_speed);
    packet.write_f32(pending.vertical_speed);
    packet
}

impl MovementSystem {
    pub fn new() -> Self {
        Self
    }

    /// Launch a player away from a source and wait for the matching client ACK.
    pub fn launch_knockback(
        &self,
        player_guid: ObjectGuid,
        cos_angle: f32,
        sin_angle: f32,
        horizontal_speed: f32,
        vertical_speed: f32,
        world: &World,
    ) -> Result<()> {
        let (pending, movement_info) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let counter = player.movement.movement_counter.wrapping_add(1);
                player.movement.movement_counter = counter;
                let pending = PendingKnockback {
                    counter,
                    cos_angle,
                    sin_angle,
                    horizontal_speed,
                    vertical_speed,
                };
                player.movement.pending_knockback = Some(pending);
                let mut movement_info = MovementInfo::new();
                movement_info.mover_guid = player_guid;
                movement_info.flags = MoveFlags::from(player.movement.flags);
                movement_info.position = player.movement.position;
                movement_info.transport_guid = player.movement.transport_guid;
                movement_info.transport_position = player.movement.transport_position;
                movement_info.transport_time = player.movement.transport_time;
                movement_info.fall_time = Some(player.movement.fall_time);
                movement_info.time = player.movement.timestamp;
                (pending, movement_info)
            })
            .ok_or_else(|| anyhow!("Player not found"))?;

        let session = world
            .session_mgr
            .get_session_by_player(player_guid)
            .ok_or_else(|| anyhow!("Player has no active session"))?;
        let mut packet = WorldPacket::new(Opcode::SMSG_MOVE_KNOCK_BACK);
        packet.write_packed_guid_raw(player_guid.raw());
        packet.write_u32(pending.counter);
        packet.write_f32(pending.cos_angle);
        packet.write_f32(pending.sin_angle);
        packet.write_f32(pending.horizontal_speed);
        packet.write_f32(pending.vertical_speed);
        session.send_packet(packet)?;

        let observer_packet = knockback_observer_packet(&movement_info, pending);
        world
            .managers
            .broadcast_mgr
            .broadcast_nearby_exclude_self(player_guid, &observer_packet);
        Ok(())
    }

    /// Start a server-scripted player spline and await its exact completion ID.
    pub fn launch_spline(
        &self,
        player_guid: ObjectGuid,
        destination: Position,
        speed: f32,
        is_walking: bool,
        world: &World,
    ) -> Result<u32> {
        let (origin, spline_id) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let spline_id = player.movement.movement_counter.wrapping_add(1);
                player.movement.movement_counter = spline_id;
                player.movement.pending_spline = Some(PendingSpline {
                    id: spline_id,
                    destination,
                });
                (player.movement.position, spline_id)
            })
            .ok_or_else(|| anyhow!("Player not found"))?;
        let distance = ((destination.x - origin.x).powi(2)
            + (destination.y - origin.y).powi(2)
            + (destination.z - origin.z).powi(2))
        .sqrt();
        let packet = SmsgMonsterMove {
            guid: player_guid,
            position: origin,
            spline_id,
            move_type: 0,
            facing_target: None,
            facing_angle: None,
            spline_flags: if is_walking {
                spline_flags::WALKMODE
            } else {
                spline_flags::RUNMODE
            },
            duration: ((distance / speed.max(f32::EPSILON)) * 1000.0) as u32,
            waypoints: vec![destination],
        };
        let session = world
            .session_mgr
            .get_session_by_player(player_guid)
            .ok_or_else(|| anyhow!("Player has no active session"))?;
        session.send_msg(packet)?;
        Ok(spline_id)
    }

    /// Event-driven update from movement packet
    pub async fn handle_move(
        &self,
        player_guid: ObjectGuid,
        opcode: Opcode,
        mut movement_info: MovementInfo,
        world: &World,
    ) -> Result<()> {
        // Batch all player state access into a single DashMap lookup for performance
        // This reduces overhead from ~600ns (3 lookups) to ~200ns (1 lookup)
        let (old_pos, map_id, instance_id) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                // 1. Get old position for validation and map relocation
                let old_pos = player.movement.position;

                // 1.5. Prevent the client from clearing its own root by omitting the flag
                preserve_root_flag(player, &mut movement_info);

                // 2. Validate movement
                validator::validate_movement(player_guid, &movement_info, &old_pos)?;

                // 3. Update movement state directly
                player.movement.position = movement_info.position;
                player.movement.transport_guid = movement_info.transport_guid;
                player.movement.transport_position = movement_info.transport_position;
                player.movement.transport_time = movement_info.transport_time;
                player.movement.flags = movement_info.flags.value();
                player.movement.timestamp = movement_info.time;
                player.movement.movement_flags = movement_info.flags.value();
                player.movement.last_movement_time = movement_info.time;

                // 4. Update fall tracking
                // JUMPING (0x2000) = player is in a jump/fall
                if movement_info.flags.has_flag(MoveFlags::JUMPING) {
                    if player.movement.fall_time == 0 {
                        player.movement.fall_start_z = old_pos.z;
                    }
                    player.movement.fall_time = movement_info.fall_time.unwrap_or(0);
                } else if player.movement.fall_time > 0 {
                    // Was falling, now stopped
                    player.movement.fall_time = 0;
                    player.movement.fall_start_z = 0.0;
                }

                // 5. Return old_pos, map_id, and instance_id for use outside the closure
                Ok::<_, anyhow::Error>((old_pos, player.map_id, player.instance_id))
            })
            .ok_or_else(|| anyhow!("Player not found"))??;

        // 6. Relocate player in map (grid tracking)
        let new_pos = movement_info.position;

        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        map.relocate_player(player_guid, old_pos, new_pos);

        // 7. Check for cell crossing and mark dirty for visibility update
        // This is O(1) - just marks the player as dirty if they crossed a cell boundary.
        // Actual visibility calculation happens in the map update loop, not inline with movement.
        world
            .systems
            .player
            .visibility()
            .check_cell_crossing(player_guid, old_pos, new_pos, world);

        // 7.5. Check if movement should interrupt auras (food/drink)
        // Only check if player actually moved (not just turned)
        if (old_pos.x - new_pos.x).abs() > 0.01
            || (old_pos.y - new_pos.y).abs() > 0.01
            || (old_pos.z - new_pos.z).abs() > 0.01
        {
            // Remove auras with MOVING or STANDING_CANCELS interrupt flags
            world.systems.auras.remove_auras_with_interrupt_flag(
                player_guid,
                crate::game::player::auras::interrupt::AuraInterruptFlags::MOVING.0
                    | crate::game::player::auras::interrupt::AuraInterruptFlags::STANDING_CANCELS.0,
                world,
            ).await?;

            // 7.6. Cancel active cast-time spell on movement
            // In vanilla WoW, moving during a cast-time spell cancels the cast.
            let has_active_cast = world
                .systems
                .player
                .manager()
                .with_player(player_guid, |player| {
                    // Check Generic slot for a cast-time spell in progress
                    player
                        .spells
                        .get_current_spell(
                            crate::game::player::spells::state::CurrentSpellType::Generic,
                        )
                        .map_or(false, |cast| cast.original_cast_time_ms > 0)
                })
                .unwrap_or(false);
            if has_active_cast {
                let _ = world.systems.spells.cancel_cast(player_guid, world).await;
            }

            // 7.7. Check if player left a tavern rest area
            check_rest_area_exit(player_guid, map_id, new_pos, world);
        }

        // 8. Broadcast movement to nearby players
        self.broadcast_movement(player_guid, opcode, movement_info, world)?;

        Ok(())
    }

    /// Synchronous movement handler for map-level processing.
    /// Called from Map::update() for buffered movement packets.
    /// Identical to handle_move but not async (handle_move has no .await calls).
    pub fn handle_move_from_buffer(
        &self,
        player_guid: ObjectGuid,
        opcode: Opcode,
        mut movement_info: MovementInfo,
        world: &World,
    ) -> Result<()> {
        let (old_pos, map_id, instance_id) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let old_pos = player.movement.position;
                preserve_root_flag(player, &mut movement_info);
                validator::validate_movement(player_guid, &movement_info, &old_pos)?;
                player.movement.position = movement_info.position;
                player.movement.flags = movement_info.flags.value();
                player.movement.timestamp = movement_info.time;
                player.movement.movement_flags = movement_info.flags.value();
                player.movement.last_movement_time = movement_info.time;
                // JUMPING (0x2000) = player is in a jump/fall
                if movement_info.flags.has_flag(MoveFlags::JUMPING) {
                    if player.movement.fall_time == 0 {
                        player.movement.fall_start_z = old_pos.z;
                    }
                    player.movement.fall_time = movement_info.fall_time.unwrap_or(0);
                } else if player.movement.fall_time > 0 {
                    player.movement.fall_time = 0;
                    player.movement.fall_start_z = 0.0;
                }
                Ok::<_, anyhow::Error>((old_pos, player.map_id, player.instance_id))
            })
            .ok_or_else(|| anyhow!("Player not found"))??;

        let new_pos = movement_info.position;
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        map.relocate_player(player_guid, old_pos, new_pos);

        world
            .systems
            .player
            .visibility()
            .check_cell_crossing(player_guid, old_pos, new_pos, world);

        self.broadcast_movement(player_guid, opcode, movement_info, world)?;

        Ok(())
    }

    /// Periodic update for server-forced movement (future)
    pub fn update_player(
        &self,
        _player_guid: ObjectGuid,
        _diff: Duration,
        _world: &World,
    ) -> Result<()> {
        // Future: handle knockbacks, scripted movement, etc.
        Ok(())
    }

    /// Broadcast movement to nearby players using raw MSG_MOVE_* packets
    /// This matches the old core's behavior - sends the same opcode the client sent
    fn broadcast_movement(
        &self,
        player_guid: ObjectGuid,
        opcode: Opcode,
        movement_info: MovementInfo,
        world: &World,
    ) -> Result<()> {
        // The client has no handler for this opcode, so observers must not receive it.
        // It is sent when a jump clips something on the way up, cutting the ascent short;
        // the relocation above still applies.
        if opcode == Opcode::CMSG_MOVE_FALL_RESET {
            return Ok(());
        }

        // Get current player position from PlayerManager (sole authority for position)
        let current_position = world
            .managers
            .player_mgr
            .get_position(player_guid)
            .unwrap_or_else(Position::default);

        // Create raw movement packet with the same opcode the client sent
        let mut packet = WorldPacket::new(opcode);

        // Create updated movement_info for broadcasting
        // CRITICAL: Use the player's CURRENT position (after update) for broadcasting, not the packet position
        // This ensures we broadcast the exact position the player now has, which is especially important
        // for MSG_MOVE_SET_FACING where the client might send slightly stale x/y coordinates
        let mut broadcast_movement_info = movement_info.clone();
        // Use current player position (which has the validated/normalized position we just set)
        broadcast_movement_info.position = current_position;

        // Keep client's original timestamp - DO NOT replace with server time
        // The client's timestamps are relative to the time base sent in SMSG_LOGIN_SETTIMESPEED
        // Replacing with server time breaks movement interpolation because the time bases don't match
        // broadcast_movement_info.time is already correct from movement_info.clone()

        // Write movement info (with server time and current position)
        // This properly handles all conditional fields (transport, fall/jump, spline)
        broadcast_movement_info.write_to_packet(&mut packet);

        // Knockback observers receive the acknowledged launch vector after the
        // movement block, matching MSG_MOVE_KNOCK_BACK in the reference core.
        if opcode == Opcode::MSG_MOVE_KNOCK_BACK {
            packet.write_f32(broadcast_movement_info.jump_cos_angle.unwrap_or(0.0));
            packet.write_f32(broadcast_movement_info.jump_sin_angle.unwrap_or(0.0));
            packet.write_f32(broadcast_movement_info.jump_xy_speed.unwrap_or(0.0));
            packet.write_f32(broadcast_movement_info.jump_velocity.unwrap_or(0.0));
        }

        // Safety check: Ensure broadcaster exists before broadcasting
        // This prevents crashes during login or when player is in an invalid state
        let broadcaster = world.managers.player_mgr.get_broadcaster(player_guid);
        if broadcaster.is_none() {
            tracing::debug!(
                "Cannot broadcast movement: no broadcaster for player {:?}",
                player_guid
            );
            return Ok(());
        }

        // Broadcast to nearby players (exclude self)
        tracing::debug!(
            "[MOVE-BROADCAST] opcode={:?} guid={:?} orient={:.4} time={} pkt_len={}",
            opcode,
            player_guid,
            broadcast_movement_info.position.o,
            broadcast_movement_info.time,
            packet.size()
        );
        world
            .managers
            .broadcast_mgr
            .broadcast_nearby_exclude_self(player_guid, &packet);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::{HighGuid, Position};

    #[test]
    fn knockback_observer_packet_preserves_movement_info_and_velocity() {
        let guid = ObjectGuid::new_without_entry(HighGuid::Player, 7);
        let mut movement_info = MovementInfo::new();
        movement_info.mover_guid = guid;
        movement_info.flags = MoveFlags::FORWARD;
        movement_info.position = Position::new(1.0, 2.0, 3.0, 0.5);
        movement_info.time = 42;
        movement_info.fall_time = Some(9);
        let pending = PendingKnockback {
            counter: 3,
            cos_angle: 0.25,
            sin_angle: 0.75,
            horizontal_speed: 12.0,
            vertical_speed: 4.0,
        };

        let mut packet = knockback_observer_packet(&movement_info, pending);

        assert_eq!(packet.opcode(), Opcode::MSG_MOVE_KNOCK_BACK);
        assert_eq!(packet.read_packed_guid_raw(), Some(guid.raw()));
        assert_eq!(packet.read_u32(), Some(MoveFlags::FORWARD.value()));
        assert_eq!(packet.read_u32(), Some(42));
        assert_eq!(packet.read_f32(), Some(1.0));
        assert_eq!(packet.read_f32(), Some(2.0));
        assert_eq!(packet.read_f32(), Some(3.0));
        assert_eq!(packet.read_f32(), Some(0.5));
        assert_eq!(packet.read_u32(), Some(9));
        assert_eq!(packet.read_f32(), Some(0.25));
        assert_eq!(packet.read_f32(), Some(0.75));
        assert_eq!(packet.read_f32(), Some(12.0));
        assert_eq!(packet.read_f32(), Some(4.0));
    }
}

impl Default for MovementSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a player has moved outside their current tavern rest area.
///
/// When `RestType::InTavern`, the player's `inn_trigger_id` records which trigger
/// they entered. On each position change we test whether they're still inside that
/// trigger zone. If not, clear the rest state and send the updated PLAYER_FLAGS.
///
/// City rest (`RestType::InCity`) is zone-based, not trigger-based, so it is not
/// checked here.
fn check_rest_area_exit(player_guid: ObjectGuid, map_id: u32, pos: Position, world: &World) {
    use crate::game::player::environment::RestType;

    // Fast path: read rest state without write lock
    let (rest_type, inn_trigger_id) =
        match world.managers.player_mgr.with_player(player_guid, |p| {
            (p.environment.rest_type, p.environment.inn_trigger_id)
        }) {
            Some(v) => v,
            None => return,
        };

    if rest_type != RestType::InTavern || inn_trigger_id == 0 {
        return;
    }

    // Look up the trigger geometry (DB template first, DBC fallback)
    let trigger = if let Some(t) = world.managers.area_trigger_mgr.get_template(inn_trigger_id) {
        t
    } else {
        let dbc = world.dbc.read();
        match dbc.get_area_trigger(inn_trigger_id) {
            Some(dbc_entry) => crate::game::area_trigger::from_dbc_entry(dbc_entry),
            None => return, // Trigger data gone — leave rest state as-is
        }
    };

    // Still inside the zone? Nothing to do.
    const TOLERANCE: f32 = 5.0;
    if crate::game::area_trigger::is_point_in_area_trigger_zone(
        &trigger, map_id, pos.x, pos.y, pos.z, TOLERANCE,
    ) {
        return;
    }

    // Player left the inn — clear rest state
    let player_mgr = &world.managers.player_mgr;
    world
        .systems
        .environment
        .set_rest_type(player_guid, RestType::No, 0, player_mgr)
        .ok(); // best-effort

    // Send updated PLAYER_FLAGS to the client
    if let Some(new_flags) = player_mgr.with_player(player_guid, |p| p.player_flags) {
        use crate::game::common::update_fields::PLAYER_FLAGS;
        use oxcore_shared::messages::update::{
            ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
        };
        use oxcore_shared::messages::ToWorldPacket;

        let world_guid = crate::core::common::guid::ObjectGuid::from_low(player_guid.counter());
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                .set_field(PLAYER_FLAGS, new_flags),
        ));
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(player_guid, update);
    }

    tracing::debug!(
        "Player {} left tavern rest area (trigger {}), cleared rest state",
        player_guid,
        inn_trigger_id
    );
}
