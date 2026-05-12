//! Movement system - stateless movement processing logic

use anyhow::{anyhow, Result};
use std::time::Duration;

use crate::shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
use crate::world::core::common::{MoveFlags, MovementInfo};
use crate::world::World;

use super::validator;

/// Movement system (stateless - operates on Player.movement via PlayerManager)
pub struct MovementSystem;

impl MovementSystem {
    pub fn new() -> Self {
        Self
    }

    /// Event-driven update from movement packet
    pub async fn handle_move(
        &self,
        player_guid: ObjectGuid,
        opcode: Opcode,
        movement_info: MovementInfo,
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

                // 2. Validate movement
                validator::validate_movement(player_guid, &movement_info, &old_pos)?;

                // 3. Update movement state directly
                player.movement.position = movement_info.position;
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

        let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
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
           || (old_pos.z - new_pos.z).abs() > 0.01 {

            // Remove auras with MOVING or STANDING_CANCELS interrupt flags
            world.systems.auras.remove_auras_with_interrupt_flag(
                player_guid,
                crate::world::game::player::auras::interrupt::AuraInterruptFlags::MOVING.0
                    | crate::world::game::player::auras::interrupt::AuraInterruptFlags::STANDING_CANCELS.0,
                world,
            ).await?;

            // 7.6. Cancel active cast-time spell on movement
            // In vanilla WoW, moving during a cast-time spell cancels the cast.
            let has_active_cast = world.systems.player.manager().with_player(player_guid, |player| {
                // Check Generic slot for a cast-time spell in progress
                player.spells.get_current_spell(crate::world::game::player::spells::state::CurrentSpellType::Generic)
                    .map_or(false, |cast| cast.original_cast_time_ms > 0)
            }).unwrap_or(false);
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
        movement_info: MovementInfo,
        world: &World,
    ) -> Result<()> {
        let (old_pos, map_id, instance_id) = world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                let old_pos = player.movement.position;
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
        let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
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
    pub fn update_player(&self, _player_guid: ObjectGuid, _diff: Duration, _world: &World) -> Result<()> {
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
            opcode, player_guid, broadcast_movement_info.position.o,
            broadcast_movement_info.time, packet.size()
        );
        world
            .managers
            .broadcast_mgr
            .broadcast_nearby_exclude_self(player_guid, &packet)
            ;

        Ok(())
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
fn check_rest_area_exit(
    player_guid: ObjectGuid,
    map_id: u32,
    pos: Position,
    world: &World,
) {
    use crate::world::game::player::environment::RestType;

    // Fast path: read rest state without write lock
    let (rest_type, inn_trigger_id) = match world.managers.player_mgr.with_player(player_guid, |p| {
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
            Some(dbc_entry) => crate::world::game::area_trigger::from_dbc_entry(dbc_entry),
            None => return, // Trigger data gone — leave rest state as-is
        }
    };

    // Still inside the zone? Nothing to do.
    const TOLERANCE: f32 = 5.0;
    if crate::world::game::area_trigger::is_point_in_area_trigger_zone(
        &trigger, map_id, pos.x, pos.y, pos.z, TOLERANCE,
    ) {
        return;
    }

    // Player left the inn — clear rest state
    let player_mgr = &world.managers.player_mgr;
    world.systems.environment.set_rest_type(
        player_guid,
        RestType::No,
        0,
        player_mgr,
    ).ok(); // best-effort

    // Send updated PLAYER_FLAGS to the client
    if let Some(new_flags) = player_mgr.with_player(player_guid, |p| p.player_flags) {
        use crate::shared::messages::update::{
            ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
        };
        use crate::shared::messages::ToWorldPacket;
        use crate::world::game::common::update_fields::PLAYER_FLAGS;

        let world_guid =
            crate::world::core::common::guid::ObjectGuid::from_low(player_guid.counter());
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
        player_guid, inn_trigger_id
    );
}
