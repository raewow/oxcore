//! Guild system types (excluding bank-related types)

use crate::shared::protocol::ObjectGuid;
use std::collections::HashMap;

// Constants
pub const GUILD_NAME_MAX_LENGTH: usize = 24;
pub const GUILD_RANKS_MAX_COUNT: usize = 10;

// Error codes
pub const ERR_GUILD_SUCCESS: u32 = 0;
pub const ERR_GUILD_NAME_INVALID: u32 = 0x06;
pub const ERR_GUILD_NAME_EXISTS: u32 = 0x07;
pub const ERR_ALREADY_IN_GUILD_S: u32 = 0x03;
pub const ERR_GUILD_PERMISSIONS: u32 = 0x08;

// Rank rights
pub const GRIGHT_OFFCHATLISTEN: u32 = 0x00000044;

// Guild member online status flag
pub const GRF_ONLINE: u8 = 1;

/// Guild emblem data
#[derive(Debug, Clone, Default)]
pub struct GuildEmblem {
    pub style: u8,
    pub color: u8,
    pub border_style: u8,
    pub border_color: u8,
    pub background_color: u8,
}

/// Core guild data
#[derive(Debug, Clone)]
pub struct Guild {
    pub id: u32,
    pub name: String,
    pub leader_guid: ObjectGuid,
    pub leader_name: String,
    pub emblem: GuildEmblem,
    pub info: String,
    pub motd: String,
    pub create_date: i64,
}

/// Guild member data
#[derive(Debug, Clone)]
pub struct GuildMember {
    pub guid: ObjectGuid,
    pub name: String,
    pub rank: u8,
    pub public_note: String,
    pub officer_note: String,
    pub level: u8,
    pub class: u8,
    pub zone: u32,
    pub account_id: u32,
    pub logout_time: i64,
}

/// Guild rank definition
#[derive(Debug, Clone)]
pub struct GuildRank {
    pub id: u8,
    pub name: String,
    pub rights: u32,
}

/// Complete cached guild data (guild + members + ranks)
/// Used in the cache layer for fast access
#[derive(Debug, Clone)]
pub struct CachedGuild {
    pub guild: Guild,
    pub ranks: Vec<GuildRank>,
    pub members: Vec<GuildMember>,
}

impl CachedGuild {
    /// Create a new cached guild
    pub fn new(guild: Guild, ranks: Vec<GuildRank>, members: Vec<GuildMember>) -> Self {
        Self {
            guild,
            ranks,
            members,
        }
    }

    /// Check if guild has a member
    pub fn has_member(&self, guid: ObjectGuid) -> bool {
        self.members.iter().any(|m| m.guid == guid)
    }

    /// Get member by GUID
    pub fn get_member(&self, guid: ObjectGuid) -> Option<&GuildMember> {
        self.members.iter().find(|m| m.guid == guid)
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get rank by ID
    pub fn get_rank(&self, rank_id: u8) -> Option<&GuildRank> {
        self.ranks.iter().find(|r| r.id == rank_id)
    }

    /// Get lowest rank ID (highest rank number - Initiate)
    pub fn get_lowest_rank_id(&self) -> u8 {
        self.ranks.iter().map(|r| r.id).max().unwrap_or(4)
    }

    /// Check if member is guild master
    pub fn is_guild_master(&self, member_guid: ObjectGuid) -> bool {
        self.guild.leader_guid == member_guid
    }
}

/// Per-guild data (owned by system) - for DashMap storage
#[derive(Debug, Clone)]
pub struct GuildData {
    pub guild_id: u32,
    pub info: Guild,
    pub members: HashMap<ObjectGuid, GuildMember>,
    pub ranks: Vec<GuildRank>,
}

impl GuildData {
    /// Get lowest rank ID (highest rank number - Initiate)
    pub fn get_lowest_rank_id(&self) -> u8 {
        self.ranks.iter().map(|r| r.id).max().unwrap_or(4)
    }
}

/// Per-player guild membership state
#[derive(Debug, Clone, Default)]
pub struct PlayerGuildState {
    pub guild_id: Option<u32>,
    pub rank_id: u8,
}
