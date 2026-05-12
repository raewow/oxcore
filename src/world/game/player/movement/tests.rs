//! Tests for the Movement System (world)
//!
//! Tests cover:
//! - Movement validation (anti-cheat checks)
//! - Movement state updates (position, flags, timestamps)
//! - Fall tracking (start, update, reset)
//! - Movement broadcasting (packet verification)

use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;

use crate::shared::protocol::{HighGuid, ObjectGuid, Opcode, Position, WorldPacket};
use crate::world::core::common::{MoveFlags, MovementInfo};
use crate::world::game::player::broadcaster::PlayerBroadcaster;
use crate::world::game::player::player::Player;
use crate::world::game::player::PlayerManager;

use super::state::MovementState;
use super::validator;

// ========== PACKET CAPTURE INFRASTRUCTURE ==========

/// Captured packet for test verification
#[derive(Clone, Debug)]
pub struct CapturedPacket {
    pub opcode: Opcode,
    pub data: Vec<u8>,
}

/// Creates a player with packet capture for testing
/// Returns (broadcaster, captured_packets)
fn create_broadcaster_with_capture(
    guid: ObjectGuid,
) -> (Arc<PlayerBroadcaster>, Arc<Mutex<Vec<CapturedPacket>>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<WorldPacket>();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = Arc::clone(&captured);

    // Spawn receiver to capture packets
    tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            captured_clone.lock().push(CapturedPacket {
                opcode: packet.opcode(),
                data: packet.contents().to_vec(),
            });
        }
    });

    let broadcaster = Arc::new(PlayerBroadcaster::new(tx, guid));
    (broadcaster, captured)
}

// ========== TEST HELPERS ==========

/// Helper to create test ObjectGuid
fn test_player_guid(low: u32) -> ObjectGuid {
    ObjectGuid::new_without_entry(HighGuid::Player, low)
}

/// Helper to create a test position
fn test_position(x: f32, y: f32, z: f32) -> Position {
    Position::new(x, y, z, 0.0)
}

/// Helper to create movement info with specific values
fn create_movement_info(
    mover_guid: ObjectGuid,
    position: Position,
    flags: u32,
    timestamp: u32,
) -> MovementInfo {
    let mut info = MovementInfo::new();
    info.mover_guid = mover_guid;
    info.position = position;
    info.flags = MoveFlags::new(flags);
    info.time = timestamp;
    info
}

/// Helper to verify a packet was sent with specific opcode
fn assert_packet_sent(captured: &Arc<Mutex<Vec<CapturedPacket>>>, expected_opcode: Opcode) {
    let packets = captured.lock();
    assert!(
        packets.iter().any(|p| p.opcode == expected_opcode),
        "Expected packet with opcode {:?} not found. Found: {:?}",
        expected_opcode,
        packets.iter().map(|p| p.opcode).collect::<Vec<_>>()
    );
}

/// Helper to verify no packet was sent with specific opcode
fn assert_packet_not_sent(captured: &Arc<Mutex<Vec<CapturedPacket>>>, opcode: Opcode) {
    let packets = captured.lock();
    assert!(
        !packets.iter().any(|p| p.opcode == opcode),
        "Unexpected packet with opcode {:?} was sent",
        opcode
    );
}

/// Get the count of packets with specific opcode
fn count_packets_with_opcode(captured: &Arc<Mutex<Vec<CapturedPacket>>>, opcode: Opcode) -> usize {
    captured.lock().iter().filter(|p| p.opcode == opcode).count()
}

/// Create a test player and add to manager
fn create_test_player(
    player_mgr: &PlayerManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
) -> Arc<Mutex<Vec<CapturedPacket>>> {
    let (broadcaster, captured) = create_broadcaster_with_capture(guid);

    let mut player = Player::new(
        guid,
        format!("TestPlayer{}", guid.counter()),
        map_id,
        0, // instance_id (continent)
        1, // zone_id
        60, // level
        1,  // race (human)
        1,  // class (warrior)
        0,  // gender (male)
    );
    player.movement.position = position;
    player.set_broadcaster(broadcaster);

    player_mgr.add_player(player, guid.counter()); // Use counter as account_id

    captured
}

// ========== MOVEMENT STATE TESTS ==========

#[test]
fn test_movement_state_default_values() {
    let state = MovementState::default();

    assert_eq!(state.position.x, 0.0);
    assert_eq!(state.position.y, 0.0);
    assert_eq!(state.position.z, 0.0);
    assert_eq!(state.flags, 0);
    assert_eq!(state.timestamp, 0);
    assert_eq!(state.fall_start_z, 0.0);
    assert_eq!(state.fall_time, 0);
    assert_eq!(state.walk_speed, 2.5);
    assert_eq!(state.run_speed, 7.0);
    assert_eq!(state.swim_speed, 4.7222);
    assert!((state.turn_rate - 3.14159).abs() < 0.001);
}

#[test]
fn test_movement_state_clone() {
    let mut state = MovementState::default();
    state.position = test_position(100.0, 200.0, 50.0);
    state.flags = MoveFlags::FORWARD.value();
    state.timestamp = 12345;

    let cloned = state.clone();

    assert_eq!(cloned.position.x, 100.0);
    assert_eq!(cloned.position.y, 200.0);
    assert_eq!(cloned.position.z, 50.0);
    assert_eq!(cloned.flags, MoveFlags::FORWARD.value());
    assert_eq!(cloned.timestamp, 12345);
}

// ========== MOVE FLAGS TESTS ==========

#[test]
fn test_move_flags_has_flag() {
    let flags = MoveFlags::new(MoveFlags::FORWARD.value() | MoveFlags::JUMPING.value());

    assert!(flags.has_flag(MoveFlags::FORWARD));
    assert!(flags.has_flag(MoveFlags::JUMPING));
    assert!(!flags.has_flag(MoveFlags::BACKWARD));
    assert!(!flags.has_flag(MoveFlags::SWIMMING));
}

#[test]
fn test_move_flags_set_and_remove() {
    let mut flags = MoveFlags::NONE;

    flags.set_flag(MoveFlags::FORWARD);
    assert!(flags.has_flag(MoveFlags::FORWARD));

    flags.set_flag(MoveFlags::STRAFE_LEFT);
    assert!(flags.has_flag(MoveFlags::FORWARD));
    assert!(flags.has_flag(MoveFlags::STRAFE_LEFT));

    flags.remove_flag(MoveFlags::FORWARD);
    assert!(!flags.has_flag(MoveFlags::FORWARD));
    assert!(flags.has_flag(MoveFlags::STRAFE_LEFT));
}

#[test]
fn test_move_flags_jumping_constant() {
    // Verify JUMPING flag has expected value (0x00002000) - vanilla 1.12
    assert_eq!(MoveFlags::JUMPING.value(), 0x00002000);
}

// ========== MOVEMENT VALIDATION TESTS ==========

#[test]
fn test_validate_movement_accepts_valid_movement() {
    let player_guid = test_player_guid(1);
    let old_pos = test_position(100.0, 100.0, 0.0);
    let new_pos = test_position(105.0, 105.0, 0.0);

    let movement_info = create_movement_info(player_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(result.is_ok(), "Valid movement should be accepted");
}

#[test]
fn test_validate_movement_rejects_mismatched_guid() {
    let player_guid = test_player_guid(1);
    let other_guid = test_player_guid(2);
    let old_pos = test_position(100.0, 100.0, 0.0);
    let new_pos = test_position(105.0, 105.0, 0.0);

    // Movement info has different mover_guid than player
    let movement_info = create_movement_info(other_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(result.is_err(), "Mismatched GUID should be rejected");
    assert!(
        result.unwrap_err().to_string().contains("non-self"),
        "Error should mention non-self GUID"
    );
}

#[test]
fn test_validate_movement_rejects_out_of_bounds_x() {
    let player_guid = test_player_guid(1);
    let old_pos = test_position(100.0, 100.0, 0.0);
    let new_pos = test_position(25000.0, 100.0, 0.0); // x > 20000

    let movement_info = create_movement_info(player_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(result.is_err(), "Out of bounds X should be rejected");
    assert!(
        result.unwrap_err().to_string().contains("out of map bounds"),
        "Error should mention map bounds"
    );
}

#[test]
fn test_validate_movement_rejects_out_of_bounds_y() {
    let player_guid = test_player_guid(1);
    let old_pos = test_position(100.0, 100.0, 0.0);
    let new_pos = test_position(100.0, -25000.0, 0.0); // y < -20000

    let movement_info = create_movement_info(player_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(result.is_err(), "Out of bounds Y should be rejected");
}

#[test]
fn test_validate_movement_rejects_out_of_bounds_z() {
    let player_guid = test_player_guid(1);
    let old_pos = test_position(100.0, 100.0, 0.0);
    let new_pos = test_position(100.0, 100.0, 30000.0); // z > 20000

    let movement_info = create_movement_info(player_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(result.is_err(), "Out of bounds Z should be rejected");
}

#[test]
fn test_validate_movement_accepts_large_distance() {
    // Large movements (>50 yards) are logged but NOT rejected
    // They could be legitimate teleports
    let player_guid = test_player_guid(1);
    let old_pos = test_position(0.0, 0.0, 0.0);
    let new_pos = test_position(100.0, 0.0, 0.0); // 100 yards away

    let movement_info = create_movement_info(player_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(
        result.is_ok(),
        "Large distance should be logged but accepted"
    );
}

#[test]
fn test_validate_movement_accepts_boundary_position() {
    // Position exactly at boundary (20000) should be accepted
    let player_guid = test_player_guid(1);
    let old_pos = test_position(100.0, 100.0, 0.0);
    let new_pos = test_position(20000.0, 20000.0, 20000.0);

    let movement_info = create_movement_info(player_guid, new_pos, 0, 1000);

    let result = validator::validate_movement(player_guid, &movement_info, &old_pos);
    assert!(result.is_ok(), "Boundary position should be accepted");
}

// ========== MOVEMENT INFO PACKET TESTS ==========

#[test]
fn test_movement_info_write_basic() {
    let guid = test_player_guid(1);
    let pos = test_position(100.0, 200.0, 50.0);

    let mut info = MovementInfo::new();
    info.mover_guid = guid;
    info.position = pos;
    info.flags = MoveFlags::FORWARD;
    info.time = 12345;

    let mut packet = WorldPacket::new(Opcode::MSG_MOVE_HEARTBEAT);
    info.write_to_packet(&mut packet);

    // Packet should contain data
    assert!(!packet.contents().is_empty(), "Packet should have content");
}

#[test]
fn test_movement_info_write_with_falling() {
    let guid = test_player_guid(1);
    let pos = test_position(100.0, 200.0, 50.0);

    let mut info = MovementInfo::new();
    info.mover_guid = guid;
    info.position = pos;
    info.flags = MoveFlags::JUMPING;
    info.time = 12345;
    info.fall_time = Some(500);
    info.jump_velocity = Some(-7.5);
    info.jump_sin_angle = Some(0.5);
    info.jump_cos_angle = Some(0.866);
    info.jump_xy_speed = Some(7.0);

    let mut packet = WorldPacket::new(Opcode::MSG_MOVE_JUMP);
    info.write_to_packet(&mut packet);

    // Packet with falling data should be larger
    assert!(
        packet.contents().len() > 30,
        "Falling packet should be larger"
    );
}

#[test]
fn test_movement_info_default() {
    let info = MovementInfo::default();

    assert_eq!(info.mover_guid, ObjectGuid::empty());
    assert_eq!(info.flags, MoveFlags::NONE);
    assert_eq!(info.time, 0);
    assert!(info.transport_guid.is_none());
    assert!(info.fall_time.is_none());
}

// ========== PLAYER MANAGER STATE TESTS ==========

#[tokio::test]
async fn test_player_manager_position_update() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);
    let initial_pos = test_position(100.0, 200.0, 50.0);

    create_test_player(&player_mgr, guid, initial_pos, 0);

    // Verify initial position
    let pos = player_mgr.get_position(guid);
    assert!(pos.is_some());
    let pos = pos.unwrap();
    assert_eq!(pos.x, 100.0);
    assert_eq!(pos.y, 200.0);
    assert_eq!(pos.z, 50.0);

    // Update position via with_player_mut
    let new_pos = test_position(150.0, 250.0, 60.0);
    player_mgr.with_player_mut(guid, |player| {
        player.movement.position = new_pos;
    });

    // Verify updated position
    let pos = player_mgr.get_position(guid).unwrap();
    assert_eq!(pos.x, 150.0);
    assert_eq!(pos.y, 250.0);
    assert_eq!(pos.z, 60.0);
}

#[tokio::test]
async fn test_player_manager_movement_flags_update() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Initial flags should be 0
    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.movement_flags, 0);

    // Update flags
    let new_flags = MoveFlags::FORWARD.value() | MoveFlags::STRAFE_LEFT.value();
    player_mgr.with_player_mut(guid, |player| {
        player.movement.flags = new_flags;
        player.movement.movement_flags = new_flags;
    });

    // Verify updated flags
    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.movement_flags, new_flags);
}

#[tokio::test]
async fn test_player_manager_timestamp_update() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Update timestamp
    let timestamp = 54321u32;
    player_mgr.with_player_mut(guid, |player| {
        player.movement.timestamp = timestamp;
        player.movement.last_movement_time = timestamp;
    });

    // Verify
    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.timestamp, 54321);
    assert_eq!(state.last_movement_time, 54321);
}

// ========== FALL TRACKING TESTS ==========

#[tokio::test]
async fn test_fall_tracking_start() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);
    let start_pos = test_position(100.0, 100.0, 100.0);

    create_test_player(&player_mgr, guid, start_pos, 0);

    // Simulate starting to fall
    player_mgr.with_player_mut(guid, |player| {
        // Player wasn't falling before
        assert_eq!(player.movement.fall_time, 0);

        // Start falling - record the starting Z
        player.movement.fall_start_z = player.movement.position.z;
        player.movement.fall_time = 0; // Just started
    });

    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.fall_start_z, 100.0);
}

#[tokio::test]
async fn test_fall_tracking_update() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(100.0, 100.0, 100.0), 0);

    // Set up initial fall state
    player_mgr.with_player_mut(guid, |player| {
        player.movement.fall_start_z = 100.0;
        player.movement.fall_time = 500; // Already falling for 500ms
    });

    // Update fall time as player continues falling
    player_mgr.with_player_mut(guid, |player| {
        player.movement.fall_time = 1000; // Now falling for 1000ms
        player.movement.position.z = 80.0; // Dropped 20 units
    });

    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.fall_time, 1000);
    assert_eq!(state.fall_start_z, 100.0);
    assert_eq!(state.position.z, 80.0);
}

#[tokio::test]
async fn test_fall_tracking_reset_on_land() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(100.0, 100.0, 100.0), 0);

    // Set up falling state
    player_mgr.with_player_mut(guid, |player| {
        player.movement.fall_start_z = 100.0;
        player.movement.fall_time = 1500;
        player.movement.position.z = 50.0;
    });

    // Simulate landing (no longer falling)
    player_mgr.with_player_mut(guid, |player| {
        // Calculate fall distance before reset
        let _fall_distance = player.movement.fall_start_z - player.movement.position.z;

        // Reset fall tracking
        player.movement.fall_time = 0;
        player.movement.fall_start_z = 0.0;
    });

    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.fall_time, 0);
    assert_eq!(state.fall_start_z, 0.0);
}

// ========== BROADCASTER TESTS ==========

#[tokio::test]
async fn test_broadcaster_listener_management() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    let (broadcaster_a, _captured_a) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, _captured_b) = create_broadcaster_with_capture(guid_b);

    // Initially no listeners
    assert_eq!(broadcaster_a.listener_count(), 0);
    assert!(!broadcaster_a.has_listener(guid_b));

    // Add B as listener to A
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));

    assert_eq!(broadcaster_a.listener_count(), 1);
    assert!(broadcaster_a.has_listener(guid_b));

    // Remove listener
    broadcaster_a.remove_listener(guid_b);

    assert_eq!(broadcaster_a.listener_count(), 0);
    assert!(!broadcaster_a.has_listener(guid_b));
}

#[tokio::test]
async fn test_broadcaster_cannot_add_self_as_listener() {
    let guid = test_player_guid(1);
    let (broadcaster, _) = create_broadcaster_with_capture(guid);

    // Try to add self as listener - should be ignored
    broadcaster.add_listener(guid, Arc::clone(&broadcaster));

    assert_eq!(broadcaster.listener_count(), 0);
    assert!(!broadcaster.has_listener(guid));
}

#[tokio::test]
async fn test_broadcaster_send_direct() {
    let guid = test_player_guid(1);
    let (broadcaster, captured) = create_broadcaster_with_capture(guid);

    // Send a packet
    let mut packet = WorldPacket::new(Opcode::MSG_MOVE_HEARTBEAT);
    packet.write_u32(0); // flags
    packet.write_u32(12345); // time

    broadcaster.send_direct(packet);

    // Allow async processing
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Verify packet was captured
    assert_packet_sent(&captured, Opcode::MSG_MOVE_HEARTBEAT);
}

#[tokio::test]
async fn test_broadcaster_clear_listeners() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);
    let guid_c = test_player_guid(3);

    let (broadcaster_a, _) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, _) = create_broadcaster_with_capture(guid_b);
    let (broadcaster_c, _) = create_broadcaster_with_capture(guid_c);

    // Add multiple listeners
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));
    broadcaster_a.add_listener(guid_c, Arc::clone(&broadcaster_c));

    assert_eq!(broadcaster_a.listener_count(), 2);

    // Clear all
    broadcaster_a.clear_listeners();

    assert_eq!(broadcaster_a.listener_count(), 0);
}

// ========== MOVEMENT BROADCAST RECIPIENT TESTS ==========

#[tokio::test]
async fn test_broadcast_to_listeners_excludes_self() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    let (broadcaster_a, captured_a) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, captured_b) = create_broadcaster_with_capture(guid_b);

    // B listens to A
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));

    // Simulate broadcasting movement from A
    // In real code, broadcast_nearby_exclude_self sends to all listeners
    // Here we verify the listener pattern:
    // - A's broadcaster has B as listener
    // - When A moves, packet goes to B, not back to A

    let mut packet = WorldPacket::new(Opcode::MSG_MOVE_START_FORWARD);
    packet.write_u32(0);

    // Send to all listeners (simulating broadcast_nearby)
    let listeners = broadcaster_a.listeners().read();
    for (_, listener_broadcaster) in listeners.iter() {
        listener_broadcaster.send_direct(packet.clone());
    }
    drop(listeners);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // B should receive the packet
    assert_packet_sent(&captured_b, Opcode::MSG_MOVE_START_FORWARD);

    // A should NOT receive the packet (we didn't send to self)
    let packets_a = captured_a.lock();
    assert!(
        packets_a.is_empty(),
        "Self should not receive broadcast packet"
    );
}

#[tokio::test]
async fn test_broadcast_to_multiple_listeners() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);
    let guid_c = test_player_guid(3);
    let guid_d = test_player_guid(4);

    let (broadcaster_a, _) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, captured_b) = create_broadcaster_with_capture(guid_b);
    let (broadcaster_c, captured_c) = create_broadcaster_with_capture(guid_c);
    let (broadcaster_d, captured_d) = create_broadcaster_with_capture(guid_d);

    // B, C, D all listen to A
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));
    broadcaster_a.add_listener(guid_c, Arc::clone(&broadcaster_c));
    broadcaster_a.add_listener(guid_d, Arc::clone(&broadcaster_d));

    // Broadcast movement
    let mut packet = WorldPacket::new(Opcode::MSG_MOVE_HEARTBEAT);
    packet.write_u32(0);

    let listeners = broadcaster_a.listeners().read();
    for (_, listener_broadcaster) in listeners.iter() {
        listener_broadcaster.send_direct(packet.clone());
    }
    drop(listeners);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // All listeners should receive the packet
    assert_packet_sent(&captured_b, Opcode::MSG_MOVE_HEARTBEAT);
    assert_packet_sent(&captured_c, Opcode::MSG_MOVE_HEARTBEAT);
    assert_packet_sent(&captured_d, Opcode::MSG_MOVE_HEARTBEAT);
}

#[tokio::test]
async fn test_broadcast_uses_same_opcode() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    let (broadcaster_a, _) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, captured_b) = create_broadcaster_with_capture(guid_b);

    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));

    // Test different movement opcodes
    let opcodes = [
        Opcode::MSG_MOVE_START_FORWARD,
        Opcode::MSG_MOVE_START_BACKWARD,
        Opcode::MSG_MOVE_STOP,
        Opcode::MSG_MOVE_JUMP,
        Opcode::MSG_MOVE_HEARTBEAT,
    ];

    for opcode in opcodes {
        let mut packet = WorldPacket::new(opcode);
        packet.write_u32(0);

        let listeners = broadcaster_a.listeners().read();
        for (_, listener_broadcaster) in listeners.iter() {
            listener_broadcaster.send_direct(packet.clone());
        }
        drop(listeners);
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    // Verify all opcodes were received
    for opcode in opcodes {
        assert_packet_sent(&captured_b, opcode);
    }
}

// ========== BIDIRECTIONAL LISTENER TESTS ==========

#[tokio::test]
async fn test_bidirectional_listeners() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    let (broadcaster_a, captured_a) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, captured_b) = create_broadcaster_with_capture(guid_b);

    // Set up bidirectional listeners (A can see B, B can see A)
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));
    broadcaster_b.add_listener(guid_a, Arc::clone(&broadcaster_a));

    // A broadcasts movement
    let mut packet_a = WorldPacket::new(Opcode::MSG_MOVE_START_FORWARD);
    packet_a.write_u32(0);

    let listeners_a = broadcaster_a.listeners().read();
    for (_, listener) in listeners_a.iter() {
        listener.send_direct(packet_a.clone());
    }
    drop(listeners_a);

    // B broadcasts movement
    let mut packet_b = WorldPacket::new(Opcode::MSG_MOVE_START_BACKWARD);
    packet_b.write_u32(0);

    let listeners_b = broadcaster_b.listeners().read();
    for (_, listener) in listeners_b.iter() {
        listener.send_direct(packet_b.clone());
    }
    drop(listeners_b);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // B receives A's movement
    assert_packet_sent(&captured_b, Opcode::MSG_MOVE_START_FORWARD);

    // A receives B's movement
    assert_packet_sent(&captured_a, Opcode::MSG_MOVE_START_BACKWARD);
}

// ========== EDGE CASE TESTS ==========

#[tokio::test]
async fn test_player_not_found_returns_none() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(999);

    assert!(player_mgr.get_position(guid).is_none());
    assert!(player_mgr.get_movement_state(guid).is_none());
    assert!(player_mgr.get_broadcaster(guid).is_none());
}

#[tokio::test]
async fn test_with_player_mut_returns_none_for_missing_player() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(999);

    let result = player_mgr.with_player_mut(guid, |player| {
        player.movement.position = test_position(1.0, 2.0, 3.0);
        42 // Return value
    });

    assert!(result.is_none());
}

#[tokio::test]
async fn test_multiple_position_updates_in_sequence() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Simulate rapid position updates
    let positions = [
        test_position(1.0, 0.0, 0.0),
        test_position(2.0, 0.0, 0.0),
        test_position(3.0, 0.0, 0.0),
        test_position(4.0, 0.0, 0.0),
        test_position(5.0, 0.0, 0.0),
    ];

    for (i, pos) in positions.iter().enumerate() {
        player_mgr.with_player_mut(guid, |player| {
            player.movement.position = *pos;
            player.movement.timestamp = (i as u32 + 1) * 100;
        });

        // Verify immediately
        let current = player_mgr.get_position(guid).unwrap();
        assert_eq!(current.x, pos.x);
    }

    // Final position check
    let final_pos = player_mgr.get_position(guid).unwrap();
    assert_eq!(final_pos.x, 5.0);

    let state = player_mgr.get_movement_state(guid).unwrap();
    assert_eq!(state.timestamp, 500);
}
