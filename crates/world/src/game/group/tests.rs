//! Group system unit tests

use oxcore_shared::game::group::{
    group_update_flags, CachedGroup, GroupData, GroupError, GroupInvite, GroupMember, LootMethod,
    MemberStatus, ERR_ALREADY_IN_GROUP_S, ERR_BAD_PLAYER_NAME_S, ERR_GROUP_FULL,
    ERR_IGNORING_YOU_S, ERR_NOT_LEADER, ERR_PARTY_RESULT_OK, ERR_PLAYER_WRONG_FACTION,
    ERR_TARGET_NOT_IN_GROUP_S, MAX_GROUP_SIZE, MAX_RAID_SIZE, MAX_RAID_SUBGROUPS, PARTY_OP_INVITE,
    PARTY_OP_LEAVE,
};
use oxcore_shared::protocol::ObjectGuid;

#[test]
fn test_group_data_new() {
    let leader_guid = ObjectGuid::new_player(1);
    let group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    assert_eq!(group.id, 1);
    assert_eq!(group.leader_guid, leader_guid);
    assert_eq!(group.leader_name, "TestLeader");
    assert_eq!(group.member_count(), 1);
    assert!(!group.is_raid);
    assert!(!group.is_full());
    assert!(group.has_member(leader_guid));
    assert!(group.is_leader(leader_guid));
}

#[test]
fn test_group_add_member() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    let result = group.add_member(member2_guid, "Member2".to_string());
    assert!(result.is_ok());
    assert_eq!(group.member_count(), 2);
    assert!(group.has_member(member2_guid));
    assert!(!group.is_leader(member2_guid));
}

#[test]
fn test_group_remove_member() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    let removed = group.remove_member(member2_guid);
    assert!(removed.is_some());
    assert_eq!(group.member_count(), 1);
    assert!(!group.has_member(member2_guid));
}

#[test]
fn test_group_is_full_party() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    // Add 4 more members to make a full 5-man party
    for i in 2..=5 {
        let guid = ObjectGuid::new_player(i);
        group.add_member(guid, format!("Member{}", i)).unwrap();
    }

    assert!(group.is_full());
    assert_eq!(group.member_count(), 5);

    // Try to add another member
    let result = group.add_member(ObjectGuid::new_player(6), "Member6".to_string());
    assert!(result.is_err());
}

#[test]
fn test_group_convert_to_raid() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    assert!(!group.is_raid);
    group.convert_to_raid();
    assert!(group.is_raid);
    assert_eq!(group.max_size(), MAX_RAID_SIZE);
}

#[test]
fn test_group_set_leader() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    let result = group.promote_new_leader(member2_guid);
    assert!(result.is_ok());
    assert_eq!(group.leader_guid, member2_guid);
    assert!(group.is_leader(member2_guid));
    assert!(!group.is_leader(leader_guid));
}

#[test]
fn test_group_set_assistant() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    // Raid-only feature
    group.convert_to_raid();

    let result = group.set_assistant(member2_guid, true);
    assert!(result.is_ok());
    assert!(group.is_assistant(member2_guid));
    assert!(group.is_leader_or_assistant(member2_guid));
}

#[test]
fn test_group_change_subgroup() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    // Raid-only feature
    group.convert_to_raid();

    let result = group.change_subgroup(member2_guid, 3);
    assert!(result.is_ok());
    assert_eq!(group.get_member(member2_guid).unwrap().subgroup, 3);
}

#[test]
fn test_group_swap_subgroups() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    let member3_guid = ObjectGuid::new_player(3);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();
    group
        .add_member(member3_guid, "Member3".to_string())
        .unwrap();

    // Raid-only feature
    group.convert_to_raid();

    // Put member2 in subgroup 2
    group.change_subgroup(member2_guid, 2).unwrap();

    // Swap subgroups
    let result = group.swap_subgroups(member2_guid, member3_guid);
    assert!(result.is_ok());
    assert_eq!(group.get_member(member2_guid).unwrap().subgroup, 0);
    assert_eq!(group.get_member(member3_guid).unwrap().subgroup, 2);
}

#[test]
fn test_group_set_target_icon() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let target_guid = ObjectGuid::new_creature(100, 1);

    let result = group.set_target_icon(0, target_guid);
    assert!(result.is_ok());
    assert_eq!(group.target_icons[0], target_guid);

    // Setting same target to different icon should clear the old one
    let result = group.set_target_icon(1, target_guid);
    assert!(result.is_ok());
    assert_eq!(group.target_icons[0], ObjectGuid::empty());
    assert_eq!(group.target_icons[1], target_guid);
}

#[test]
fn test_member_status_flags() {
    let status = MemberStatus::new()
        .with_flag(MemberStatus::PVP)
        .with_flag(MemberStatus::AFK);

    assert!(status.is_online());
    assert!(status.has_flag(MemberStatus::PVP));
    assert!(status.has_flag(MemberStatus::AFK));
    assert!(!status.has_flag(MemberStatus::DEAD));

    let status2 = status.without_flag(MemberStatus::AFK);
    assert!(!status2.has_flag(MemberStatus::AFK));
    assert!(status2.has_flag(MemberStatus::PVP));
}

#[test]
fn test_loot_method_conversion() {
    assert_eq!(LootMethod::from(0), LootMethod::FreeForAll);
    assert_eq!(LootMethod::from(1), LootMethod::RoundRobin);
    assert_eq!(LootMethod::from(2), LootMethod::MasterLooter);
    assert_eq!(LootMethod::from(3), LootMethod::GroupLoot);
    assert_eq!(LootMethod::from(4), LootMethod::NeedBeforeGreed);
    assert_eq!(LootMethod::from(255), LootMethod::GroupLoot); // Invalid defaults to GroupLoot
}

#[test]
fn test_group_invite() {
    let invite = GroupInvite::new(ObjectGuid::new_player(1), "TestInviter".to_string(), 123);

    assert_eq!(invite.inviter_guid, ObjectGuid::new_player(1));
    assert_eq!(invite.inviter_name, "TestInviter");
    assert_eq!(invite.group_id, 123);
}

#[test]
fn test_group_select_new_leader() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    let member3_guid = ObjectGuid::new_player(3);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();
    group
        .add_member(member3_guid, "Member3".to_string())
        .unwrap();

    // Remove current leader
    group.remove_member(leader_guid);

    // Select new leader
    let new_leader = group.select_new_leader(|_| true, false);
    assert!(new_leader.is_some());
    assert!(group.is_leader(new_leader.unwrap()));
}

#[test]
fn test_group_get_member_by_name() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    // Case-insensitive search
    let found = group.get_member_by_name("member2");
    assert!(found.is_some());
    assert_eq!(found.unwrap().guid, member2_guid);

    let not_found = group.get_member_by_name("NonExistent");
    assert!(not_found.is_none());
}

#[test]
fn test_group_member_guids() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    let guids = group.get_member_guids();
    assert_eq!(guids.len(), 2);
    assert!(guids.contains(&leader_guid));
    assert!(guids.contains(&member2_guid));
}

#[test]
fn test_group_online_member_guids() {
    let leader_guid = ObjectGuid::new_player(1);
    let mut group = GroupData::new(1, leader_guid, "TestLeader".to_string());

    let member2_guid = ObjectGuid::new_player(2);
    group
        .add_member(member2_guid, "Member2".to_string())
        .unwrap();

    // Set member2 offline
    group.set_member_status(member2_guid, MemberStatus::offline());

    let online_guids = group.get_online_member_guids();
    assert_eq!(online_guids.len(), 1);
    assert!(online_guids.contains(&leader_guid));
    assert!(!online_guids.contains(&member2_guid));
}

// ========== Phase 5: group loot rolls ==========

use crate::game::group::rolls::{can_use_item, LootContext};
use crate::game::items::manager::ItemTemplate;
use crate::game::loot::{Loot, LootItem};
use crate::game::player::player::Player;
use crate::{Config, World};
use oxcore_db::database::Databases;
use oxcore_shared::protocol::Position;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use std::sync::Arc;

fn test_world() -> World {
    let pool = || {
        PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    };
    let databases = Arc::new(Databases {
        world: pool(),
        character: pool(),
        auth: pool(),
        logs: oxcore_db::database::lazy_logs_pool(),
    });
    World::new(
        databases,
        Arc::new(Config::default()),
        50,
        PathBuf::from("."),
    )
}

fn armor(entry: u32, subclass: u32, required_level: u32) -> ItemTemplate {
    ItemTemplate {
        entry,
        required_level,
        item_class: 4,
        item_subclass: subclass,
        ..Default::default()
    }
}

/// Insert an online player with the given position/map into the world, and register
/// them in `group 1` with the given loot method.
fn add_group_player(
    world: &World,
    guid: ObjectGuid,
    class: u8,
    pos: Position,
    loot_method: LootMethod,
) {
    let mut player = Player::new(guid, "Tester".to_string(), 1, 0, 0, 60, 1, class, 0);
    player.movement.position = pos;
    world.managers.player_mgr.add_player(player, 1);

    let mut group = world
        .systems
        .group
        .groups
        .entry(1)
        .or_insert_with(|| GroupData::new(1, guid, "Tester".to_string()));
    if !group.has_member(guid) {
        group.add_member(guid, "Tester".to_string()).unwrap();
    }
    group.loot_method = loot_method;
    group.loot_threshold = 2;
}

/// A single over-threshold item (quality 3) in the loot of `loot_guid`, on map 1.
fn loot_with_quality_item(
    world: &World,
    loot_guid: ObjectGuid,
    item_id: u32,
    quality: u8,
    allowed_looters: Vec<ObjectGuid>,
) {
    world.managers.item_mgr.add_template(ItemTemplate {
        entry: item_id,
        quality,
        ..Default::default()
    });
    let mut loot = Loot::new();
    loot.add_item(LootItem {
        slot: 0,
        item_id,
        count: 1,
        is_looted: false,
        is_blocked: false,
        is_counted: false,
        roll_winner: None,
        is_underthreshold: false,
        freeforall: false,
        loot_owner: None,
    });
    loot.allowed_looters = allowed_looters;
    world.systems.loot_manager.insert_loot(loot_guid, loot);
}

fn roll_context() -> LootContext {
    LootContext {
        position: Position { x: 0.0, y: 0.0, z: 0.0, o: 0.0 },
        map_id: 1,
        instance_id: 0,
        rank: 0,
    }
}

fn item_state(world: &World, loot_guid: ObjectGuid, slot: u8) -> Option<LootItem> {
    world
        .systems
        .loot_manager
        .get_loot(loot_guid)?
        .items
        .iter()
        .find(|i| i.slot == slot)
        .cloned()
}

fn has_open_roll(world: &World, group_id: u32) -> bool {
    world.systems.group.loot_rolls.get(&group_id).is_some()
}

#[test]
fn test_can_use_item_armor_proficiency() {
    // Cloth: every class.
    let cloth = armor(1, 1, 1);
    assert!(can_use_item(1, 60, &cloth)); // warrior
    assert!(can_use_item(8, 60, &cloth)); // mage

    // Leather: not warriors or paladins.
    let leather = armor(2, 2, 1);
    assert!(!can_use_item(1, 60, &leather)); // warrior
    assert!(!can_use_item(2, 60, &leather)); // paladin
    assert!(can_use_item(4, 60, &leather)); // rogue
    assert!(can_use_item(8, 60, &leather)); // mage

    // Mail: warrior/paladin/hunter/shaman only.
    let mail = armor(3, 3, 1);
    assert!(can_use_item(1, 60, &mail)); // warrior
    assert!(can_use_item(2, 60, &mail)); // paladin
    assert!(can_use_item(3, 60, &mail)); // hunter
    assert!(can_use_item(7, 60, &mail)); // shaman
    assert!(!can_use_item(8, 60, &mail)); // mage
    assert!(!can_use_item(4, 60, &mail)); // rogue

    // Plate: warrior/paladin only.
    let plate = armor(4, 4, 1);
    assert!(can_use_item(1, 60, &plate));
    assert!(can_use_item(2, 60, &plate));
    assert!(!can_use_item(7, 60, &plate)); // shaman
    assert!(!can_use_item(5, 60, &plate)); // priest

    // Weapons (item_class != 4) are never proficiency-gated.
    let weapon = ItemTemplate {
        entry: 5,
        item_class: 2,
        ..Default::default()
    };
    assert!(can_use_item(8, 60, &weapon));
}

#[test]
fn test_can_use_item_required_level_gate() {
    let plate = armor(10, 4, 40);
    assert!(!can_use_item(1, 39, &plate));
    assert!(can_use_item(1, 40, &plate));
    assert!(can_use_item(1, 60, &plate));
}

#[tokio::test]
async fn test_group_loot_below_threshold_is_underthreshold_and_not_rolled() {
    let world = test_world();
    let loot_guid = ObjectGuid::new_creature(500, 1);
    let a = ObjectGuid::new_player(1);

    add_group_player(&world, a, 1, Position { x: 0.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::GroupLoot);
    loot_with_quality_item(&world, loot_guid, 9000, 0, vec![a]); // quality 0 < threshold 2

    world
        .systems
        .group
        .group_loot(&world, 1, loot_guid, roll_context())
        .await;

    let item = item_state(&world, loot_guid, 0).expect("item present");
    assert!(item.is_underthreshold, "below-threshold items are never rolled");
    assert!(!item.is_blocked, "no roll was opened");
    assert!(!has_open_roll(&world, 1), "no roll queued");
}

#[tokio::test]
async fn test_single_looter_auto_award() {
    let world = test_world();
    let loot_guid = ObjectGuid::new_creature(500, 2);
    let solo = ObjectGuid::new_player(1);

    add_group_player(&world, solo, 1, Position { x: 0.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::GroupLoot);
    loot_with_quality_item(&world, loot_guid, 9001, 3, vec![solo]);

    world
        .systems
        .group
        .group_loot(&world, 1, loot_guid, roll_context())
        .await;

    assert!(!has_open_roll(&world, 1), "single looter is never asked to roll");
    let item = item_state(&world, loot_guid, 0).expect("item present");
    assert!(!item.is_blocked, "item unblocked after auto-award");
    // The test world has no bags, so `add_item` cannot store it; the item stays in the
    // loot reserved for the winner (reference: failed store leaves the item in the loot).
    assert_eq!(item.loot_owner, Some(solo));
}

#[tokio::test]
async fn test_need_beats_greed_and_all_pass_unblocks() {
    let world = test_world();
    let loot_guid = ObjectGuid::new_creature(500, 3);
    let needer = ObjectGuid::new_player(1);
    let greeder = ObjectGuid::new_player(2);

    add_group_player(&world, needer, 1, Position { x: 0.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::GroupLoot);
    add_group_player(&world, greeder, 4, Position { x: 5.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::GroupLoot);
    loot_with_quality_item(&world, loot_guid, 9002, 3, vec![needer, greeder]);

    world
        .systems
        .group
        .group_loot(&world, 1, loot_guid, roll_context())
        .await;
    assert!(has_open_roll(&world, 1), "two voters open a roll");
    assert!(item_state(&world, loot_guid, 0).unwrap().is_blocked);

    world
        .systems
        .group
        .count_roll_vote(&world, 1, needer, loot_guid, 0, crate::game::loot::RollVote::Need)
        .await;
    // Second voter outstanding; the roll must still be pending.
    assert!(has_open_roll(&world, 1));

    world
        .systems
        .group
        .count_roll_vote(&world, 1, greeder, loot_guid, 0, crate::game::loot::RollVote::Greed)
        .await;

    assert!(!has_open_roll(&world, 1), "roll resolved after all votes");
    let item = item_state(&world, loot_guid, 0).expect("item present");
    assert!(!item.is_blocked);
    assert_eq!(item.loot_owner, Some(needer), "need beats greed");
}

#[tokio::test]
async fn test_all_pass_unblocks_item_without_winner() {
    let world = test_world();
    let loot_guid = ObjectGuid::new_creature(500, 4);
    let a = ObjectGuid::new_player(1);
    let b = ObjectGuid::new_player(2);

    add_group_player(&world, a, 1, Position { x: 0.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::GroupLoot);
    add_group_player(&world, b, 4, Position { x: 5.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::GroupLoot);
    loot_with_quality_item(&world, loot_guid, 9003, 3, vec![a, b]);

    world
        .systems
        .group
        .group_loot(&world, 1, loot_guid, roll_context())
        .await;

    world
        .systems
        .group
        .count_roll_vote(&world, 1, a, loot_guid, 0, crate::game::loot::RollVote::Pass)
        .await;
    world
        .systems
        .group
        .count_roll_vote(&world, 1, b, loot_guid, 0, crate::game::loot::RollVote::Pass)
        .await;

    assert!(!has_open_roll(&world, 1));
    let item = item_state(&world, loot_guid, 0).expect("item present");
    assert!(!item.is_blocked, "all-pass frees the item for manual looting");
    assert!(!item.is_looted, "no winner was awarded");
    assert_eq!(item.loot_owner, None);
}

#[tokio::test]
async fn test_round_robin_looter_skips_out_of_range_members() {
    let world = test_world();
    let loot_guid = ObjectGuid::new_creature(500, 5);
    let leader = ObjectGuid::new_player(1);
    let far = ObjectGuid::new_player(2);
    let near = ObjectGuid::new_player(3);

    // Default group_xp_distance is 74 yards. "far" is 100 yards out, "near" 30.
    add_group_player(&world, leader, 1, Position { x: 0.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::RoundRobin);
    add_group_player(&world, far, 4, Position { x: 100.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::RoundRobin);
    add_group_player(&world, near, 4, Position { x: 30.0, y: 0.0, z: 0.0, o: 0.0 }, LootMethod::RoundRobin);

    let context = roll_context();
    let group = &world.systems.group;

    // First pick starts at the leader and lands on the first in-range member (skips "far").
    group.update_looter_guid(&world, 1, loot_guid, &context, false);
    assert_eq!(group.current_looter(1), Some(near));

    // Next pick advances past "near" and wraps around to the leader (also in range).
    group.update_looter_guid(&world, 1, loot_guid, &context, false);
    assert_eq!(group.current_looter(1), Some(leader));

    // And so on: full cycle of the two in-range members.
    group.update_looter_guid(&world, 1, loot_guid, &context, false);
    assert_eq!(group.current_looter(1), Some(near));
}
