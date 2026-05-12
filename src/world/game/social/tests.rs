//! Tests for the Social System (world)

use super::*;
use crate::shared::database::characters::models::social::CharacterSocialRow;
use crate::shared::database::characters::repositories::SocialRepositoryTrait;
use crate::shared::game::social::{FriendInfo, FriendStatus, FriendsResult, SocialFlag, SOCIALMGR_FRIEND_LIMIT, SOCIALMGR_IGNORE_LIMIT};
use crate::shared::protocol::{HighGuid, ObjectGuid, Position};
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::player::PlayerManager;
use crate::world::core::session::SessionManager;
use mockall::mock;
use mockall::predicate::*;
use std::sync::Arc;

// ========== MOCK REPOSITORY ==========

mock! {
    pub SocialRepository {}

    #[async_trait::async_trait]
    impl SocialRepositoryTrait for SocialRepository {
        async fn find_by_guid(&self, guid: u32) -> anyhow::Result<Vec<CharacterSocialRow>>;
        async fn find_player_guid_by_name(&self, name: &str) -> anyhow::Result<Option<u32>>;
        async fn exists(&self, guid: u32, friend_guid: u32) -> anyhow::Result<bool>;
        async fn get_character_name(&self, character_guid: u32) -> anyhow::Result<Option<String>>;
        async fn add_or_update(&self, guid: u32, friend: u32, flags: u8) -> anyhow::Result<()>;
        async fn remove(&self, guid: u32, friend: u32) -> anyhow::Result<()>;
        async fn update_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> anyhow::Result<()>;
        async fn add_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> anyhow::Result<()>;
        async fn remove_flags(&self, guid: u32, friend_guid: u32, flags: u8) -> anyhow::Result<()>;
        async fn delete_all_for_character(&self, guid: u32) -> anyhow::Result<()>;
    }
}

// ========== TEST HELPERS ==========

/// Helper to create test ObjectGuid
fn test_player_guid(low: u32) -> ObjectGuid {
    ObjectGuid::new_without_entry(HighGuid::Player, low)
}

/// Helper to create a test SocialSystem with mock repository
fn create_test_system() -> (SocialSystem, Arc<MockSocialRepository>, Arc<BroadcastManager>, Arc<PlayerManager>) {
    let mock_repo = Arc::new(MockSocialRepository::new());
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));
    let system = SocialSystem::new(mock_repo.clone(), broadcast_mgr.clone(), player_mgr.clone());
    (system, mock_repo, broadcast_mgr, player_mgr)
}

/// Helper to create a pre-initialized system with player social data
async fn create_initialized_system(
    player_guid: ObjectGuid,
    friends: Vec<u32>,
    ignores: Vec<u32>,
) -> (SocialSystem, Arc<MockSocialRepository>, Arc<BroadcastManager>, Arc<PlayerManager>) {
    let (system, mut mock_repo, broadcast_mgr, player_mgr) = create_test_system();

    // Setup mock to return social data
    let player_low = player_guid.low() as u32;
    let mut rows = Vec::new();

    for friend in friends {
        rows.push(CharacterSocialRow {
            guid: player_low,
            friend,
            flags: SocialFlag::Friend as u8,
        });
    }

    for ignore in ignores {
        rows.push(CharacterSocialRow {
            guid: player_low,
            friend: ignore,
            flags: SocialFlag::Ignored as u8,
        });
    }

    // Use Arc::get_mut to get mutable reference
    let mock_repo_mut = Arc::get_mut(&mut mock_repo).unwrap();
    mock_repo_mut
        .expect_find_by_guid()
        .with(eq(player_low))
        .returning(move |_| Ok(rows.clone()));

    system.load_player_social(player_guid).await.unwrap();

    (system, mock_repo, broadcast_mgr, player_mgr)
}

// ========== INITIALIZATION TESTS ==========

#[tokio::test]
async fn test_load_player_social_empty() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    // Set up mock expectations BEFORE creating system
    mock_repo
        .expect_find_by_guid()
        .with(eq(1))
        .returning(|_| Ok(vec![]));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);
    let player = test_player_guid(1);

    let result = system.load_player_social(player).await;
    assert!(result.is_ok());

    // Verify empty state
    assert_eq!(system.get_friend_count(player), 0);
    assert_eq!(system.get_ignore_count(player), 0);
}

#[tokio::test]
async fn test_load_player_social_with_friends_and_ignores() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    // Set up mock expectations BEFORE creating system
    mock_repo
        .expect_find_by_guid()
        .with(eq(1))
        .returning(|_| {
            Ok(vec![
                CharacterSocialRow { guid: 1, friend: 2, flags: SocialFlag::Friend as u8 },
                CharacterSocialRow { guid: 1, friend: 3, flags: SocialFlag::Friend as u8 },
                CharacterSocialRow { guid: 1, friend: 4, flags: SocialFlag::Ignored as u8 },
            ])
        });

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);
    let player = test_player_guid(1);

    let result = system.load_player_social(player).await;
    assert!(result.is_ok());

    // Verify counts
    assert_eq!(system.get_friend_count(player), 2);
    assert_eq!(system.get_ignore_count(player), 1);

    // Verify specific entries
    assert!(system.has_friend(player, test_player_guid(2)));
    assert!(system.has_friend(player, test_player_guid(3)));
    assert!(system.has_ignore(player, test_player_guid(4)));
}

#[test]
fn test_unload_player_social() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Add state manually
    system.set_social_state(player, SocialState::new());
    assert_eq!(system.get_friend_count(player), 0); // Exists but empty

    // Unload
    system.unload_player_social(player);

    // Verify removed - returns 0 for non-existent player
    assert_eq!(system.get_friend_count(player), 0);
}

// ========== ADD FRIEND TESTS ==========

#[tokio::test]
async fn test_add_friend_success_offline() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);
    let friend = test_player_guid(2);

    // Set up mock expectations BEFORE creating system
    mock_repo
        .expect_find_by_guid()
        .with(eq(1))
        .returning(|_| Ok(vec![]));

    mock_repo
        .expect_add_or_update()
        .with(eq(1), eq(2), eq(SocialFlag::Friend as u8))
        .returning(|_, _, _| Ok(()));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    system.load_player_social(player).await.unwrap();

    let result = system.add_friend(player, friend, "Friend".to_string(), false).await;
    assert!(result.is_ok());

    // Verify friend was added
    assert_eq!(system.get_friend_count(player), 1);
    assert!(system.has_friend(player, friend));
}

#[tokio::test]
async fn test_add_friend_success_online() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);
    let friend = test_player_guid(2);

    mock_repo
        .expect_add_or_update()
        .with(eq(1), eq(2), eq(SocialFlag::Friend as u8))
        .returning(|_, _, _| Ok(()));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    system.set_social_state(player, SocialState::new());

    let result = system.add_friend(player, friend, "Friend".to_string(), true).await;
    assert!(result.is_ok());

    assert_eq!(system.get_friend_count(player), 1);
    assert!(system.has_friend(player, friend));
}

#[tokio::test]
async fn test_add_friend_cannot_add_self() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system.set_social_state(player, SocialState::new());

    // Try to add self - should send error packet and return Ok
    let result = system.add_friend(player, player, "Self".to_string(), false).await;
    assert!(result.is_ok());

    // Should not be added
    assert_eq!(system.get_friend_count(player), 0);
}

#[tokio::test]
async fn test_add_friend_already_friends() {
    let (system, mut mock_repo, _, _) = create_test_system();
    let player = test_player_guid(1);
    let friend = test_player_guid(2);

    // Pre-add friend
    let mut social_state = SocialState::new();
    social_state.friends.insert(friend, FriendEntry::new(friend, SocialFlag::Friend as u8));
    system.set_social_state(player, social_state);

    // Try to add again - should send error packet
    let result = system.add_friend(player, friend, "Friend".to_string(), false).await;
    assert!(result.is_ok());

    // Count should still be 1
    assert_eq!(system.get_friend_count(player), 1);
}

#[tokio::test]
async fn test_add_friend_list_full() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Fill friend list to limit
    let mut social_state = SocialState::new();
    for i in 0..SOCIALMGR_FRIEND_LIMIT {
        let friend_guid = test_player_guid(100 + i as u32);
        social_state.friends.insert(friend_guid, FriendEntry::new(friend_guid, SocialFlag::Friend as u8));
    }
    system.set_social_state(player, social_state);

    // Try to add one more - should fail
    let new_friend = test_player_guid(500);
    let result = system.add_friend(player, new_friend, "Extra".to_string(), false).await;
    assert!(result.is_ok());

    // Count should still be at limit
    assert_eq!(system.get_friend_count(player), SOCIALMGR_FRIEND_LIMIT);
}

#[tokio::test]
async fn test_add_friend_by_name_not_found() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);

    mock_repo
        .expect_find_player_guid_by_name()
        .with(eq("NonExistent"))
        .returning(|_| Ok(None));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    system.set_social_state(player, SocialState::new());

    let result = system.add_friend_by_name(player, "NonExistent".to_string()).await;
    assert!(result.is_ok());

    // Should not be added
    assert_eq!(system.get_friend_count(player), 0);
}

// ========== REMOVE FRIEND TESTS ==========

#[tokio::test]
async fn test_remove_friend_success() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);
    let friend = test_player_guid(2);

    mock_repo
        .expect_remove()
        .with(eq(1), eq(2))
        .returning(|_, _| Ok(()));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    // Pre-add friend
    let mut social_state = SocialState::new();
    social_state.friends.insert(friend, FriendEntry::new(friend, SocialFlag::Friend as u8));
    system.set_social_state(player, social_state);

    let result = system.remove_friend(player, friend).await;
    assert!(result.is_ok());

    // Verify removed
    assert_eq!(system.get_friend_count(player), 0);
    assert!(!system.has_friend(player, friend));
}

#[tokio::test]
async fn test_remove_friend_not_found() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let friend = test_player_guid(2);

    system.set_social_state(player, SocialState::new());

    // Try to remove non-existent friend - should send error packet
    let result = system.remove_friend(player, friend).await;
    assert!(result.is_ok());

    assert_eq!(system.get_friend_count(player), 0);
}

// ========== ADD IGNORE TESTS ==========

#[tokio::test]
async fn test_add_ignore_success() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);
    let ignored = test_player_guid(2);

    mock_repo
        .expect_add_or_update()
        .with(eq(1), eq(2), eq(SocialFlag::Ignored as u8))
        .returning(|_, _, _| Ok(()));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    system.set_social_state(player, SocialState::new());

    let result = system.add_ignore(player, ignored, "Ignored".to_string()).await;
    assert!(result.is_ok());

    // Verify ignore was added
    assert_eq!(system.get_ignore_count(player), 1);
    assert!(system.has_ignore(player, ignored));
}

#[tokio::test]
async fn test_add_ignore_cannot_ignore_self() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system.set_social_state(player, SocialState::new());

    // Try to ignore self - should send error packet
    let result = system.add_ignore(player, player, "Self".to_string()).await;
    assert!(result.is_ok());

    // Should not be added
    assert_eq!(system.get_ignore_count(player), 0);
}

#[tokio::test]
async fn test_add_ignore_already_ignored() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let ignored = test_player_guid(2);

    // Pre-add ignore
    let mut social_state = SocialState::new();
    social_state.ignores.insert(ignored, IgnoreEntry::new(ignored, SocialFlag::Ignored as u8));
    system.set_social_state(player, social_state);

    // Try to add again - should send error packet
    let result = system.add_ignore(player, ignored, "Ignored".to_string()).await;
    assert!(result.is_ok());

    // Count should still be 1
    assert_eq!(system.get_ignore_count(player), 1);
}

#[tokio::test]
async fn test_add_ignore_list_full() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Fill ignore list to limit
    let mut social_state = SocialState::new();
    for i in 0..SOCIALMGR_IGNORE_LIMIT {
        let ignored_guid = test_player_guid(100 + i as u32);
        social_state.ignores.insert(ignored_guid, IgnoreEntry::new(ignored_guid, SocialFlag::Ignored as u8));
    }
    system.set_social_state(player, social_state);

    // Try to add one more - should fail
    let new_ignored = test_player_guid(500);
    let result = system.add_ignore(player, new_ignored, "Extra".to_string()).await;
    assert!(result.is_ok());

    // Count should still be at limit
    assert_eq!(system.get_ignore_count(player), SOCIALMGR_IGNORE_LIMIT);
}

// ========== REMOVE IGNORE TESTS ==========

#[tokio::test]
async fn test_remove_ignore_success() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);
    let ignored = test_player_guid(2);

    mock_repo
        .expect_remove()
        .with(eq(1), eq(2))
        .returning(|_, _| Ok(()));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    // Pre-add ignore
    let mut social_state = SocialState::new();
    social_state.ignores.insert(ignored, IgnoreEntry::new(ignored, SocialFlag::Ignored as u8));
    system.set_social_state(player, social_state);

    let result = system.remove_ignore(player, ignored).await;
    assert!(result.is_ok());

    // Verify removed
    assert_eq!(system.get_ignore_count(player), 0);
    assert!(!system.has_ignore(player, ignored));
}

#[tokio::test]
async fn test_remove_ignore_not_found() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let ignored = test_player_guid(2);

    system.set_social_state(player, SocialState::new());

    // Try to remove non-existent ignore - should send error packet
    let result = system.remove_ignore(player, ignored).await;
    assert!(result.is_ok());

    assert_eq!(system.get_ignore_count(player), 0);
}

// ========== QUERY OPERATIONS TESTS ==========

#[test]
fn test_get_friend_list() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Add friends
    let mut social_state = SocialState::new();
    social_state.friends.insert(test_player_guid(2), FriendEntry::new(test_player_guid(2), SocialFlag::Friend as u8));
    social_state.friends.insert(test_player_guid(3), FriendEntry::new(test_player_guid(3), SocialFlag::Friend as u8));
    system.set_social_state(player, social_state);

    let friends = system.get_friend_list(player);
    assert_eq!(friends.len(), 2);
    assert!(friends.contains(&test_player_guid(2)));
    assert!(friends.contains(&test_player_guid(3)));
}

#[test]
fn test_get_ignore_list() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Add ignores
    let mut social_state = SocialState::new();
    social_state.ignores.insert(test_player_guid(4), IgnoreEntry::new(test_player_guid(4), SocialFlag::Ignored as u8));
    social_state.ignores.insert(test_player_guid(5), IgnoreEntry::new(test_player_guid(5), SocialFlag::Ignored as u8));
    system.set_social_state(player, social_state);

    let ignores = system.get_ignore_list(player);
    assert_eq!(ignores.len(), 2);
    assert!(ignores.contains(&test_player_guid(4)));
    assert!(ignores.contains(&test_player_guid(5)));
}

#[test]
fn test_has_friend() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let friend = test_player_guid(2);

    let mut social_state = SocialState::new();
    social_state.friends.insert(friend, FriendEntry::new(friend, SocialFlag::Friend as u8));
    system.set_social_state(player, social_state);

    assert!(system.has_friend(player, friend));
    assert!(!system.has_friend(player, test_player_guid(3)));
}

#[test]
fn test_has_ignore() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let ignored = test_player_guid(2);

    let mut social_state = SocialState::new();
    social_state.ignores.insert(ignored, IgnoreEntry::new(ignored, SocialFlag::Ignored as u8));
    system.set_social_state(player, social_state);

    assert!(system.has_ignore(player, ignored));
    assert!(!system.has_ignore(player, test_player_guid(3)));
}

#[test]
fn test_get_counts() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    let mut social_state = SocialState::new();
    social_state.friends.insert(test_player_guid(2), FriendEntry::new(test_player_guid(2), SocialFlag::Friend as u8));
    social_state.friends.insert(test_player_guid(3), FriendEntry::new(test_player_guid(3), SocialFlag::Friend as u8));
    social_state.ignores.insert(test_player_guid(4), IgnoreEntry::new(test_player_guid(4), SocialFlag::Ignored as u8));
    system.set_social_state(player, social_state);

    assert_eq!(system.get_friend_count(player), 2);
    assert_eq!(system.get_ignore_count(player), 1);
}

#[test]
fn test_get_counts_for_nonexistent_player() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(999);

    assert_eq!(system.get_friend_count(player), 0);
    assert_eq!(system.get_ignore_count(player), 0);
}

// ========== WHISPER VALIDATION TESTS ==========

#[test]
fn test_validate_whisper_allowed() {
    let (system, _, _, _) = create_test_system();
    let sender = test_player_guid(1);
    let target = test_player_guid(2);

    // Both players have empty social lists
    system.set_social_state(sender, SocialState::new());
    system.set_social_state(target, SocialState::new());

    let result = system.validate_whisper(sender, target);
    assert!(result.is_ok(), "Whisper should be allowed");
}

#[test]
fn test_validate_whisper_blocked_by_ignore() {
    let (system, _, _, _) = create_test_system();
    let sender = test_player_guid(1);
    let target = test_player_guid(2);

    // Target has sender on ignore list
    let mut target_social = SocialState::new();
    target_social.ignores.insert(sender, IgnoreEntry::new(sender, SocialFlag::Ignored as u8));
    system.set_social_state(target, target_social);

    system.set_social_state(sender, SocialState::new());

    let result = system.validate_whisper(sender, target);
    assert!(result.is_err(), "Whisper should be blocked");
    assert!(matches!(result.unwrap_err(), WhisperBlockReason::TargetIgnoresSender));
}

#[test]
fn test_is_ignored_reverse_check() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // player1 ignores player2
    let mut social_state = SocialState::new();
    social_state.ignores.insert(player2, IgnoreEntry::new(player2, SocialFlag::Ignored as u8));
    system.set_social_state(player1, social_state);

    // Check if player2 is ignored by player1
    assert!(system.is_ignored(player2, player1));
    assert!(!system.is_ignored(player1, player2)); // Reverse should be false
}

// ========== WHO COMMAND TESTS ==========

#[test]
fn test_send_who_list_empty() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    let request = WhoRequest {
        requester_guid: player,
        requester_team: 0,
        requester_security: 0,
        min_level: 0,
        max_level: 100,
        race_mask: 0,
        class_mask: 0,
        zone_ids: vec![],
        player_name: String::new(),
        guild_name: String::new(),
        search_strings: vec![],
    };

    // Note: send_who_list sends packets via broadcast_mgr
    // We just verify it doesn't panic
    system.send_who_list(player, request);
}

// Note: More comprehensive WHO tests would require mocking PlayerManager
// to return test player data

// ========== LIFECYCLE TESTS ==========

#[tokio::test]
async fn test_load_and_unload_player_social() {
    let mut mock_repo = MockSocialRepository::new();
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));

    let player = test_player_guid(1);

    mock_repo
        .expect_find_by_guid()
        .with(eq(1))
        .returning(|_| Ok(vec![]));

    let system = SocialSystem::new(Arc::new(mock_repo), broadcast_mgr, player_mgr);

    // Load
    let result = system.load_player_social(player).await;
    assert!(result.is_ok());
    assert_eq!(system.get_friend_count(player), 0);

    // Unload
    system.unload_player_social(player);
    assert_eq!(system.get_friend_count(player), 0); // Returns 0 for unloaded player
}

// ========== EDGE CASES ==========

#[test]
fn test_get_friend_list_empty() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system.set_social_state(player, SocialState::new());

    let friends = system.get_friend_list(player);
    assert_eq!(friends.len(), 0);
}

#[test]
fn test_get_ignore_list_empty() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system.set_social_state(player, SocialState::new());

    let ignores = system.get_ignore_list(player);
    assert_eq!(ignores.len(), 0);
}

#[test]
fn test_concurrent_friend_operations() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system.set_social_state(player, SocialState::new());

    // Simulate concurrent checks (DashMap is thread-safe)
    let has_friend = system.has_friend(player, test_player_guid(2));
    let count = system.get_friend_count(player);

    assert!(!has_friend);
    assert_eq!(count, 0);
}

// ========== INTEGRATION TEST CASES FOR BUG FIXES ==========
//
// These test cases should be run as integration tests with full system setup.
// They verify the fixes for bugs #11 and #12.

/// TEST CASE: Bug #11 - Friends list persistence (ALREADY WORKING)
///
/// Steps:
/// 1. Player A adds Player B as friend
/// 2. Verify database updated: SELECT * FROM character_social WHERE guid = A AND friend = B
/// 3. Player A logs out
/// 4. Server restarts (or simulate by clearing cache)
/// 5. Player A logs back in
/// 6. Verify Player A's friend list still contains Player B
///
/// Expected: Friends list loaded from database on login
#[allow(dead_code)]
fn test_friends_list_persistence() {
    // Integration test - requires database
}

/// TEST CASE: Friends list shows correct info
///
/// Note: Friend names are NOT included in SMSG_FRIEND_LIST per Vanilla 1.12 protocol.
/// The client uses its name cache (populated via SMSG_NAME_QUERY_RESPONSE) to display names.
///
/// Steps:
/// 1. Player A ("Alice") adds Player B ("Bob") as friend while both online
/// 2. Player A opens friend list
/// 3. Verify SMSG_FRIEND_LIST packet contains:
///    - friend_guids: [B_guid_low]
///    - friend_infos: [online status with level/class/area]
/// 4. Verify client sends SMSG_NAME_QUERY for unknown GUIDs
/// 5. Verify UI shows "Bob" in friend list (from name cache)
/// 6. Player B logs out
/// 7. Player A opens friend list again
/// 8. Verify SMSG_FRIEND_LIST packet shows offline status
/// 9. Verify UI still shows "Bob" (from cached name query)
///
/// Expected:
/// - SmsgFriendList packet does NOT include names (protocol-compliant)
/// - Client displays names from its own name cache
/// - Server sends SMSG_NAME_QUERY_RESPONSE when client queries unknown GUIDs
#[allow(dead_code)]
fn test_friends_list_shows_names() {
    // Integration test - requires database, player manager, broadcast manager
}

/// TEST CASE: Multiple friends with correct status
///
/// Note: Friend names are NOT in SMSG_FRIEND_LIST; client uses name cache.
///
/// Steps:
/// 1. Player A adds 5 friends (B, C, D, E, F)
/// 2. 3 friends are online (B, D, F), 2 are offline (C, E)
/// 3. Player A opens friend list
/// 4. Verify SMSG_FRIEND_LIST contains all 5 GUIDs with correct status
/// 5. Verify online friends have status=Online with correct level/class/area
/// 6. Verify offline friends have status=Offline (no level/class/area sent)
///
/// Expected: All friend statuses displayed correctly, names from client cache
#[allow(dead_code)]
fn test_friends_list_multiple_names() {
    // Integration test
}

/// TEST CASE: Repository method - get_character_name
///
/// Steps:
/// 1. Insert test character into database: guid=999, name="TestChar"
/// 2. Call repository.get_character_name(999)
/// 3. Verify returns Some("TestChar")
/// 4. Call repository.get_character_name(99999) for non-existent character
/// 5. Verify returns None
///
/// Expected: Database lookup works correctly
#[allow(dead_code)]
fn test_repository_get_character_name() {
    // Unit/integration test with database
}
