//! Tests for GossipSystem behavior
//!
//! Covers the critical invariant fixed in this session:
//! - TRAINER and VENDOR gossip options must NOT send SMSG_GOSSIP_COMPLETE before
//!   their respective windows are opened (vmangos never calls CloseGossip() first).
//! - Other options (INNKEEPER, BANKER, etc.) must send SMSG_GOSSIP_COMPLETE.

use std::sync::Arc;

use parking_lot::Mutex;
use sqlx::mysql::MySqlPoolOptions;
use tokio::sync::mpsc;

use crate::shared::protocol::{HighGuid, ObjectGuid, Opcode, WorldPacket};
use crate::world::core::session::SessionManager;
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::creature::CreatureManager;
use crate::world::game::npc::gossip::manager::GossipManager;
use crate::world::game::npc::gossip::system::GossipSystem;
use crate::world::game::npc::gossip::types::{gossip_option, GossipMenuItem};
use crate::world::game::player::broadcaster::PlayerBroadcaster;
use crate::world::game::player::player::Player;
use crate::world::game::player::PlayerManager;

// ========== INFRASTRUCTURE ==========

#[derive(Clone, Debug)]
struct CapturedPacket {
    opcode: Opcode,
}

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
            });
        }
    });

    let broadcaster = Arc::new(PlayerBroadcaster::new(tx, guid));
    (broadcaster, captured)
}

fn assert_packet_sent(captured: &Arc<Mutex<Vec<CapturedPacket>>>, opcode: Opcode) {
    let packets = captured.lock();
    assert!(
        packets.iter().any(|p| p.opcode == opcode),
        "Expected {:?} but only got: {:?}",
        opcode,
        packets.iter().map(|p| p.opcode).collect::<Vec<_>>()
    );
}

fn assert_packet_not_sent(captured: &Arc<Mutex<Vec<CapturedPacket>>>, opcode: Opcode) {
    let packets = captured.lock();
    assert!(
        !packets.iter().any(|p| p.opcode == opcode),
        "Unexpected {:?} was sent",
        opcode
    );
}

// ========== HELPERS ==========

fn player_guid() -> ObjectGuid {
    ObjectGuid::new_without_entry(HighGuid::Player, 1)
}

/// Build a GossipSystem with one menu item of the given option_id.
/// Returns (system, player_guid, captured_packets).
async fn setup_with_option(
    option_type: u32,
) -> (GossipSystem, ObjectGuid, Arc<Mutex<Vec<CapturedPacket>>>) {
    let guid = player_guid();

    // Player with broadcaster
    let player_mgr = Arc::new(PlayerManager::new());
    let (broadcaster, captured) = create_broadcaster_with_capture(guid);
    let mut player = Player::new(guid, "Tester".to_string(), 0, 0, 1, 10, 1, 1, 0);
    player.set_broadcaster(broadcaster);
    player_mgr.add_player(player, 1);

    // Session + broadcast managers
    let session_mgr = Arc::new(SessionManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, Arc::clone(&player_mgr)));

    // Fake pool — only used for load(), which we don't call in tests
    let pool = MySqlPoolOptions::new()
        .connect_lazy("mysql://test:test@localhost/test")
        .expect("lazy connect should not fail");
    let pool = Arc::new(pool);

    let gossip_mgr = Arc::new(GossipManager::new(Arc::clone(&pool)));
    let creature_mgr = Arc::new(CreatureManager::new(Arc::clone(&pool)));

    // Seed one menu (id=1) with one item of the requested option type
    gossip_mgr.add_menu_item(GossipMenuItem {
        menu_id: 1,
        id: 0,
        option_icon: 0,
        option_text: "Test option".to_string(),
        option_broadcast_text: 0,
        option_id: option_type,
        npc_option_npcflag: 0,
        action_menu_id: 0,
        action_poi_id: 0,
        action_script_id: 0,
        box_coded: false,
        box_money: 0,
        box_text: String::new(),
        box_broadcast_text: 0,
        condition_id: 0,
    });

    let system = GossipSystem::new(gossip_mgr, broadcast_mgr, player_mgr, creature_mgr);
    (system, guid, captured)
}

// ========== TESTS ==========

/// TRAINER option must NOT send SMSG_GOSSIP_COMPLETE.
/// vmangos Player.cpp line 12297: SendTrainerList(guid) with no CloseGossip() before it.
/// The trainer window itself replaces the gossip UI on the client.
#[tokio::test]
async fn trainer_option_does_not_send_gossip_complete() {
    let (system, guid, captured) = setup_with_option(gossip_option::TRAINER).await;
    let npc_guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);

    // handle_option_select errors if option not found; suppress error (it returns Ok here)
    let _ = system.handle_option_select(guid, npc_guid, 1, 0).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    assert_packet_not_sent(&captured, Opcode::SMSG_GOSSIP_COMPLETE);
}

/// VENDOR option must NOT send SMSG_GOSSIP_COMPLETE.
/// vmangos Player.cpp line 12291: SendListInventory() with no CloseGossip() before it.
#[tokio::test]
async fn vendor_option_does_not_send_gossip_complete() {
    let (system, guid, captured) = setup_with_option(gossip_option::VENDOR).await;
    let npc_guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);

    let _ = system.handle_option_select(guid, npc_guid, 1, 0).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    assert_packet_not_sent(&captured, Opcode::SMSG_GOSSIP_COMPLETE);
}

/// ARMORER option must NOT send SMSG_GOSSIP_COMPLETE (same branch as VENDOR).
#[tokio::test]
async fn armorer_option_does_not_send_gossip_complete() {
    let (system, guid, captured) = setup_with_option(gossip_option::ARMORER).await;
    let npc_guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);

    let _ = system.handle_option_select(guid, npc_guid, 1, 0).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    assert_packet_not_sent(&captured, Opcode::SMSG_GOSSIP_COMPLETE);
}

/// INNKEEPER option MUST send SMSG_GOSSIP_COMPLETE (different branch).
#[tokio::test]
async fn innkeeper_option_sends_gossip_complete() {
    let (system, guid, captured) = setup_with_option(gossip_option::INNKEEPER).await;
    let npc_guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);

    let _ = system.handle_option_select(guid, npc_guid, 1, 0).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    assert_packet_sent(&captured, Opcode::SMSG_GOSSIP_COMPLETE);
}

/// BANKER option MUST send SMSG_GOSSIP_COMPLETE.
#[tokio::test]
async fn banker_option_sends_gossip_complete() {
    let (system, guid, captured) = setup_with_option(gossip_option::BANKER).await;
    let npc_guid = ObjectGuid::from_raw(0xF130_0000_00C6_0001);

    let _ = system.handle_option_select(guid, npc_guid, 1, 0).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    assert_packet_sent(&captured, Opcode::SMSG_GOSSIP_COMPLETE);
}
