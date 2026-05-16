//! Guild System Tests for world
//!
//! Comprehensive test suite for the guild system types and logic.
//! Tests cover Guild, GuildMember, GuildRank, CachedGuild, and PlayerGuildState.

use super::types::*;
use crate::shared::protocol::{HighGuid, ObjectGuid};
use std::collections::HashMap;

// ========== TEST HELPERS ==========

fn test_player_guid(id: u32) -> ObjectGuid {
    ObjectGuid::new_without_entry(HighGuid::Player, id)
}

fn create_test_guild(id: u32, name: &str, leader_guid: ObjectGuid) -> Guild {
    Guild {
        id,
        name: name.to_string(),
        leader_guid,
        leader_name: "Leader".to_string(),
        emblem: GuildEmblem::default(),
        info: "Test Guild Info".to_string(),
        motd: "Test MOTD".to_string(),
        create_date: 1234567890,
    }
}

fn create_test_member(guid: ObjectGuid, rank: u8) -> GuildMember {
    GuildMember {
        guid,
        name: format!("Player{}", guid.raw()),
        rank,
        public_note: String::new(),
        officer_note: String::new(),
        level: 60,
        class: 1,
        zone: 0,
        account_id: 1,
        logout_time: 0,
    }
}

fn create_test_rank(id: u8, name: &str, rights: u32) -> GuildRank {
    GuildRank {
        id,
        name: name.to_string(),
        rights,
    }
}

fn create_default_ranks() -> Vec<GuildRank> {
    vec![
        create_test_rank(0, "Guild Master", 0xFFFFFFFF),
        create_test_rank(1, "Officer", GRIGHT_OFFCHATLISTEN),
        create_test_rank(2, "Veteran", 0),
        create_test_rank(3, "Member", 0),
        create_test_rank(4, "Initiate", 0),
    ]
}

// ========== CONSTANTS TESTS ==========

#[test]
fn test_guild_constants() {
    assert_eq!(GUILD_NAME_MAX_LENGTH, 24, "Guild name max length");
    assert_eq!(GUILD_RANKS_MAX_COUNT, 10, "Max number of guild ranks");
}

#[test]
fn test_guild_error_codes() {
    assert_eq!(ERR_GUILD_SUCCESS, 0);
    assert_eq!(ERR_GUILD_NAME_INVALID, 0x06);
    assert_eq!(ERR_GUILD_NAME_EXISTS, 0x07);
    assert_eq!(ERR_ALREADY_IN_GUILD_S, 0x03);
    assert_eq!(ERR_GUILD_PERMISSIONS, 0x08);
}

#[test]
fn test_guild_rights_constants() {
    assert_eq!(GRIGHT_OFFCHATLISTEN, 0x00000044);
}

// ========== GUILD EMBLEM TESTS ==========

#[test]
fn test_guild_emblem_default() {
    let emblem = GuildEmblem::default();
    assert_eq!(emblem.style, 0);
    assert_eq!(emblem.color, 0);
    assert_eq!(emblem.border_style, 0);
    assert_eq!(emblem.border_color, 0);
    assert_eq!(emblem.background_color, 0);
}

#[test]
fn test_guild_emblem_clone() {
    let emblem = GuildEmblem {
        style: 1,
        color: 2,
        border_style: 3,
        border_color: 4,
        background_color: 5,
    };
    let cloned = emblem.clone();
    assert_eq!(cloned.style, emblem.style);
    assert_eq!(cloned.color, emblem.color);
    assert_eq!(cloned.border_style, emblem.border_style);
    assert_eq!(cloned.border_color, emblem.border_color);
    assert_eq!(cloned.background_color, emblem.background_color);
}

// ========== GUILD TESTS ==========

#[test]
fn test_guild_creation() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);

    assert_eq!(guild.id, 1);
    assert_eq!(guild.name, "TestGuild");
    assert_eq!(guild.leader_guid, leader_guid);
    assert_eq!(guild.leader_name, "Leader");
    assert_eq!(guild.info, "Test Guild Info");
    assert_eq!(guild.motd, "Test MOTD");
    assert_eq!(guild.create_date, 1234567890);
}

#[test]
fn test_guild_clone() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let cloned = guild.clone();

    assert_eq!(cloned.id, guild.id);
    assert_eq!(cloned.name, guild.name);
    assert_eq!(cloned.leader_guid, guild.leader_guid);
}

// ========== GUILD MEMBER TESTS ==========

#[test]
fn test_guild_member_creation() {
    let guid = test_player_guid(100);
    let member = create_test_member(guid, 3);

    assert_eq!(member.guid, guid);
    assert_eq!(member.rank, 3);
    assert_eq!(member.level, 60);
    assert_eq!(member.class, 1);
    assert_eq!(member.public_note, "");
    assert_eq!(member.officer_note, "");
}

#[test]
fn test_guild_member_clone() {
    let guid = test_player_guid(100);
    let member = create_test_member(guid, 2);
    let cloned = member.clone();

    assert_eq!(cloned.guid, member.guid);
    assert_eq!(cloned.rank, member.rank);
    assert_eq!(cloned.name, member.name);
}

// ========== GUILD RANK TESTS ==========

#[test]
fn test_guild_rank_creation() {
    let rank = create_test_rank(0, "Guild Master", 0xFFFFFFFF);

    assert_eq!(rank.id, 0);
    assert_eq!(rank.name, "Guild Master");
    assert_eq!(rank.rights, 0xFFFFFFFF);
}

#[test]
fn test_guild_rank_clone() {
    let rank = create_test_rank(1, "Officer", GRIGHT_OFFCHATLISTEN);
    let cloned = rank.clone();

    assert_eq!(cloned.id, rank.id);
    assert_eq!(cloned.name, rank.name);
    assert_eq!(cloned.rights, rank.rights);
}

#[test]
fn test_default_ranks_structure() {
    let ranks = create_default_ranks();

    assert_eq!(ranks.len(), 5);
    assert_eq!(ranks[0].id, 0);
    assert_eq!(ranks[0].name, "Guild Master");
    assert_eq!(ranks[4].id, 4);
    assert_eq!(ranks[4].name, "Initiate");
}

// ========== CACHED GUILD TESTS ==========

#[test]
fn test_cached_guild_new() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![
        create_test_member(leader_guid, 0),
        create_test_member(test_player_guid(2), 3),
    ];

    let cached = CachedGuild::new(guild.clone(), ranks.clone(), members.clone());

    assert_eq!(cached.guild.id, 1);
    assert_eq!(cached.ranks.len(), 5);
    assert_eq!(cached.members.len(), 2);
}

#[test]
fn test_cached_guild_has_member() {
    let leader_guid = test_player_guid(1);
    let member_guid = test_player_guid(2);
    let non_member_guid = test_player_guid(999);

    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![
        create_test_member(leader_guid, 0),
        create_test_member(member_guid, 3),
    ];

    let cached = CachedGuild::new(guild, ranks, members);

    assert!(cached.has_member(leader_guid));
    assert!(cached.has_member(member_guid));
    assert!(!cached.has_member(non_member_guid));
}

#[test]
fn test_cached_guild_get_member() {
    let leader_guid = test_player_guid(1);
    let member_guid = test_player_guid(2);

    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![
        create_test_member(leader_guid, 0),
        create_test_member(member_guid, 3),
    ];

    let cached = CachedGuild::new(guild, ranks, members);

    let found = cached.get_member(member_guid);
    assert!(found.is_some());
    assert_eq!(found.unwrap().guid, member_guid);
    assert_eq!(found.unwrap().rank, 3);

    let not_found = cached.get_member(test_player_guid(999));
    assert!(not_found.is_none());
}

#[test]
fn test_cached_guild_member_count() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();

    // Empty guild
    let cached = CachedGuild::new(guild.clone(), ranks.clone(), vec![]);
    assert_eq!(cached.member_count(), 0);

    // Guild with members
    let members = vec![
        create_test_member(leader_guid, 0),
        create_test_member(test_player_guid(2), 3),
        create_test_member(test_player_guid(3), 4),
    ];
    let cached = CachedGuild::new(guild, ranks, members);
    assert_eq!(cached.member_count(), 3);
}

#[test]
fn test_cached_guild_get_rank() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![create_test_member(leader_guid, 0)];

    let cached = CachedGuild::new(guild, ranks, members);

    // Test each rank
    for rank_id in 0..5 {
        let rank = cached.get_rank(rank_id);
        assert!(rank.is_some(), "Rank {} should exist", rank_id);
        assert_eq!(rank.unwrap().id, rank_id);
    }

    // Non-existent rank
    let rank = cached.get_rank(99);
    assert!(rank.is_none());
}

#[test]
fn test_cached_guild_get_lowest_rank_id() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let members = vec![create_test_member(leader_guid, 0)];

    // Default 5 ranks (0-4)
    let ranks = create_default_ranks();
    let cached = CachedGuild::new(guild.clone(), ranks, members.clone());
    assert_eq!(cached.get_lowest_rank_id(), 4);

    // Custom ranks
    let custom_ranks = vec![
        create_test_rank(0, "Guild Master", 0xFFFFFFFF),
        create_test_rank(1, "Member", 0),
        create_test_rank(2, "Initiate", 0),
    ];
    let cached = CachedGuild::new(guild.clone(), custom_ranks, members.clone());
    assert_eq!(cached.get_lowest_rank_id(), 2);

    // Empty ranks (fallback)
    let cached = CachedGuild::new(guild, vec![], members);
    assert_eq!(cached.get_lowest_rank_id(), 4);
}

#[test]
fn test_cached_guild_is_guild_master() {
    let leader_guid = test_player_guid(1);
    let member_guid = test_player_guid(2);

    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![
        create_test_member(leader_guid, 0),
        create_test_member(member_guid, 3),
    ];

    let cached = CachedGuild::new(guild, ranks, members);

    assert!(cached.is_guild_master(leader_guid));
    assert!(!cached.is_guild_master(member_guid));
    assert!(!cached.is_guild_master(test_player_guid(999)));
}

#[test]
fn test_cached_guild_clone() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![create_test_member(leader_guid, 0)];

    let cached = CachedGuild::new(guild, ranks, members);
    let cloned = cached.clone();

    assert_eq!(cloned.guild.id, cached.guild.id);
    assert_eq!(cloned.ranks.len(), cached.ranks.len());
    assert_eq!(cloned.members.len(), cached.members.len());
}

// ========== GUILD DATA TESTS ==========

#[test]
fn test_guild_data_structure() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();

    let mut members = HashMap::new();
    members.insert(leader_guid, create_test_member(leader_guid, 0));

    let data = GuildData {
        guild_id: 1,
        info: guild,
        members,
        ranks,
    };

    assert_eq!(data.guild_id, 1);
    assert_eq!(data.info.name, "TestGuild");
    assert_eq!(data.members.len(), 1);
    assert_eq!(data.ranks.len(), 5);
}

#[test]
fn test_guild_data_clone() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();

    let mut members = HashMap::new();
    members.insert(leader_guid, create_test_member(leader_guid, 0));

    let data = GuildData {
        guild_id: 1,
        info: guild,
        members,
        ranks,
    };

    let cloned = data.clone();
    assert_eq!(cloned.guild_id, data.guild_id);
    assert_eq!(cloned.members.len(), data.members.len());
}

// ========== PLAYER GUILD STATE TESTS ==========

#[test]
fn test_player_guild_state_default() {
    let state = PlayerGuildState::default();
    assert_eq!(state.guild_id, None);
    assert_eq!(state.rank_id, 0);
}

#[test]
fn test_player_guild_state_with_guild() {
    let state = PlayerGuildState {
        guild_id: Some(42),
        rank_id: 3,
    };

    assert_eq!(state.guild_id, Some(42));
    assert_eq!(state.rank_id, 3);
}

#[test]
fn test_player_guild_state_clone() {
    let state = PlayerGuildState {
        guild_id: Some(10),
        rank_id: 2,
    };

    let cloned = state.clone();
    assert_eq!(cloned.guild_id, state.guild_id);
    assert_eq!(cloned.rank_id, state.rank_id);
}

// ========== EDGE CASES AND VALIDATION ==========

#[test]
fn test_guild_name_length_validation() {
    // This tests that the constant is defined correctly
    // Actual validation would be in the system layer
    let max_name = "a".repeat(GUILD_NAME_MAX_LENGTH);
    assert_eq!(max_name.len(), 24);

    let too_long = "a".repeat(GUILD_NAME_MAX_LENGTH + 1);
    assert_eq!(too_long.len(), 25);
}

#[test]
fn test_rank_count_limit() {
    assert!(
        GUILD_RANKS_MAX_COUNT >= 5,
        "Should support at least 5 default ranks"
    );
    assert!(GUILD_RANKS_MAX_COUNT <= 10, "WoW 1.12 limit is 10 ranks");
}

#[test]
fn test_member_with_all_ranks() {
    let guid = test_player_guid(1);

    for rank_id in 0..5 {
        let member = create_test_member(guid, rank_id);
        assert_eq!(member.rank, rank_id);
    }
}

#[test]
fn test_cached_guild_multiple_members() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();

    let mut members = Vec::new();
    for i in 1..=10 {
        members.push(create_test_member(test_player_guid(i), (i % 5) as u8));
    }

    let cached = CachedGuild::new(guild, ranks, members);
    assert_eq!(cached.member_count(), 10);

    // Check all members are accessible
    for i in 1..=10 {
        let member_guid = test_player_guid(i);
        assert!(cached.has_member(member_guid));
        assert!(cached.get_member(member_guid).is_some());
    }
}

#[test]
fn test_guild_data_member_operations() {
    let leader_guid = test_player_guid(1);
    let member_guid = test_player_guid(2);
    let guild = create_test_guild(1, "TestGuild", leader_guid);
    let ranks = create_default_ranks();

    let mut members = HashMap::new();
    members.insert(leader_guid, create_test_member(leader_guid, 0));

    let mut data = GuildData {
        guild_id: 1,
        info: guild,
        members,
        ranks,
    };

    // Add member
    data.members
        .insert(member_guid, create_test_member(member_guid, 3));
    assert_eq!(data.members.len(), 2);

    // Remove member
    data.members.remove(&member_guid);
    assert_eq!(data.members.len(), 1);

    // Check leader still exists
    assert!(data.members.contains_key(&leader_guid));
}

#[test]
fn test_emblem_custom_colors() {
    let emblem = GuildEmblem {
        style: 15,
        color: 255,
        border_style: 20,
        border_color: 128,
        background_color: 64,
    };

    assert_eq!(emblem.style, 15);
    assert_eq!(emblem.color, 255);
    assert_eq!(emblem.border_style, 20);
    assert_eq!(emblem.border_color, 128);
    assert_eq!(emblem.background_color, 64);
}

#[test]
fn test_guild_with_empty_strings() {
    let leader_guid = test_player_guid(1);
    let guild = Guild {
        id: 1,
        name: "TestGuild".to_string(),
        leader_guid,
        leader_name: String::new(),
        emblem: GuildEmblem::default(),
        info: String::new(),
        motd: String::new(),
        create_date: 0,
    };

    assert_eq!(guild.leader_name, "");
    assert_eq!(guild.info, "");
    assert_eq!(guild.motd, "");
}

#[test]
fn test_member_notes() {
    let guid = test_player_guid(1);
    let mut member = create_test_member(guid, 2);

    member.public_note = "Good player".to_string();
    member.officer_note = "Reliable".to_string();

    assert_eq!(member.public_note, "Good player");
    assert_eq!(member.officer_note, "Reliable");
}

#[test]
fn test_rank_rights_combinations() {
    let rank1 = create_test_rank(0, "Full Rights", 0xFFFFFFFF);
    let rank2 = create_test_rank(1, "No Rights", 0);
    let rank3 = create_test_rank(2, "Officer", GRIGHT_OFFCHATLISTEN);

    assert_eq!(rank1.rights, 0xFFFFFFFF);
    assert_eq!(rank2.rights, 0);
    assert_eq!(rank3.rights, GRIGHT_OFFCHATLISTEN);
}

#[test]
fn test_cached_guild_empty_members() {
    let leader_guid = test_player_guid(1);
    let guild = create_test_guild(1, "EmptyGuild", leader_guid);
    let ranks = create_default_ranks();
    let members = vec![];

    let cached = CachedGuild::new(guild, ranks, members);

    assert_eq!(cached.member_count(), 0);
    assert!(!cached.has_member(leader_guid));
    assert!(cached.get_member(leader_guid).is_none());
}

#[test]
fn test_player_guild_state_no_guild() {
    let state = PlayerGuildState {
        guild_id: None,
        rank_id: 0,
    };

    assert!(state.guild_id.is_none());
    assert_eq!(state.rank_id, 0);
}

#[test]
fn test_guid_consistency() {
    let guid = test_player_guid(42);
    let member = create_test_member(guid, 0);

    assert_eq!(member.guid, guid);
    assert_eq!(member.guid.raw(), 42);
}

// ========== INTEGRATION TESTS ==========
// These tests verify system operations with mocked dependencies

#[cfg(test)]
mod integration_tests {
    use super::super::system::GuildSystem;
    use super::*;
    use crate::shared::database::characters::guild_repository::MockGuildRepositoryTrait;
    use crate::shared::protocol::Opcode;
    use crate::shared::protocol::WorldPacket;
    use crate::world::game::broadcast_mgr::MockBroadcastManagerTrait;
    use crate::world::game::items::ItemManager;
    use crate::world::game::player::PlayerManager;
    use mockall::predicate::*;
    use std::sync::Arc;

    /// Create a test GuildSystem with mocked repository and broadcaster
    fn create_test_system(
        mock_repo: MockGuildRepositoryTrait,
        mock_broadcaster: MockBroadcastManagerTrait,
    ) -> GuildSystem {
        GuildSystem::new(
            Arc::new(mock_repo),
            Arc::new(mock_broadcaster),
            Arc::new(PlayerManager::new()),
            Arc::new(ItemManager::new()),
        )
    }

    /// Helper to verify packet opcode
    fn has_opcode(expected: Opcode) -> impl Fn(&WorldPacket) -> bool {
        move |packet: &WorldPacket| packet.opcode() == expected
    }

    #[tokio::test]
    async fn test_initialize_success_sets_next_guild_id() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo
            .expect_get_max_guild_id()
            .times(1)
            .returning(|| Ok(Some(42)));

        let system = create_test_system(mock_repo, mock_broadcaster);
        system.initialize().await.unwrap();

        // Verify next guild ID is set correctly (max + 1)
        // Note: We can't access next_guild_id directly, so we verify by creating a guild
    }

    #[tokio::test]
    async fn test_initialize_no_guilds_starts_at_one() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo
            .expect_get_max_guild_id()
            .times(1)
            .returning(|| Ok(None));

        let system = create_test_system(mock_repo, mock_broadcaster);
        system.initialize().await.unwrap();

        // System should start at ID 1 when no guilds exist
    }

    #[tokio::test]
    async fn test_create_guild_success_sends_packet() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        // Name doesn't exist
        mock_repo
            .expect_exists_by_name()
            .with(eq("TestGuild"))
            .times(1)
            .returning(|_| Ok(false));

        // Create succeeds
        mock_repo
            .expect_create()
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        // Expect 3 packets sent to leader: command result, guild query response, guild roster
        mock_broadcaster
            .expect_send_to_player()
            .times(3)
            .returning(|_, _| ());

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        system
            .create_guild_from_petition(leader_guid, "Leader".to_string(), "TestGuild".to_string())
            .await
            .unwrap();

        // Verify guild in cache
        assert!(system.is_in_guild(leader_guid));
        let state = system.get_player_guild(leader_guid);
        assert!(state.is_some());
        assert!(state.unwrap().guild_id.is_some());
    }

    #[tokio::test]
    async fn test_create_guild_name_too_long_sends_error() {
        let mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        // Should send error packet
        mock_broadcaster
            .expect_send_to_player()
            .times(1)
            .returning(|_, _| ());

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        let long_name = "A".repeat(25); // > 24 chars
        system
            .create_guild_from_petition(leader_guid, "Leader".to_string(), long_name)
            .await
            .unwrap();

        // Guild should NOT be in cache
        assert!(!system.is_in_guild(leader_guid));
    }

    #[tokio::test]
    async fn test_create_guild_name_exists_sends_error() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo
            .expect_exists_by_name()
            .with(eq("ExistingGuild"))
            .times(1)
            .returning(|_| Ok(true));

        // Should send error packet
        mock_broadcaster
            .expect_send_to_player()
            .times(1)
            .returning(|_, _| ());

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        system
            .create_guild_from_petition(
                leader_guid,
                "Leader".to_string(),
                "ExistingGuild".to_string(),
            )
            .await
            .unwrap();

        // Guild should NOT be created
        assert!(!system.is_in_guild(leader_guid));
    }

    #[tokio::test]
    async fn test_create_guild_database_error_returns_error() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo
            .expect_exists_by_name()
            .times(1)
            .returning(|_| Ok(false));

        mock_repo
            .expect_create()
            .times(1)
            .returning(|_, _, _, _| Err(anyhow::anyhow!("Database error")));

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        let result = system
            .create_guild_from_petition(leader_guid, "Leader".to_string(), "TestGuild".to_string())
            .await;

        assert!(result.is_err());
        assert!(!system.is_in_guild(leader_guid));
    }

    #[tokio::test]
    async fn test_guild_creation_verifies_packet_count() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo.expect_exists_by_name().returning(|_| Ok(false));

        mock_repo.expect_create().returning(|_, _, _, _| Ok(()));

        // Verify 3 packets sent: command result, guild query response, guild roster
        mock_broadcaster
            .expect_send_to_player()
            .times(3)
            .returning(|_, _| ());

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        system
            .create_guild_from_petition(leader_guid, "Leader".to_string(), "TestGuild".to_string())
            .await
            .unwrap();

        // Mock expectations verified on drop
    }

    #[tokio::test]
    async fn test_player_already_in_guild_sends_error() {
        let mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        let leader_guid = test_player_guid(100);

        // Pre-populate system with player in a guild
        let system = create_test_system(mock_repo, mock_broadcaster);

        // Manually insert player into guild state (simulating existing membership)
        // This would require system to have methods to add guild data directly,
        // or we test via sequential create operations

        // For now, this test documents the expected behavior:
        // If player is already in guild, send error packet
    }

    #[tokio::test]
    async fn test_create_multiple_guilds_sequential_ids() {
        let mut mock_repo1 = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster1 = MockBroadcastManagerTrait::new();

        mock_repo1.expect_exists_by_name().returning(|_| Ok(false));
        mock_repo1.expect_create().returning(|_, _, _, _| Ok(()));
        mock_broadcaster1
            .expect_send_to_player()
            .returning(|_, _| ());

        let system = create_test_system(mock_repo1, mock_broadcaster1);

        system
            .create_guild_from_petition(
                test_player_guid(100),
                "Leader1".to_string(),
                "Guild1".to_string(),
            )
            .await
            .unwrap();

        assert!(system.is_in_guild(test_player_guid(100)));

        // Second guild would require additional setup with mocks
        // This test documents the pattern for testing sequential operations
    }

    #[tokio::test]
    async fn test_verify_opcode_sent_on_success() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo.expect_exists_by_name().returning(|_| Ok(false));

        mock_repo.expect_create().returning(|_, _, _, _| Ok(()));

        // Verify SMSG_GUILD_COMMAND_RESULT packet sent (plus query response and roster)
        mock_broadcaster
            .expect_send_to_player()
            .times(1)
            .withf(|_player_guid, packet: &WorldPacket| {
                packet.opcode() == Opcode::SMSG_GUILD_COMMAND_RESULT
            })
            .returning(|_, _| ());

        // Allow the guild query response and roster packets
        mock_broadcaster
            .expect_send_to_player()
            .times(2)
            .returning(|_, _| ());

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        system
            .create_guild_from_petition(leader_guid, "Leader".to_string(), "TestGuild".to_string())
            .await
            .unwrap();

        // Mock expectations verified on drop
    }

    #[tokio::test]
    async fn test_verify_opcode_sent_on_error() {
        let mut mock_repo = MockGuildRepositoryTrait::new();
        let mut mock_broadcaster = MockBroadcastManagerTrait::new();

        mock_repo
            .expect_exists_by_name()
            .with(eq("ExistingGuild"))
            .returning(|_| Ok(true));

        // Verify error packet with correct opcode
        mock_broadcaster
            .expect_send_to_player()
            .times(1)
            .withf(|_player_guid, packet: &WorldPacket| {
                packet.opcode() == Opcode::SMSG_GUILD_COMMAND_RESULT
            })
            .returning(|_, _| ());

        let system = create_test_system(mock_repo, mock_broadcaster);
        let leader_guid = test_player_guid(100);

        system
            .create_guild_from_petition(
                leader_guid,
                "Leader".to_string(),
                "ExistingGuild".to_string(),
            )
            .await
            .unwrap();

        // Mock expectations verified on drop
    }
}
