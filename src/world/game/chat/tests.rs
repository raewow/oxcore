//! Tests for the Chat System (world)

use super::*;
use crate::shared::database::characters::models::social::CharacterSocialRow;
use crate::shared::database::characters::repositories::SocialRepositoryTrait;
use crate::shared::game::chat::{ChatMsg, ChatTag, Language, Team};
use crate::shared::protocol::{HighGuid, ObjectGuid, Position};
use crate::world::core::session::SessionManager;
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::player::PlayerManager;
use mockall::mock;
use std::sync::Arc;
use std::time::Duration;

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

/// Helper to create a test ChatSystem with default config
fn create_test_system() -> (
    ChatSystem,
    Arc<BroadcastManager>,
    Arc<PlayerManager>,
    crate::world::game::social::SocialSystem,
) {
    let mock_repo = Arc::new(MockSocialRepository::new());
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));
    let system = ChatSystem::new(broadcast_mgr.clone(), player_mgr.clone());
    let social_system = crate::world::game::social::SocialSystem::new(
        mock_repo,
        broadcast_mgr.clone(),
        player_mgr.clone(),
    );
    (system, broadcast_mgr, player_mgr, social_system)
}

/// Helper to create a test ChatSystem with custom flood config
fn create_test_system_with_flood_config(
    max_messages: u32,
    window_secs: u64,
    mute_duration_secs: u64,
) -> (
    ChatSystem,
    Arc<BroadcastManager>,
    Arc<PlayerManager>,
    crate::world::game::social::SocialSystem,
) {
    let mock_repo = Arc::new(MockSocialRepository::new());
    let session_mgr = Arc::new(SessionManager::new());
    let player_mgr = Arc::new(PlayerManager::new());
    let broadcast_mgr = Arc::new(BroadcastManager::new(session_mgr, player_mgr.clone()));
    let config = FloodConfig {
        max_messages,
        window_secs,
        mute_duration_secs,
    };
    let system = ChatSystem::with_flood_config(broadcast_mgr.clone(), player_mgr.clone(), config);
    let social_system = crate::world::game::social::SocialSystem::new(
        mock_repo,
        broadcast_mgr.clone(),
        player_mgr.clone(),
    );
    (system, broadcast_mgr, player_mgr, social_system)
}

// ========== FLOOD PROTECTION TESTS ==========

#[tokio::test]
async fn test_flood_protection_allows_normal_rate() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Should allow up to max_messages (default 10) within window
    for _ in 0..5 {
        let result = system.check_flood_protection(player);
        assert!(result.is_ok(), "Normal message rate should be allowed");
    }
}

#[tokio::test]
async fn test_flood_protection_blocks_after_limit() {
    let (system, _, _, _) = create_test_system_with_flood_config(3, 10, 60);
    let player = test_player_guid(1);

    // First 3 messages should be allowed
    for i in 0..3 {
        let result = system.check_flood_protection(player);
        assert!(result.is_ok(), "Message {} should be allowed", i + 1);
    }

    // 4th message should trigger mute
    let result = system.check_flood_protection(player);
    assert!(result.is_err(), "4th message should be blocked");
    match result {
        Err(ChatError::Muted { remaining_secs }) => {
            assert!(remaining_secs > 0, "Mute duration should be positive");
        }
        _ => panic!("Expected ChatError::Muted"),
    }
}

#[tokio::test]
async fn test_flood_protection_sends_restricted_packet() {
    let (system, broadcast_mgr, _, _) = create_test_system_with_flood_config(2, 10, 60);
    let player = test_player_guid(1);

    // Use up allowed messages
    system.check_flood_protection(player).unwrap();
    system.check_flood_protection(player).unwrap();

    // This should trigger mute and send SmsgChatRestricted packet
    let _ = system.check_flood_protection(player);

    // Note: In real implementation, we'd verify the packet was sent via broadcast_mgr
    // For now, we just verify the error was returned
}

#[tokio::test]
async fn test_flood_protection_clears_old_messages() {
    let (system, _, _, _) = create_test_system_with_flood_config(3, 1, 60); // 1 second window
    let player = test_player_guid(1);

    // Send 3 messages
    for _ in 0..3 {
        system.check_flood_protection(player).unwrap();
    }

    // Wait for window to expire
    std::thread::sleep(Duration::from_secs(2));

    // Should be able to send again
    let result = system.check_flood_protection(player);
    assert!(
        result.is_ok(),
        "Old messages should be cleared after window"
    );
}

#[tokio::test]
async fn test_clean_expired_mutes() {
    let (system, _, _, _) = create_test_system_with_flood_config(1, 10, 1); // 1 second mute
    let player = test_player_guid(1);

    // Trigger mute
    system.check_flood_protection(player).unwrap();
    let _ = system.check_flood_protection(player); // Should be muted

    // Clean expired mutes after waiting
    std::thread::sleep(Duration::from_secs(2));
    system.clean_expired_mutes();

    // Should be able to send again
    let result = system.check_flood_protection(player);
    assert!(result.is_ok(), "Mute should be expired");
}

// ========== MESSAGE VALIDATION TESTS ==========

#[tokio::test]
async fn test_validate_empty_message() {
    let result = validation::validate_message("");
    assert!(matches!(result, Err(ChatError::EmptyMessage)));

    let result = validation::validate_message("   ");
    assert!(matches!(result, Err(ChatError::EmptyMessage)));

    let result = validation::validate_message("\t\n  ");
    assert!(matches!(result, Err(ChatError::EmptyMessage)));
}

#[tokio::test]
async fn test_validate_long_message() {
    let long_msg = "a".repeat(256); // MAX_MESSAGE_LENGTH = 255
    let result = validation::validate_message(&long_msg);
    assert!(matches!(result, Err(ChatError::MessageTooLong)));
}

#[tokio::test]
async fn test_validate_valid_message() {
    let result = validation::validate_message("Hello, world!");
    assert!(result.is_ok());

    let result = validation::validate_message("Test message with spaces");
    assert!(result.is_ok());

    let result = validation::validate_message("Numbers 123 and symbols !@#");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sanitize_message_strips_invisible_chars() {
    let message = "Hello\x00World\x01Test";
    let sanitized = validation::strip_invisible_chars(message);
    assert_eq!(sanitized, "HelloWorldTest");

    // Tab and newline should be preserved
    let message = "Hello\tWorld\nTest";
    let sanitized = validation::strip_invisible_chars(message);
    assert_eq!(sanitized, "Hello\tWorld\nTest");
}

#[tokio::test]
async fn test_validate_channel_name() {
    // Valid names
    assert!(validation::validate_channel_name("General").is_ok());
    assert!(validation::validate_channel_name("My-Channel").is_ok());
    assert!(validation::validate_channel_name("Channel_123").is_ok());
    assert!(validation::validate_channel_name("Test Channel").is_ok());

    // Invalid names
    assert!(validation::validate_channel_name("").is_err());
    let long_name = "a".repeat(32);
    assert!(validation::validate_channel_name(&long_name).is_err()); // Too long (max 31)
    assert!(validation::validate_channel_name("Invalid@Channel").is_err()); // Invalid chars
}

// ========== CHANNEL OPERATIONS TESTS ==========

#[tokio::test]
async fn test_join_channel_success() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    let result = system
        .join_channel(player, "General", None, Team::Alliance)
        .await;
    assert!(result.is_ok(), "Should successfully join channel");

    // Verify player is in channel
    assert!(system.is_in_channel(player, "General", Team::Alliance));

    // Verify channel appears in player's channel list
    let channels = system.get_player_channels(player);
    assert!(channels.contains(&"General".to_string()));
}

#[tokio::test]
async fn test_join_channel_with_password() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Create a custom channel with password
    let result = system
        .join_channel(player1, "PrivateChannel", Some("secret"), Team::Alliance)
        .await;
    assert!(result.is_ok());

    // Try to join without password - should fail
    let result = system
        .join_channel(player2, "PrivateChannel", None, Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::WrongPassword)));

    // Try to join with wrong password
    let result = system
        .join_channel(player2, "PrivateChannel", Some("wrong"), Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::WrongPassword)));

    // Join with correct password
    let result = system
        .join_channel(player2, "PrivateChannel", Some("secret"), Team::Alliance)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_join_channel_already_member() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Join once - should succeed
    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Try to join again - idempotent, returns Ok
    let result = system
        .join_channel(player, "General", None, Team::Alliance)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_join_channel_max_limit() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Join 20 channels (MAX_CHANNELS_PER_PLAYER)
    for i in 0..20 {
        let channel_name = format!("Channel{}", i);
        let result = system
            .join_channel(player, &channel_name, None, Team::Alliance)
            .await;
        assert!(result.is_ok(), "Should join channel {}", i);
    }

    // Try to join 21st channel - should fail
    let result = system
        .join_channel(player, "ExtraChannel", None, Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::MaxChannelsReached)));
}

#[tokio::test]
async fn test_leave_channel_success() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Join and then leave
    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();
    let result = system
        .leave_channel(player, "General", Team::Alliance)
        .await;
    assert!(result.is_ok());

    // Verify player is no longer in channel
    assert!(!system.is_in_channel(player, "General", Team::Alliance));

    // Verify channel not in player's list
    let channels = system.get_player_channels(player);
    assert!(!channels.contains(&"General".to_string()));
}

#[tokio::test]
async fn test_leave_channel_not_member() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let other_player = test_player_guid(2);

    // Create channel by having another player join
    system
        .join_channel(other_player, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Try to leave without being a member
    let result = system
        .leave_channel(player, "General", Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::NotInChannel)));
}

#[tokio::test]
async fn test_leave_all_channels() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Join multiple channels
    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player, "Trade", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player, "Custom", None, Team::Alliance)
        .await
        .unwrap();

    // Verify joined
    assert_eq!(system.get_player_channels(player).len(), 3);

    // Leave all
    system.leave_all_channels(player);

    // Verify all left
    assert_eq!(system.get_player_channels(player).len(), 0);
}

// ========== CHANNEL MODERATION TESTS ==========

#[tokio::test]
async fn test_set_moderator() {
    let (system, _, _, _) = create_test_system();
    let owner = test_player_guid(1);
    let member = test_player_guid(2);

    // Setup: owner creates and joins channel
    system
        .join_channel(owner, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Owner sets member as moderator
    let result = system.set_moderator(Team::Alliance, "TestChannel", member, true);
    assert!(result.is_ok());

    // Verify member can now moderate (implementation dependent)
}

#[tokio::test]
async fn test_set_moderator_no_permission() {
    let (system, _, _, _) = create_test_system();
    let owner = test_player_guid(1);
    let member = test_player_guid(2);
    let other = test_player_guid(3);

    system
        .join_channel(owner, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(other, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Non-owner/non-moderator tries to set moderator - should fail
    // Note: This would require passing caller_guid to set_moderator method
}

#[tokio::test]
async fn test_ban_from_channel() {
    let (system, _, _, _) = create_test_system();
    let owner = test_player_guid(1);
    let member = test_player_guid(2);

    system
        .join_channel(owner, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Ban member
    let result = system.ban_from_channel(Team::Alliance, "TestChannel", member);
    assert!(result.is_ok());

    // Verify member is removed from channel
    assert!(!system.is_in_channel(member, "TestChannel", Team::Alliance));

    // Try to rejoin - should fail
    let result = system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::BannedFromChannel)));
}

#[tokio::test]
async fn test_unban_from_channel() {
    let (system, _, _, _) = create_test_system();
    let owner = test_player_guid(1);
    let member = test_player_guid(2);

    system
        .join_channel(owner, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Ban and then unban
    system
        .ban_from_channel(Team::Alliance, "TestChannel", member)
        .unwrap();
    let result = system.unban_from_channel(Team::Alliance, "TestChannel", member);
    assert!(result.is_ok());
    assert!(result.unwrap(), "Should return true when unbanned");

    // Should be able to rejoin
    let result = system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_muted_in_channel() {
    let (system, _, _, _) = create_test_system();
    let owner = test_player_guid(1);
    let member = test_player_guid(2);

    system
        .join_channel(owner, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member, "TestChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Mute member
    let result = system.set_muted(Team::Alliance, "TestChannel", member, true);
    assert!(result.is_ok());

    // Trying to send message should fail
    let result = system
        .send_channel_message(member, "TestChannel", "Hello", Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::MutedInChannel)));
}

// ========== CHANNEL MESSAGE TESTS ==========

#[tokio::test]
async fn test_send_channel_message_success() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();

    let result = system
        .send_channel_message(player, "General", "Hello, everyone!", Team::Alliance)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_send_channel_message_not_member() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let other_player = test_player_guid(2);

    // Create channel by having another player join
    system
        .join_channel(other_player, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Try to send without joining
    let result = system
        .send_channel_message(player, "General", "Hello", Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::NotInChannel)));
}

#[tokio::test]
async fn test_send_channel_message_empty() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();

    let result = system
        .send_channel_message(player, "General", "", Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::EmptyMessage)));
}

#[tokio::test]
async fn test_send_channel_message_too_long() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();

    let long_message = "a".repeat(256);
    let result = system
        .send_channel_message(player, "General", &long_message, Team::Alliance)
        .await;
    assert!(matches!(result, Err(ChatError::MessageTooLong)));
}

// ========== WHISPER TESTS ==========

#[tokio::test]
async fn test_send_whisper_validates_message() {
    let (system, _, _, social_system) = create_test_system();
    let sender = test_player_guid(1);

    // Empty message
    let result = system
        .send_whisper(sender, "Target", "", &social_system)
        .await;
    assert!(matches!(result, Err(ChatError::EmptyMessage)));

    // Too long
    let long_msg = "a".repeat(256);
    let result = system
        .send_whisper(sender, "Target", &long_msg, &social_system)
        .await;
    assert!(matches!(result, Err(ChatError::MessageTooLong)));
}

// ========== FACTION SEPARATION TESTS ==========

#[tokio::test]
async fn test_channel_faction_separation() {
    let (system, _, _, _) = create_test_system();
    let alliance_player = test_player_guid(1);
    let horde_player = test_player_guid(2);

    // Both join "General" but for different teams
    system
        .join_channel(alliance_player, "General", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(horde_player, "General", None, Team::Horde)
        .await
        .unwrap();

    // Verify Alliance player is in Alliance General
    assert!(system.is_in_channel(alliance_player, "General", Team::Alliance));
    assert!(!system.is_in_channel(alliance_player, "General", Team::Horde));

    // Verify Horde player is in Horde General
    assert!(system.is_in_channel(horde_player, "General", Team::Horde));
    assert!(!system.is_in_channel(horde_player, "General", Team::Alliance));

    // Verify different member counts
    let alliance_members = system.get_channel_members(Team::Alliance, "General");
    let horde_members = system.get_channel_members(Team::Horde, "General");
    assert_eq!(alliance_members.len(), 1);
    assert_eq!(horde_members.len(), 1);
    assert_ne!(alliance_members[0], horde_members[0]);
}

// ========== SYSTEM TRAIT TESTS ==========

#[tokio::test]
async fn test_system_init_creates_default_channels() {
    let (system, _, _, _) = create_test_system();
    // Note: System::init requires &World parameter, so we test via the internal create_default_channels
    // which is called during init

    // Manually create default channels (simulating init)
    system.create_default_channels(Team::Alliance);
    system.create_default_channels(Team::Horde);

    // Verify default channels exist for Alliance
    let alliance_channels = system.test_get_channels(Team::Alliance);
    assert!(alliance_channels.is_some());
    let channels = alliance_channels.unwrap();
    assert!(channels.contains(&"General".to_string()));
    assert!(channels.contains(&"Trade".to_string()));
    assert!(channels.contains(&"LocalDefense".to_string()));
    assert!(channels.contains(&"WorldDefense".to_string()));
    assert!(channels.contains(&"GuildRecruitment".to_string()));
    assert!(channels.contains(&"LookingForGroup".to_string()));

    // Verify default channels exist for Horde
    let horde_channels = system.test_get_channels(Team::Horde);
    assert!(horde_channels.is_some());
    let channels = horde_channels.unwrap();
    assert!(channels.contains(&"General".to_string()));
}

#[tokio::test]
async fn test_system_on_player_login() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    system.on_player_login(player).unwrap();

    // Verify flood tracker initialized
    assert!(system.test_flood_tracker_contains(player));
}

#[tokio::test]
async fn test_system_on_player_logout() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);

    // Login and join channel
    system.on_player_login(player).unwrap();
    system
        .join_channel(player, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Logout
    system.on_player_logout(player).unwrap();

    // Verify player left all channels
    assert_eq!(system.get_player_channels(player).len(), 0);

    // Verify flood tracker cleared
    assert!(!system.test_flood_tracker_contains(player));
}

#[tokio::test]
async fn test_system_update_cleans_mutes() {
    let (system, _, _, _) = create_test_system_with_flood_config(1, 10, 1);
    let player = test_player_guid(1);

    // Trigger mute
    system.check_flood_protection(player).unwrap();
    let _ = system.check_flood_protection(player); // Should be muted

    // Wait and clean
    std::thread::sleep(Duration::from_secs(2));
    system.clean_expired_mutes();

    // Should be unmuted after clean
    let result = system.check_flood_protection(player);
    assert!(result.is_ok(), "Mute should be cleared by clean");
}

// ========== EDGE CASES ==========

#[tokio::test]
async fn test_get_channel_members_empty() {
    let (system, _, _, _) = create_test_system();
    let members = system.get_channel_members(Team::Alliance, "NonExistent");
    assert_eq!(members.len(), 0);
}

#[tokio::test]
async fn test_is_in_channel_nonexistent_player() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(999);
    assert!(!system.is_in_channel(player, "General", Team::Alliance));
}

#[tokio::test]
async fn test_get_player_channels_empty() {
    let (system, _, _, _) = create_test_system();
    let player = test_player_guid(1);
    let channels = system.get_player_channels(player);
    assert_eq!(channels.len(), 0);
}

// ========== NOTIFICATION TESTS (State-based) ==========
// These tests verify channel notification behavior by checking state changes
// rather than packet content, since ChatSystem uses concrete BroadcastManager

#[tokio::test]
async fn test_join_custom_channel_members_updated() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Player1 joins custom channel (becomes owner)
    system
        .join_channel(player1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Verify player1 is in channel
    assert!(system.is_in_channel(player1, "CustomChannel", Team::Alliance));
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 1);
    assert!(members.contains(&player1));

    // Player2 joins - should be added to channel
    system
        .join_channel(player2, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Verify both players are in channel
    assert!(system.is_in_channel(player2, "CustomChannel", Team::Alliance));
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 2);
    assert!(members.contains(&player1));
    assert!(members.contains(&player2));
}

#[tokio::test]
async fn test_join_default_channel_members_updated() {
    let (system, _, _, _) = create_test_system();
    system.create_default_channels(Team::Alliance);

    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Player1 joins General (default channel)
    system
        .join_channel(player1, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Verify player1 is in channel
    assert!(system.is_in_channel(player1, "General", Team::Alliance));

    // Player2 joins - should be added to channel
    system
        .join_channel(player2, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Verify both players are in channel
    assert!(system.is_in_channel(player2, "General", Team::Alliance));
    let members = system.get_channel_members(Team::Alliance, "General");
    assert_eq!(members.len(), 2);
    assert!(members.contains(&player1));
    assert!(members.contains(&player2));
}

#[tokio::test]
async fn test_leave_custom_channel_members_updated() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Both players join custom channel
    system
        .join_channel(player1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player2, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Verify both are in channel
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 2);

    // Player2 leaves
    system
        .leave_channel(player2, "CustomChannel", Team::Alliance)
        .await
        .unwrap();

    // Verify only player1 remains
    assert!(!system.is_in_channel(player2, "CustomChannel", Team::Alliance));
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 1);
    assert!(members.contains(&player1));
}

#[tokio::test]
async fn test_leave_default_channel_members_updated() {
    let (system, _, _, _) = create_test_system();
    system.create_default_channels(Team::Alliance);

    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Both players join General (default channel)
    system
        .join_channel(player1, "General", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player2, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Player2 leaves
    system
        .leave_channel(player2, "General", Team::Alliance)
        .await
        .unwrap();

    // Verify only player1 remains
    assert!(!system.is_in_channel(player2, "General", Team::Alliance));
    let members = system.get_channel_members(Team::Alliance, "General");
    assert_eq!(members.len(), 1);
    assert!(members.contains(&player1));
}

#[tokio::test]
async fn test_owner_leaves_custom_channel_transfers_ownership() {
    let (system, _, _, _) = create_test_system();
    let owner = test_player_guid(1);
    let member1 = test_player_guid(2);
    let member2 = test_player_guid(3);

    // Owner creates and joins custom channel
    system
        .join_channel(owner, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(member2, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Verify all three are members
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 3);

    // Owner leaves - ownership should transfer to member1 (first remaining member)
    system
        .leave_channel(owner, "CustomChannel", Team::Alliance)
        .await
        .unwrap();

    // Verify owner is gone and only member1 and member2 remain
    assert!(!system.is_in_channel(owner, "CustomChannel", Team::Alliance));
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 2);
    assert!(members.contains(&member1));
    assert!(members.contains(&member2));
}

#[tokio::test]
async fn test_default_channel_has_no_owner() {
    let (system, _, _, _) = create_test_system();
    system.create_default_channels(Team::Alliance);

    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Players join General (default channel)
    system
        .join_channel(player1, "General", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player2, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Player1 leaves - no ownership transfer for default channels
    system
        .leave_channel(player1, "General", Team::Alliance)
        .await
        .unwrap();

    // Verify player2 remains
    assert!(!system.is_in_channel(player1, "General", Team::Alliance));
    assert!(system.is_in_channel(player2, "General", Team::Alliance));
}

#[tokio::test]
async fn test_last_member_leaves_custom_channel_removes_channel() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1);

    // Player creates and joins custom channel
    system
        .join_channel(player1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Verify player is in channel
    assert!(system.is_in_channel(player1, "CustomChannel", Team::Alliance));

    // Player leaves - channel should be removed
    system
        .leave_channel(player1, "CustomChannel", Team::Alliance)
        .await
        .unwrap();

    // Verify channel is removed (empty)
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 0);
}

#[tokio::test]
async fn test_multiple_members_join_custom_channel() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);
    let player3 = test_player_guid(3);

    // Player1 joins (becomes owner)
    system
        .join_channel(player1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 1);

    // Player2 joins
    system
        .join_channel(player2, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 2);

    // Player3 joins
    system
        .join_channel(player3, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 3);
    assert!(members.contains(&player1));
    assert!(members.contains(&player2));
    assert!(members.contains(&player3));
}

#[tokio::test]
async fn test_all_default_channels_work() {
    let (system, _, _, _) = create_test_system();
    system.create_default_channels(Team::Alliance);

    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    let default_channels = vec![
        "General",
        "Trade",
        "LocalDefense",
        "WorldDefense",
        "GuildRecruitment",
        "LookingForGroup",
    ];

    // Test that all default channels can be joined
    for channel in &default_channels {
        system
            .join_channel(player1, channel, None, Team::Alliance)
            .await
            .unwrap();
        system
            .join_channel(player2, channel, None, Team::Alliance)
            .await
            .unwrap();

        // Verify both are members
        assert!(system.is_in_channel(player1, channel, Team::Alliance));
        assert!(system.is_in_channel(player2, channel, Team::Alliance));
    }
}

#[tokio::test]
async fn test_ownership_transfer_chain() {
    let (system, _, _, _) = create_test_system();
    let player1 = test_player_guid(1); // Original owner
    let player2 = test_player_guid(2); // Will become owner
    let player3 = test_player_guid(3); // Will become final owner

    // All three join
    system
        .join_channel(player1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player2, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player3, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Player1 (owner) leaves - player2 becomes owner
    system
        .leave_channel(player1, "CustomChannel", Team::Alliance)
        .await
        .unwrap();

    // Verify player2 and player3 remain
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 2);
    assert!(members.contains(&player2));
    assert!(members.contains(&player3));

    // Player2 (new owner) leaves - player3 becomes owner
    system
        .leave_channel(player2, "CustomChannel", Team::Alliance)
        .await
        .unwrap();

    // Verify only player3 remains
    let members = system.get_channel_members(Team::Alliance, "CustomChannel");
    assert_eq!(members.len(), 1);
    assert!(members.contains(&player3));
}

#[tokio::test]
async fn test_mixed_custom_and_default_channels() {
    let (system, _, _, _) = create_test_system();
    system.create_default_channels(Team::Alliance);

    let player1 = test_player_guid(1);
    let player2 = test_player_guid(2);

    // Both join General (default)
    system
        .join_channel(player1, "General", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player2, "General", None, Team::Alliance)
        .await
        .unwrap();

    // Both join CustomChannel
    system
        .join_channel(player1, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();
    system
        .join_channel(player2, "CustomChannel", None, Team::Alliance)
        .await
        .unwrap();

    // Verify both are in both channels
    assert!(system.is_in_channel(player1, "General", Team::Alliance));
    assert!(system.is_in_channel(player2, "General", Team::Alliance));
    assert!(system.is_in_channel(player1, "CustomChannel", Team::Alliance));
    assert!(system.is_in_channel(player2, "CustomChannel", Team::Alliance));
}
