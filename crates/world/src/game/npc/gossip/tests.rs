//! Tests for GossipSystem behavior
//!
//! - Menus are built per player: conditions, NPC flags and per-service checks decide which
//!   options show, and the greeting is the best-matching conditioned row.
//! - A selection is the position in the menu the player was shown; anything else is ignored.
//! - TRAINER and VENDOR options open their window without closing the gossip first.
//! - BANKER sends SMSG_SHOW_BANK; INNKEEPER closes gossip and asks to confirm the new home.

use std::path::PathBuf;
use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

use crate::core::session::WorldSession;
use crate::game::conditions::ConditionEntry;
use crate::game::creature::manager::CreatureTemplate;
use crate::game::creature::Creature;
use crate::game::npc::gossip::types::{
    gossip_option, BroadcastText, GossipMenu, GossipMenuItem, PlayerGossipMenu,
    PreparedGossipOption,
};
use crate::game::player::broadcaster::PlayerBroadcaster;
use crate::game::player::player::Player;
use crate::{Config, World};
use oxcore_db::database::Databases;
use oxcore_shared::protocol::{HighGuid, ObjectGuid, Opcode, Position, WorldPacket};

const NPC_FLAG_GOSSIP: u32 = 0x01;
const NPC_FLAG_TRAINER: u32 = 0x10;
const NPC_FLAG_STABLEMASTER: u32 = 0x2000;
const MENU: u32 = 50;
const ENTRY: u32 = 900;

// ========== INFRASTRUCTURE ==========

fn test_world() -> World {
    let pool = || {
        PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    };
    World::new(
        Arc::new(Databases {
            world: pool(),
            character: pool(),
            auth: pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        }),
        Arc::new(Config::default()),
        50,
        PathBuf::from("."),
    )
}

/// A level-10 player (race 1, class `class`) whose outgoing packets land in the receiver.
fn add_player(world: &World, class: u8) -> (ObjectGuid, mpsc::UnboundedReceiver<WorldPacket>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let session = Arc::new(WorldSession::new(1, 1, "Tester".to_string(), 0, tx));
    let guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
    session.set_player_guid(Some(guid));
    world.session_mgr.add_session(Arc::clone(&session));
    world.session_mgr.register_player(session.id(), guid);

    let mut player = Player::new(guid, "Tester".to_string(), 0, 0, 1, 10, 1, class, 0);
    player.set_broadcaster(Arc::new(PlayerBroadcaster::new(session.packet_tx(), guid)));
    world.managers.player_mgr.add_player(player, 1);
    (guid, rx)
}

/// A creature using gossip menu `MENU` with the given NPC flags.
fn add_npc(world: &World, npc_flags: u32) -> ObjectGuid {
    let template = CreatureTemplate {
        entry: ENTRY,
        name: "Gossiper".to_string(),
        subname: None,
        min_level: 1,
        max_level: 1,
        faction: 35,
        model_id_1: 1,
        model_id_2: 0,
        model_id_3: 0,
        model_id_4: 0,
        scale: 1.0,
        npc_flags,
        unit_flags: 0,
        static_flags1: 0,
        flags_extra: 0,
        creature_type: 7,
        unit_class: 1,
        health_multiplier: 1.0,
        power_multiplier: 1.0,
        armor_multiplier: 1.0,
        damage_multiplier: 1.0,
        damage_variance: 0.0,
        attack_time: 2000,
        rank: 0,
        gossip_menu_id: MENU,
        vendor_id: 0,
        trainer_id: 0,
        trainer_type: 0,
        spells: [0; 4],
        gold_min: 0,
        gold_max: 0,
        loot_id: 0,
        pickpocket_loot_id: 0,
        skinning_loot_id: 0,
        ai: Default::default(),
    };
    world.managers.creature_mgr.add_template(template.clone());
    let guid = ObjectGuid::new_creature(ENTRY, 1);
    world.managers.creature_mgr.add_creature(Creature::new(
        guid,
        ENTRY,
        1,
        Position::default(),
        0,
        0,
        &template,
        1,
        None,
    ));
    guid
}

fn option(menu_id: u32, id: u32, option_id: u32, npcflag: u32, condition_id: u32) -> GossipMenuItem {
    GossipMenuItem {
        menu_id,
        id,
        option_icon: 0,
        option_text: format!("option {id}"),
        option_broadcast_text: 0,
        option_id,
        npc_option_npcflag: npcflag,
        action_menu_id: 0,
        action_poi_id: 0,
        action_script_id: 0,
        box_coded: false,
        box_money: 0,
        box_text: String::new(),
        box_broadcast_text: 0,
        condition_id,
    }
}

fn menu_row(entry: u32, text_id: u32, condition_id: u32) -> GossipMenu {
    GossipMenu {
        entry,
        text_id,
        script_id: 0,
        condition_id,
    }
}

/// Condition `id` holds for players of class `class`.
fn class_condition(world: &World, id: u32, class: u8) {
    world.managers.condition_mgr.add_for_test(
        id,
        ConditionEntry {
            kind: 14,
            values: [0, 1 << (class - 1), 0, 0],
            flags: 0,
        },
    );
}

fn shown_texts(world: &World, player: ObjectGuid) -> Vec<String> {
    world
        .managers
        .player_mgr
        .with_player(player, |p| p.gossip.options.iter().map(|o| o.text.clone()).collect())
        .unwrap()
}

fn sent(rx: &mut mpsc::UnboundedReceiver<WorldPacket>) -> Vec<WorldPacket> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn opcodes(packets: &[WorldPacket]) -> Vec<Opcode> {
    packets.iter().map(|p| p.opcode()).collect()
}

/// Puts one option of type `action` in front of the player as if they had been shown it.
fn show_option(world: &World, player: ObjectGuid, source: ObjectGuid, action: u32) {
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.gossip = PlayerGossipMenu {
            source,
            menu_id: MENU,
            options: vec![PreparedGossipOption {
                action,
                ..Default::default()
            }],
            ..Default::default()
        };
    });
}

// ========== MENU BUILDING ==========

#[tokio::test]
async fn failed_condition_hides_option() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    class_condition(&world, 1, 1);
    class_condition(&world, 2, 8);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(MENU, 0, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 1));
    gossip.add_menu_item(option(MENU, 1, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 2));

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);

    assert_eq!(shown_texts(&world, player), vec!["option 0"]);
}

#[tokio::test]
async fn game_master_sees_conditioned_option_marked() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    world
        .managers
        .player_mgr
        .with_player_mut(player, |p| p.gm_mode = true);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    class_condition(&world, 2, 8);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(MENU, 0, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 2));

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);

    assert_eq!(shown_texts(&world, player), vec!["option 0 (GM mode is ON)"]);
}

#[tokio::test]
async fn option_needs_one_of_its_npc_flags() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 3);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP | NPC_FLAG_STABLEMASTER);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(MENU, 0, gossip_option::GOSSIP, 0, 0));
    gossip.add_menu_item(option(MENU, 1, gossip_option::GOSSIP, NPC_FLAG_TRAINER, 0));
    gossip.add_menu_item(option(MENU, 2, gossip_option::STABLEPET, NPC_FLAG_STABLEMASTER, 0));

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);

    assert_eq!(shown_texts(&world, player), vec!["option 2"]);
}

#[tokio::test]
async fn stable_option_is_for_hunters_only() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_STABLEMASTER);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(MENU, 0, gossip_option::STABLEPET, NPC_FLAG_STABLEMASTER, 0));

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);

    assert!(shown_texts(&world, player).is_empty());
}

#[tokio::test]
async fn default_menu_without_options_offers_shared_options() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 3);
    let npc = add_npc(&world, NPC_FLAG_STABLEMASTER);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(0, 12, gossip_option::STABLEPET, NPC_FLAG_STABLEMASTER, 0));

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);
    assert_eq!(shown_texts(&world, player), vec!["option 12"]);

    // A sub-menu without options stays empty.
    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU + 1);
    assert!(shown_texts(&world, player).is_empty());
}

#[tokio::test]
async fn option_text_follows_broadcast_text_and_gender() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    world
        .managers
        .player_mgr
        .with_player_mut(player, |p| p.gender = 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_broadcast_text(BroadcastText {
        entry: 77,
        male_text: "Hello, sir".to_string(),
        female_text: "Hello, madam".to_string(),
        chat_type: 0,
        language_id: 0,
        sound_id: 0,
        emote_ids: [0; 3],
        emote_delays: [0; 3],
    });
    let mut item = option(MENU, 0, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 0);
    item.option_broadcast_text = 77;
    gossip.add_menu_item(item);

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);

    assert_eq!(shown_texts(&world, player), vec!["Hello, madam"]);
}

#[tokio::test]
async fn greeting_is_highest_met_condition() {
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    class_condition(&world, 3, 1);
    class_condition(&world, 5, 1);
    class_condition(&world, 9, 8);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 100, 0));
    gossip.add_menu(menu_row(MENU, 103, 3));
    gossip.add_menu(menu_row(MENU, 105, 5));
    gossip.add_menu(menu_row(MENU, 109, 9));

    let text = world.systems.gossip.gossip_text_id(&world, player, npc, MENU);
    assert_eq!(text, 105);

    let world = test_world();
    let (player, _rx) = add_player(&world, 2);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    class_condition(&world, 5, 1);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 100, 0));
    gossip.add_menu(menu_row(MENU, 105, 5));
    assert_eq!(world.systems.gossip.gossip_text_id(&world, player, npc, MENU), 100);
}

#[tokio::test]
async fn options_are_numbered_by_position() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(MENU, 4, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 0));
    gossip.add_menu_item(option(MENU, 9, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 0));

    world.systems.gossip.gossip_hello(&world, player, npc).await.unwrap();

    let packets = sent(&mut rx);
    let menu = packets
        .iter()
        .find(|p| p.opcode() == Opcode::SMSG_GOSSIP_MESSAGE)
        .expect("gossip menu sent");
    let data = menu.data();
    // guid(8) text(4) count(4), then index(4) icon(1) coded(1) text...
    assert_eq!(u32::from_le_bytes(data[16..20].try_into().unwrap()), 0);
    let second = data[22..].iter().position(|&b| b == 0).unwrap() + 23;
    assert_eq!(u32::from_le_bytes(data[second..second + 4].try_into().unwrap()), 1);
}

// ========== SELECTION ==========

#[tokio::test]
async fn unshown_option_is_ignored() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    show_option(&world, player, npc, gossip_option::BANKER);

    world.systems.gossip.on_gossip_select(&world, player, npc, 1).await.unwrap();
    let other = ObjectGuid::new_creature(ENTRY, 2);
    world.systems.gossip.on_gossip_select(&world, player, other, 0).await.unwrap();

    assert!(sent(&mut rx).is_empty());
}

#[tokio::test]
async fn trainer_option_does_not_send_gossip_complete() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_TRAINER);
    show_option(&world, player, npc, gossip_option::TRAINER);

    let _ = world.systems.gossip.on_gossip_select(&world, player, npc, 0).await;

    assert!(!opcodes(&sent(&mut rx)).contains(&Opcode::SMSG_GOSSIP_COMPLETE));
}

#[tokio::test]
async fn vendor_options_do_not_send_gossip_complete() {
    for action in [gossip_option::VENDOR, gossip_option::ARMORER] {
        let world = test_world();
        let (player, mut rx) = add_player(&world, 1);
        let npc = add_npc(&world, 0x80);
        show_option(&world, player, npc, action);

        let _ = world.systems.gossip.on_gossip_select(&world, player, npc, 0).await;

        assert!(!opcodes(&sent(&mut rx)).contains(&Opcode::SMSG_GOSSIP_COMPLETE));
    }
}

#[tokio::test]
async fn innkeeper_option_closes_gossip_and_asks_to_bind() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, 0x80);
    show_option(&world, player, npc, gossip_option::INNKEEPER);

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    let packets = sent(&mut rx);
    assert!(opcodes(&packets).contains(&Opcode::SMSG_GOSSIP_COMPLETE));
    let confirm = packets
        .iter()
        .find(|p| p.opcode() == Opcode::SMSG_BINDER_CONFIRM)
        .expect("binder confirm sent");
    assert_eq!(&confirm.data()[..], &npc.raw().to_le_bytes()[..]);
}

#[tokio::test]
async fn banker_option_shows_bank() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, 0x100);
    show_option(&world, player, npc, gossip_option::BANKER);

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    let packets = sent(&mut rx);
    assert!(!opcodes(&packets).contains(&Opcode::SMSG_GOSSIP_COMPLETE));
    let bank = packets
        .iter()
        .find(|p| p.opcode() == Opcode::SMSG_SHOW_BANK)
        .expect("bank shown");
    assert_eq!(&bank.data()[..], &npc.raw().to_le_bytes()[..]);
}

#[tokio::test]
async fn sub_menu_option_opens_next_menu() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu(menu_row(MENU + 1, 2, 0));
    let mut item = option(MENU, 0, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 0);
    item.action_menu_id = (MENU + 1) as i32;
    gossip.add_menu_item(item);
    gossip.add_menu_item(option(MENU + 1, 0, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 0));

    world.systems.gossip.prepare_gossip_menu(&world, player, npc, MENU);
    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    let menu_id = world
        .managers
        .player_mgr
        .with_player(player, |p| p.gossip.menu_id)
        .unwrap();
    assert_eq!(menu_id, MENU + 1);
    let packets = sent(&mut rx);
    let menu = packets
        .iter()
        .find(|p| p.opcode() == Opcode::SMSG_GOSSIP_MESSAGE)
        .expect("sub-menu sent");
    assert_eq!(u32::from_le_bytes(menu.data()[8..12].try_into().unwrap()), 2);
}

// ========== SERVICE OPTIONS ==========

#[tokio::test]
async fn closing_option_credits_speak_to_objective() {
    use crate::game::npc::quest::types::{QuestProgress, QuestSpecialFlags, QuestTemplate};
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    let mut quest = QuestTemplate {
        id: 10,
        title: "Speak to the gossiper".to_string(),
        special_flags: QuestSpecialFlags::SPEAKTO,
        ..QuestTemplate::default()
    };
    quest.req_creature_or_go_id[0] = ENTRY as i32;
    quest.req_creature_or_go_count[0] = 1;
    world.systems.quest.manager.add_quest_template(quest);
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.active_quests.push(QuestProgress::new(10));
    });
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.gossip = PlayerGossipMenu {
            source: npc,
            menu_id: MENU,
            options: vec![PreparedGossipOption {
                action: gossip_option::GOSSIP,
                action_menu: -1,
                ..Default::default()
            }],
            ..Default::default()
        };
    });

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    let packets = opcodes(&sent(&mut rx));
    assert!(packets.contains(&Opcode::SMSG_GOSSIP_COMPLETE));
    assert!(packets.contains(&Opcode::SMSG_QUESTUPDATE_ADD_KILL));
    let count = world
        .managers
        .player_mgr
        .with_player(player, |p| p.active_quests[0].creature_or_go_count[0])
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn quest_option_opens_the_quest() {
    use crate::game::npc::quest::types::QuestTemplate;
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP | 0x02);
    world.systems.quest.manager.add_quest_template(QuestTemplate {
        id: 11,
        title: "Errand".to_string(),
        quest_level: 5,
        ..QuestTemplate::default()
    });
    world.systems.quest.manager.add_creature_quest_starter(ENTRY, 11);
    show_option(&world, player, npc, gossip_option::QUESTGIVER);

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    assert!(opcodes(&sent(&mut rx)).contains(&Opcode::SMSG_QUESTGIVER_QUEST_DETAILS));
}

#[tokio::test]
async fn unlearn_talents_option_asks_for_confirmation() {
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_TRAINER);
    show_option(&world, player, npc, gossip_option::UNLEARNTALENTS);

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    let packets = sent(&mut rx);
    assert!(opcodes(&packets).contains(&Opcode::SMSG_GOSSIP_COMPLETE));
    let confirm = packets
        .iter()
        .find(|p| p.opcode() == Opcode::MSG_TALENT_WIPE_CONFIRM)
        .expect("talent wipe prompt sent");
    assert_eq!(&confirm.data()[0..8], &npc.raw().to_le_bytes()[..]);
}

#[tokio::test]
async fn guild_options_open_their_windows() {
    for (action, opcode) in [
        (gossip_option::PETITIONER, Opcode::SMSG_PETITION_SHOWLIST),
        (gossip_option::TABARDDESIGNER, Opcode::MSG_TABARDVENDOR_ACTIVATE),
    ] {
        let world = test_world();
        let (player, mut rx) = add_player(&world, 1);
        let npc = add_npc(&world, 0x200 | 0x400);
        show_option(&world, player, npc, action);

        world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

        let packets = opcodes(&sent(&mut rx));
        assert!(packets.contains(&Opcode::SMSG_GOSSIP_COMPLETE));
        assert!(packets.contains(&opcode), "{opcode:?} not sent: {packets:?}");
    }
}

// ========== SCRIPTS ==========

fn script_step() -> crate::game::db_scripts::ScriptInfo {
    crate::game::db_scripts::ScriptInfo::default()
}

#[tokio::test]
async fn greeting_starts_its_menu_script() {
    use crate::game::db_scripts::{insert_script, is_scheduled, ScriptTable};
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    insert_script(ScriptTable::Gossip, 7001, vec![script_step()]);
    world.systems.gossip_manager.add_menu(GossipMenu {
        entry: MENU,
        text_id: 1,
        script_id: 7001,
        condition_id: 0,
    });

    world.systems.gossip.gossip_text_id(&world, player, npc, MENU);

    assert!(is_scheduled(ScriptTable::Gossip, 7001, Some(npc), Some(player)));
}

#[tokio::test]
async fn option_starts_its_script() {
    use crate::game::db_scripts::{insert_script, is_scheduled, ScriptTable};
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    insert_script(ScriptTable::Gossip, 7002, vec![script_step()]);
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.gossip = PlayerGossipMenu {
            source: npc,
            menu_id: MENU,
            options: vec![PreparedGossipOption {
                action: gossip_option::GOSSIP,
                action_script: 7002,
                ..Default::default()
            }],
            ..Default::default()
        };
    });

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    assert!(is_scheduled(ScriptTable::Gossip, 7002, Some(npc), Some(player)));
}

// ========== POINTS OF INTEREST ==========

#[tokio::test]
async fn option_marks_its_point_of_interest_before_the_next_menu() {
    use crate::game::npc::gossip::types::PointOfInterest;
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    let gossip = &world.systems.gossip_manager;
    gossip.add_point_of_interest(PointOfInterest {
        entry: 658,
        x: 1.0,
        y: 2.0,
        icon: 6,
        flags: 99,
        data: 0,
        icon_name: "Lion's Pride Inn".to_string(),
    });
    gossip.add_menu(menu_row(MENU + 1, 2, 0));
    world.managers.player_mgr.with_player_mut(player, |p| {
        p.gossip = PlayerGossipMenu {
            source: npc,
            menu_id: MENU,
            options: vec![PreparedGossipOption {
                action: gossip_option::GOSSIP,
                action_poi: 658,
                action_menu: (MENU + 1) as i32,
                ..Default::default()
            }],
            ..Default::default()
        };
    });

    world.systems.gossip.on_gossip_select(&world, player, npc, 0).await.unwrap();

    let packets = opcodes(&sent(&mut rx));
    assert_eq!(packets[0], Opcode::SMSG_GOSSIP_POI);
    assert!(packets.contains(&Opcode::SMSG_GOSSIP_MESSAGE));
}

// ========== FLIGHT MASTERS ==========

#[tokio::test]
async fn flight_master_greeting_teaches_node_instead_of_a_menu() {
    const NPC_FLAG_FLIGHTMASTER: u32 = 0x08;
    let world = test_world();
    let (player, mut rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP | NPC_FLAG_FLIGHTMASTER);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU, 1, 0));
    gossip.add_menu_item(option(MENU, 0, gossip_option::TAXIVENDOR, NPC_FLAG_FLIGHTMASTER, 0));

    world.systems.gossip.gossip_hello(&world, player, npc).await.unwrap();

    let discovered = world
        .managers
        .player_mgr
        .with_player(player, |p| p.gossip.discovered_node)
        .unwrap();
    assert!(discovered);
    assert!(!opcodes(&sent(&mut rx)).contains(&Opcode::SMSG_GOSSIP_MESSAGE));
}

// ========== MENU OVERRIDE ==========

#[tokio::test]
async fn script_command_swaps_the_creature_menu() {
    use crate::game::db_scripts::{commands::SCRIPT_COMMAND_SET_GOSSIP_MENU, run_step, ScriptInfo};
    let world = test_world();
    let (player, _rx) = add_player(&world, 1);
    let npc = add_npc(&world, NPC_FLAG_GOSSIP);
    let gossip = &world.systems.gossip_manager;
    gossip.add_menu(menu_row(MENU + 5, 1, 0));
    gossip.add_menu_item(option(MENU + 5, 0, gossip_option::GOSSIP, NPC_FLAG_GOSSIP, 0));

    let mut step = ScriptInfo {
        command: SCRIPT_COMMAND_SET_GOSSIP_MENU,
        ..ScriptInfo::default()
    };
    step.datalong[0] = MENU + 5;
    run_step(&world, &step, Some(npc), Some(player));

    assert_eq!(world.systems.gossip.default_menu_id(&world, npc), MENU + 5);
    world.systems.gossip.gossip_hello(&world, player, npc).await.unwrap();
    assert_eq!(shown_texts(&world, player), vec!["option 0"]);
}
