//! Group system unit tests

use super::types::*;
use crate::shared::protocol::ObjectGuid;

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
    let new_leader = group.select_new_leader();
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

// ========== INTEGRATION TEST CASES FOR BUG FIXES ==========
//
// These test cases should be run as integration tests with full system setup.
// They verify the fixes for bugs #2, #3, and #4.

/// TEST CASE: Bug #1 - Loot threshold persists (ALREADY WORKING)
///
/// Steps:
/// 1. Create group with 2+ players
/// 2. Leader sets loot threshold to Rare (3)
/// 3. Verify database updated (SELECT loot_threshold FROM groups WHERE id = ?)
/// 4. All players relog
/// 5. Verify group list shows loot_threshold = 3 for all members
///
/// Expected: Loot threshold persists across relogs
#[allow(dead_code)]
fn test_loot_threshold_persists() {
    // Integration test - requires database and full system
}

/// TEST CASE: Bug #2/#3 - Group visible after relog with correct online status
///
/// Steps:
/// 1. Create group with Player A (leader) and Player B
/// 2. Player A logs out
/// 3. Verify Player B sees Player A as offline in group UI
/// 4. Player A logs back in
/// 5. Verify Player A sees group UI on login (bug #3 fix)
/// 6. Verify Player B sees Player A as online (bug #2 fix)
///
/// Expected:
/// - on_player_login_async broadcasts group list to all members
/// - Logging player sees their group
/// - Other members see updated online status
#[allow(dead_code)]
fn test_group_relog_shows_group() {
    // Integration test - requires full system with async login hooks
}

/// TEST CASE: Bug #2/#3 - Member online status updates correctly
///
/// Steps:
/// 1. Create group with 3 players (A, B, C)
/// 2. Player B logs out
/// 3. Verify A and C see B as offline
/// 4. Player B logs back in
/// 5. Verify A and C receive SMSG_GROUP_LIST update
/// 6. Verify B's status changes from offline to online in A's and C's UI
///
/// Expected: broadcast_group_list() sends updates to all members
#[allow(dead_code)]
fn test_group_member_online_status_updates() {
    // Integration test - requires broadcast manager and player manager
}

/// TEST CASE: Bug #4 - Convert to raid (ALREADY WORKING)
///
/// Steps:
/// 1. Create 5-player group
/// 2. Leader converts to raid
/// 3. Verify database updated (SELECT is_raid FROM groups WHERE id = ?)
/// 4. Verify all members receive SMSG_GROUP_LIST with is_raid = true
/// 5. Verify UI shows raid frames instead of party frames
///
/// Expected: Raid conversion works correctly
#[allow(dead_code)]
fn test_convert_to_raid_succeeds() {
    // Integration test - requires full system
}
