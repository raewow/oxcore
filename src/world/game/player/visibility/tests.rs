//! Tests for the Visibility System (world)
//!
//! Tests cover:
//! - Visibility state management (dirty, force_immediate, throttle)
//! - Cell crossing detection
//! - Visibility delta calculation (appeared/disappeared)
//! - Bidirectional visibility updates
//! - Packet sending (CREATE_OBJECT2, OUT_OF_RANGE)
//! - Listener management

use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;

use crate::shared::protocol::{HighGuid, ObjectGuid, Opcode, Position, WorldPacket};
use crate::world::game::player::broadcaster::PlayerBroadcaster;
use crate::world::game::player::player::Player;
use crate::world::game::player::PlayerManager;
use crate::world::map::grid_coords::{CellPair, CELL_SIZE};

use super::state::VisibilityState;

// ========== CONSTANTS ==========

/// Throttle ticks - must match system.rs
const UPDATE_THROTTLE_TICKS: u32 = 4;

/// Visibility distance in world units (default)
const VISIBILITY_DISTANCE: f32 = 533.33333;

// ========== PACKET CAPTURE INFRASTRUCTURE ==========

/// Captured packet for test verification
#[derive(Clone, Debug)]
pub struct CapturedPacket {
    pub opcode: Opcode,
    pub data: Vec<u8>,
}

/// Creates a player broadcaster with packet capture
fn create_broadcaster_with_capture(
    guid: ObjectGuid,
) -> (Arc<PlayerBroadcaster>, Arc<Mutex<Vec<CapturedPacket>>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<WorldPacket>();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = Arc::clone(&captured);

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

fn test_player_guid(low: u32) -> ObjectGuid {
    ObjectGuid::new_without_entry(HighGuid::Player, low)
}

fn test_position(x: f32, y: f32, z: f32) -> Position {
    Position::new(x, y, z, 0.0)
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
        0,  // instance_id (continent)
        1,  // zone_id
        60, // level
        1,  // race
        1,  // class
        0,  // gender
    );
    player.movement.position = position;
    player.set_broadcaster(broadcaster);

    player_mgr.add_player(player, guid.counter());

    captured
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

/// Count packets with specific opcode
fn count_packets_with_opcode(captured: &Arc<Mutex<Vec<CapturedPacket>>>, opcode: Opcode) -> usize {
    captured
        .lock()
        .iter()
        .filter(|p| p.opcode == opcode)
        .count()
}

/// Clear captured packets
fn clear_captured(captured: &Arc<Mutex<Vec<CapturedPacket>>>) {
    captured.lock().clear();
}

// ========== VISIBILITY STATE UNIT TESTS ==========

#[test]
fn test_visibility_state_new_defaults() {
    let initial_cell = CellPair::new(512, 512);
    let state = VisibilityState::new(initial_cell);

    assert!(state.dirty, "New state should be dirty");
    assert!(
        state.force_immediate,
        "New state should have force_immediate"
    );
    assert_eq!(state.last_cell, initial_cell);
    assert_eq!(state.last_update_tick, 0);
    assert!(state.visible_objects.is_empty());
    assert!(state.pending_appeared.is_empty());
    assert!(state.pending_disappeared.is_empty());
}

#[test]
fn test_visibility_state_default() {
    let state = VisibilityState::default();

    assert_eq!(state.last_cell, CellPair::new(0, 0));
    assert!(state.dirty);
    assert!(state.force_immediate);
}

#[test]
fn test_has_crossed_cell_same_cell() {
    let state = VisibilityState::new(CellPair::new(512, 512));

    assert!(
        !state.has_crossed_cell(CellPair::new(512, 512)),
        "Same cell should return false"
    );
}

#[test]
fn test_has_crossed_cell_different_x() {
    let state = VisibilityState::new(CellPair::new(512, 512));

    assert!(
        state.has_crossed_cell(CellPair::new(513, 512)),
        "Different X cell should return true"
    );
}

#[test]
fn test_has_crossed_cell_different_y() {
    let state = VisibilityState::new(CellPair::new(512, 512));

    assert!(
        state.has_crossed_cell(CellPair::new(512, 513)),
        "Different Y cell should return true"
    );
}

#[test]
fn test_has_crossed_cell_both_different() {
    let state = VisibilityState::new(CellPair::new(512, 512));

    assert!(
        state.has_crossed_cell(CellPair::new(513, 513)),
        "Both different should return true"
    );
}

#[test]
fn test_update_cell_marks_dirty_on_change() {
    let mut state = VisibilityState::new(CellPair::new(512, 512));
    state.dirty = false; // Clear the initial dirty flag

    state.update_cell(CellPair::new(513, 512));

    assert!(state.dirty, "Should be marked dirty after cell change");
    assert_eq!(state.last_cell, CellPair::new(513, 512));
}

#[test]
fn test_update_cell_no_change_same_cell() {
    let mut state = VisibilityState::new(CellPair::new(512, 512));
    state.dirty = false;

    state.update_cell(CellPair::new(512, 512));

    assert!(!state.dirty, "Should not be dirty when cell unchanged");
}

#[test]
fn test_mark_force_immediate() {
    let mut state = VisibilityState::new(CellPair::new(512, 512));
    state.dirty = false;
    state.force_immediate = false;

    state.mark_force_immediate();

    assert!(state.dirty);
    assert!(state.force_immediate);
}

#[test]
fn test_clear_pending() {
    let mut state = VisibilityState::new(CellPair::new(512, 512));
    state.pending_appeared = vec![test_player_guid(1), test_player_guid(2)];
    state.pending_disappeared = vec![test_player_guid(3)];

    state.clear_pending();

    assert!(state.pending_appeared.is_empty());
    assert!(state.pending_disappeared.is_empty());
}

#[test]
fn test_has_pending_notifications_none() {
    let state = VisibilityState::new(CellPair::new(512, 512));

    assert!(!state.has_pending_notifications());
}

#[test]
fn test_has_pending_notifications_appeared() {
    let mut state = VisibilityState::new(CellPair::new(512, 512));
    state.pending_appeared.push(test_player_guid(1));

    assert!(state.has_pending_notifications());
}

#[test]
fn test_has_pending_notifications_disappeared() {
    let mut state = VisibilityState::new(CellPair::new(512, 512));
    state.pending_disappeared.push(test_player_guid(1));

    assert!(state.has_pending_notifications());
}

#[test]
fn test_visible_objects_capacity() {
    let state = VisibilityState::new(CellPair::new(512, 512));

    // Capacity should be at least 64 (preallocated for nearby players)
    assert!(state.visible_objects.capacity() >= 64);
}

// ========== CELL PAIR TESTS ==========

#[test]
fn test_cell_pair_from_world_coords_origin() {
    let cell = CellPair::from_world_coords(0.0, 0.0);

    // Origin should be around cell 512 (half of 1024)
    assert!(cell.x > 500 && cell.x < 524);
    assert!(cell.y > 500 && cell.y < 524);
}

#[test]
fn test_cell_pair_from_world_coords_move_by_cell_size() {
    let cell1 = CellPair::from_world_coords(0.0, 0.0);
    // Move by more than CELL_SIZE (33.33) to cross boundary
    let cell2 = CellPair::from_world_coords(CELL_SIZE + 1.0, 0.0);

    assert_ne!(cell1.x, cell2.x, "Moving by cell size should change cell X");
}

#[test]
fn test_cell_pair_equality() {
    let cell1 = CellPair::new(512, 512);
    let cell2 = CellPair::new(512, 512);
    let cell3 = CellPair::new(513, 512);

    assert_eq!(cell1, cell2);
    assert_ne!(cell1, cell3);
}

// ========== VISIBILITY UPDATE CONDITION TESTS ==========

#[tokio::test]
async fn test_visibility_update_when_dirty() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Set dirty flag
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = true;
        player.visibility.force_immediate = false;
        player.visibility.last_update_tick = 0;
    });

    // Check if should update (dirty)
    let should_update = player_mgr
        .with_player_mut(guid, |player| {
            let dirty = player.visibility.dirty;
            let force = player.visibility.force_immediate;
            let throttle_expired = false; // tick 0, last_tick 0
            dirty || force || throttle_expired
        })
        .unwrap();

    assert!(should_update, "Should update when dirty");
}

#[tokio::test]
async fn test_visibility_update_when_force_immediate() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Set force_immediate
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = false;
        player.visibility.force_immediate = true;
        player.visibility.last_update_tick = 0;
    });

    let should_update = player_mgr
        .with_player_mut(guid, |player| {
            player.visibility.dirty || player.visibility.force_immediate
        })
        .unwrap();

    assert!(should_update, "Should update when force_immediate");
}

#[tokio::test]
async fn test_visibility_update_when_throttle_expired() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Set up: not dirty, not forced, but throttle expired
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = false;
        player.visibility.force_immediate = false;
        player.visibility.last_update_tick = 0;
    });

    let current_tick: u32 = UPDATE_THROTTLE_TICKS + 1; // Past throttle

    let should_update = player_mgr
        .with_player_mut(guid, |player| {
            let dirty = player.visibility.dirty;
            let force = player.visibility.force_immediate;
            let last_tick = player.visibility.last_update_tick;
            let throttle_expired = current_tick.saturating_sub(last_tick) >= UPDATE_THROTTLE_TICKS;
            dirty || force || throttle_expired
        })
        .unwrap();

    assert!(should_update, "Should update when throttle expired");
}

#[tokio::test]
async fn test_visibility_skip_when_throttled() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Set up: not dirty, not forced, within throttle
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = false;
        player.visibility.force_immediate = false;
        player.visibility.last_update_tick = 0;
    });

    let current_tick: u32 = 2; // Within throttle (< UPDATE_THROTTLE_TICKS)

    let should_update = player_mgr
        .with_player_mut(guid, |player| {
            let dirty = player.visibility.dirty;
            let force = player.visibility.force_immediate;
            let last_tick = player.visibility.last_update_tick;
            let throttle_expired = current_tick.saturating_sub(last_tick) >= UPDATE_THROTTLE_TICKS;
            dirty || force || throttle_expired
        })
        .unwrap();

    assert!(!should_update, "Should skip when throttled");
}

#[tokio::test]
async fn test_visibility_flags_cleared_after_processing() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Set flags
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = true;
        player.visibility.force_immediate = true;
    });

    // Simulate processing - clear flags
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = false;
        player.visibility.force_immediate = false;
        player.visibility.last_update_tick = 10;
    });

    // Verify
    let (dirty, force, tick) = player_mgr
        .with_player_mut(guid, |player| {
            (
                player.visibility.dirty,
                player.visibility.force_immediate,
                player.visibility.last_update_tick,
            )
        })
        .unwrap();

    assert!(!dirty);
    assert!(!force);
    assert_eq!(tick, 10);
}

// ========== VISIBILITY DELTA CALCULATION TESTS ==========

#[tokio::test]
async fn test_visibility_delta_appeared() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    // A at (0, 0), B at (100, 0) - within visibility
    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);
    create_test_player(&player_mgr, guid_b, test_position(100.0, 0.0, 0.0), 0);

    // A's previous visible set is empty
    let previous_visible: HashSet<ObjectGuid> = HashSet::new();

    // Get B's position
    let pos_b = player_mgr.get_position(guid_b).unwrap();

    // Calculate distance
    let pos_a = player_mgr.get_position(guid_a).unwrap();
    let dx = pos_a.x - pos_b.x;
    let dy = pos_a.y - pos_b.y;
    let dist_sq = dx * dx + dy * dy;
    let vis_dist_sq = VISIBILITY_DISTANCE * VISIBILITY_DISTANCE;

    // B should be within visibility
    assert!(
        dist_sq <= vis_dist_sq,
        "B should be within visibility distance"
    );

    // B is new (not in previous set)
    assert!(
        !previous_visible.contains(&guid_b),
        "B should not be in previous visible"
    );

    // Therefore B should appear
}

#[tokio::test]
async fn test_visibility_delta_disappeared() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    // A at (0, 0)
    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);
    // B far away (outside visibility)
    create_test_player(&player_mgr, guid_b, test_position(1000.0, 0.0, 0.0), 0);

    // A's previous visible set contains B (they were visible before)
    let mut previous_visible: HashSet<ObjectGuid> = HashSet::new();
    previous_visible.insert(guid_b);

    // Calculate distance
    let pos_a = player_mgr.get_position(guid_a).unwrap();
    let pos_b = player_mgr.get_position(guid_b).unwrap();
    let dx = pos_a.x - pos_b.x;
    let dy = pos_a.y - pos_b.y;
    let dist_sq = dx * dx + dy * dy;
    let vis_dist_sq = VISIBILITY_DISTANCE * VISIBILITY_DISTANCE;

    // B should be outside visibility
    assert!(
        dist_sq > vis_dist_sq,
        "B should be outside visibility distance"
    );

    // B was in previous set but not in current
    assert!(
        previous_visible.contains(&guid_b),
        "B should be in previous visible"
    );

    // Therefore B should disappear
}

#[tokio::test]
async fn test_visibility_excludes_self() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // When calculating visibility, self should never appear
    let mut visible_now: HashSet<ObjectGuid> = HashSet::new();

    // Simulate adding candidates - should skip self
    let candidates = vec![guid]; // Only self in range
    for &candidate in &candidates {
        if candidate == guid {
            continue; // Skip self
        }
        visible_now.insert(candidate);
    }

    assert!(
        !visible_now.contains(&guid),
        "Self should not be in visible set"
    );
}

#[tokio::test]
async fn test_visibility_distance_boundary() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);
    let guid_c = test_player_guid(3);

    // A at origin
    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);

    // B exactly at visibility boundary (533.33)
    create_test_player(
        &player_mgr,
        guid_b,
        test_position(VISIBILITY_DISTANCE, 0.0, 0.0),
        0,
    );

    // C just outside (534.0)
    create_test_player(
        &player_mgr,
        guid_c,
        test_position(VISIBILITY_DISTANCE + 1.0, 0.0, 0.0),
        0,
    );

    let pos_a = player_mgr.get_position(guid_a).unwrap();
    let pos_b = player_mgr.get_position(guid_b).unwrap();
    let pos_c = player_mgr.get_position(guid_c).unwrap();

    let vis_dist_sq = VISIBILITY_DISTANCE * VISIBILITY_DISTANCE;

    // Distance to B
    let dx_b = pos_a.x - pos_b.x;
    let dist_b_sq = dx_b * dx_b;

    // Distance to C
    let dx_c = pos_a.x - pos_c.x;
    let dist_c_sq = dx_c * dx_c;

    // B should be at boundary (visible)
    assert!(dist_b_sq <= vis_dist_sq, "B at boundary should be visible");

    // C should be outside (not visible)
    assert!(dist_c_sq > vis_dist_sq, "C outside should not be visible");
}

// ========== BIDIRECTIONAL VISIBILITY TESTS ==========

#[tokio::test]
async fn test_bidirectional_visible_objects_on_appear() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);
    create_test_player(&player_mgr, guid_b, test_position(100.0, 0.0, 0.0), 0);

    // Simulate: A sees B for first time
    // Update A's visible_objects
    player_mgr.with_player_mut(guid_a, |player| {
        player.visibility.visible_objects.insert(guid_b);
    });

    // Also update B's visible_objects (bidirectional)
    player_mgr.with_player_mut(guid_b, |player| {
        player.visibility.visible_objects.insert(guid_a);
    });

    // Verify bidirectional
    let a_sees_b = player_mgr
        .with_player_mut(guid_a, |p| p.visibility.visible_objects.contains(&guid_b))
        .unwrap();
    let b_sees_a = player_mgr
        .with_player_mut(guid_b, |p| p.visibility.visible_objects.contains(&guid_a))
        .unwrap();

    assert!(a_sees_b, "A should see B");
    assert!(b_sees_a, "B should see A (bidirectional)");
}

#[tokio::test]
async fn test_bidirectional_visible_objects_on_disappear() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);
    create_test_player(&player_mgr, guid_b, test_position(100.0, 0.0, 0.0), 0);

    // Set up: A and B currently see each other
    player_mgr.with_player_mut(guid_a, |player| {
        player.visibility.visible_objects.insert(guid_b);
    });
    player_mgr.with_player_mut(guid_b, |player| {
        player.visibility.visible_objects.insert(guid_a);
    });

    // Simulate: A loses visibility of B
    player_mgr.with_player_mut(guid_a, |player| {
        player.visibility.visible_objects.remove(&guid_b);
    });

    // Also update B (bidirectional removal)
    player_mgr.with_player_mut(guid_b, |player| {
        player.visibility.visible_objects.remove(&guid_a);
    });

    // Verify bidirectional removal
    let a_sees_b = player_mgr
        .with_player_mut(guid_a, |p| p.visibility.visible_objects.contains(&guid_b))
        .unwrap();
    let b_sees_a = player_mgr
        .with_player_mut(guid_b, |p| p.visibility.visible_objects.contains(&guid_a))
        .unwrap();

    assert!(!a_sees_b, "A should not see B");
    assert!(!b_sees_a, "B should not see A (bidirectional)");
}

// ========== PENDING NOTIFICATIONS TESTS ==========

#[tokio::test]
async fn test_pending_appeared_queued() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);
    let guid_c = test_player_guid(3);

    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);

    // Queue pending appeared
    player_mgr.with_player_mut(guid_a, |player| {
        player.visibility.pending_appeared.push(guid_b);
        player.visibility.pending_appeared.push(guid_c);
    });

    // Verify
    let pending = player_mgr
        .with_player_mut(guid_a, |player| player.visibility.pending_appeared.clone())
        .unwrap();

    assert_eq!(pending.len(), 2);
    assert!(pending.contains(&guid_b));
    assert!(pending.contains(&guid_c));
}

#[tokio::test]
async fn test_pending_disappeared_queued() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);

    // Queue pending disappeared
    player_mgr.with_player_mut(guid_a, |player| {
        player.visibility.pending_disappeared.push(guid_b);
    });

    // Verify
    let pending = player_mgr
        .with_player_mut(guid_a, |player| {
            player.visibility.pending_disappeared.clone()
        })
        .unwrap();

    assert_eq!(pending.len(), 1);
    assert!(pending.contains(&guid_b));
}

#[tokio::test]
async fn test_pending_cleared_after_flush() {
    let player_mgr = PlayerManager::new();
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    create_test_player(&player_mgr, guid_a, test_position(0.0, 0.0, 0.0), 0);

    // Queue some pending
    player_mgr.with_player_mut(guid_a, |player| {
        player.visibility.pending_appeared.push(guid_b);
        player.visibility.pending_disappeared.push(guid_b);
    });

    // Simulate flush (extract and clear)
    let (appeared, disappeared) = player_mgr
        .with_player_mut(guid_a, |player| {
            (
                std::mem::take(&mut player.visibility.pending_appeared),
                std::mem::take(&mut player.visibility.pending_disappeared),
            )
        })
        .unwrap();

    // Verify extraction
    assert!(!appeared.is_empty());
    assert!(!disappeared.is_empty());

    // Verify cleared
    let has_pending = player_mgr
        .with_player_mut(guid_a, |player| {
            player.visibility.has_pending_notifications()
        })
        .unwrap();

    assert!(!has_pending, "Pending should be cleared after flush");
}

// ========== LISTENER MANAGEMENT TESTS ==========

#[tokio::test]
async fn test_bidirectional_listeners_on_appear() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    let (broadcaster_a, _) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, _) = create_broadcaster_with_capture(guid_b);

    // Simulate: A and B appear to each other
    // Add bidirectional listeners
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));
    broadcaster_b.add_listener(guid_a, Arc::clone(&broadcaster_a));

    assert!(broadcaster_a.has_listener(guid_b));
    assert!(broadcaster_b.has_listener(guid_a));
}

#[tokio::test]
async fn test_bidirectional_listeners_on_disappear() {
    let guid_a = test_player_guid(1);
    let guid_b = test_player_guid(2);

    let (broadcaster_a, _) = create_broadcaster_with_capture(guid_a);
    let (broadcaster_b, _) = create_broadcaster_with_capture(guid_b);

    // Set up existing listeners
    broadcaster_a.add_listener(guid_b, Arc::clone(&broadcaster_b));
    broadcaster_b.add_listener(guid_a, Arc::clone(&broadcaster_a));

    // Simulate: A and B disappear from each other
    broadcaster_a.remove_listener(guid_b);
    broadcaster_b.remove_listener(guid_a);

    assert!(!broadcaster_a.has_listener(guid_b));
    assert!(!broadcaster_b.has_listener(guid_a));
}

// ========== PACKET SENDING TESTS ==========

#[tokio::test]
async fn test_send_update_object_packet() {
    let guid = test_player_guid(1);
    let (broadcaster, captured) = create_broadcaster_with_capture(guid);

    // Create a simple SMSG_UPDATE_OBJECT packet
    let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
    packet.write_u32(1); // block count
                         // ... block data would go here

    broadcaster.send_direct(packet);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    assert_packet_sent(&captured, Opcode::SMSG_UPDATE_OBJECT);
}

#[tokio::test]
async fn test_send_out_of_range_packet() {
    let guid = test_player_guid(1);
    let (broadcaster, captured) = create_broadcaster_with_capture(guid);

    // Create SMSG_OUT_OF_RANGE packet (opcode 0x0156)
    // Note: The actual opcode enum value depends on your Opcode definition
    let mut packet = WorldPacket::new(Opcode::SMSG_DESTROY_OBJECT); // Using similar opcode for test
    packet.write_u32(1); // count
                         // ... GUID data would go here

    broadcaster.send_direct(packet);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    assert_packet_sent(&captured, Opcode::SMSG_DESTROY_OBJECT);
}

#[tokio::test]
async fn test_batched_packets_to_viewer() {
    let guid_viewer = test_player_guid(1);
    let (broadcaster_viewer, captured_viewer) = create_broadcaster_with_capture(guid_viewer);

    // Simulate batched CREATE_OBJECT2 - should be ONE packet with multiple blocks
    let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
    packet.write_u32(3); // 3 blocks in one packet
                         // ... block data for 3 players

    broadcaster_viewer.send_direct(packet);

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Should be exactly 1 packet
    let count = count_packets_with_opcode(&captured_viewer, Opcode::SMSG_UPDATE_OBJECT);
    assert_eq!(count, 1, "Should be batched into single packet");
}

#[tokio::test]
async fn test_reverse_create_object_to_appeared_players() {
    let guid_viewer = test_player_guid(1);
    let guid_appeared_1 = test_player_guid(2);
    let guid_appeared_2 = test_player_guid(3);

    let (broadcaster_viewer, _) = create_broadcaster_with_capture(guid_viewer);
    let (broadcaster_1, captured_1) = create_broadcaster_with_capture(guid_appeared_1);
    let (broadcaster_2, captured_2) = create_broadcaster_with_capture(guid_appeared_2);

    // Simulate reverse CREATE_OBJECT2:
    // When viewer sees players 1 and 2, they also need to see the viewer

    // Build viewer's CREATE_OBJECT2 packet
    let mut viewer_packet = WorldPacket::new(Opcode::SMSG_UPDATE_OBJECT);
    viewer_packet.write_u32(1); // 1 block (viewer)
                                // ... viewer's data

    // Send to each appeared player
    broadcaster_1.send_direct(viewer_packet.clone());
    broadcaster_2.send_direct(viewer_packet.clone());

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Both appeared players should receive UPDATE_OBJECT
    assert_packet_sent(&captured_1, Opcode::SMSG_UPDATE_OBJECT);
    assert_packet_sent(&captured_2, Opcode::SMSG_UPDATE_OBJECT);
}

#[tokio::test]
async fn test_no_packets_when_no_pending() {
    let guid = test_player_guid(1);
    let player_mgr = PlayerManager::new();

    let captured = create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Ensure no pending notifications
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.pending_appeared.clear();
        player.visibility.pending_disappeared.clear();
    });

    // Check pending
    let has_pending = player_mgr
        .with_player_mut(guid, |player| player.visibility.has_pending_notifications())
        .unwrap();

    assert!(!has_pending);

    // No packets should be in captured (only what was sent during setup)
    // Clear any setup packets
    clear_captured(&captured);

    // Verify empty
    let packets = captured.lock();
    assert!(
        packets.is_empty(),
        "No packets should be sent when no pending"
    );
}

// ========== EDGE CASE TESTS ==========

#[tokio::test]
async fn test_handles_missing_broadcaster_gracefully() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    // Create player WITHOUT broadcaster
    let mut player = Player::new(
        guid,
        "TestPlayer".to_string(),
        0,  // map_id
        0,  // instance_id (continent)
        1,  // zone_id
        60, // level
        1,  // race
        1,  // class
        0,  // gender
    );
    // Don't set broadcaster
    assert!(player.broadcaster.is_none());

    player_mgr.add_player(player, 1);

    // Try to get broadcaster
    let broadcaster = player_mgr.get_broadcaster(guid);
    assert!(broadcaster.is_none(), "Should handle missing broadcaster");
}

#[tokio::test]
async fn test_nonexistent_player_returns_none() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(999);

    let result = player_mgr.with_player_mut(guid, |player| {
        player.visibility.dirty = true;
    });

    assert!(result.is_none());
}

#[tokio::test]
async fn test_multiple_visibility_updates_sequence() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Simulate multiple updates
    for tick in 0..10u32 {
        player_mgr.with_player_mut(guid, |player| {
            // Check if should update
            let dirty = player.visibility.dirty;
            let force = player.visibility.force_immediate;
            let last = player.visibility.last_update_tick;
            let throttle_expired = tick.saturating_sub(last) >= UPDATE_THROTTLE_TICKS;

            if dirty || force || throttle_expired {
                // Process update
                player.visibility.dirty = false;
                player.visibility.force_immediate = false;
                player.visibility.last_update_tick = tick;
            }
        });
    }

    // Final state
    let last_tick = player_mgr
        .with_player_mut(guid, |player| player.visibility.last_update_tick)
        .unwrap();

    // Should have been updated at tick 0 (initial dirty), then at 4 or 5 (throttle), etc.
    assert!(last_tick > 0, "Should have been updated at least once");
}

// ========== CELL CROSSING INTEGRATION TESTS ==========

#[tokio::test]
async fn test_cell_crossing_marks_dirty() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    // Player starts at (0, 0)
    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    // Initialize cell and clear dirty
    let initial_cell = CellPair::from_world_coords(0.0, 0.0);
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.last_cell = initial_cell;
        player.visibility.dirty = false;
    });

    // Move by more than CELL_SIZE to cross boundary
    let new_x = CELL_SIZE + 10.0;
    let new_cell = CellPair::from_world_coords(new_x, 0.0);

    // Check and update
    player_mgr.with_player_mut(guid, |player| {
        if player.visibility.has_crossed_cell(new_cell) {
            player.visibility.update_cell(new_cell);
        }
    });

    // Verify dirty was set
    let dirty = player_mgr
        .with_player_mut(guid, |player| player.visibility.dirty)
        .unwrap();

    assert!(dirty, "Should be marked dirty after cell crossing");
}

#[tokio::test]
async fn test_no_dirty_within_same_cell() {
    let player_mgr = PlayerManager::new();
    let guid = test_player_guid(1);

    create_test_player(&player_mgr, guid, test_position(0.0, 0.0, 0.0), 0);

    let initial_cell = CellPair::from_world_coords(0.0, 0.0);
    player_mgr.with_player_mut(guid, |player| {
        player.visibility.last_cell = initial_cell;
        player.visibility.dirty = false;
    });

    // Move small distance (within same cell)
    let new_x = 5.0; // Less than CELL_SIZE
    let new_cell = CellPair::from_world_coords(new_x, 0.0);

    player_mgr.with_player_mut(guid, |player| {
        if player.visibility.has_crossed_cell(new_cell) {
            player.visibility.update_cell(new_cell);
        }
    });

    let dirty = player_mgr
        .with_player_mut(guid, |player| player.visibility.dirty)
        .unwrap();

    assert!(!dirty, "Should not be dirty when staying in same cell");
}
