//! Guild system types (excluding bank-related types)

use crate::protocol::ObjectGuid;
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
#[derive(Debug, Clone)]
pub struct CachedGuild {
    pub guild: Guild,
    pub ranks: Vec<GuildRank>,
    pub members: Vec<GuildMember>,
}

impl CachedGuild {
    pub fn new(guild: Guild, ranks: Vec<GuildRank>, members: Vec<GuildMember>) -> Self {
        Self {
            guild,
            ranks,
            members,
        }
    }

    pub fn has_member(&self, guid: ObjectGuid) -> bool {
        self.members.iter().any(|m| m.guid == guid)
    }

    pub fn get_member(&self, guid: ObjectGuid) -> Option<&GuildMember> {
        self.members.iter().find(|m| m.guid == guid)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn get_rank(&self, rank_id: u8) -> Option<&GuildRank> {
        self.ranks.iter().find(|r| r.id == rank_id)
    }

    pub fn get_lowest_rank_id(&self) -> u8 {
        self.ranks.iter().map(|r| r.id).max().unwrap_or(4)
    }

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

pub type GuildId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GuildEvent {
    Joined = 0,
    Left = 1,
    Removed = 2,
    IsLeader = 3,
    ChangedLeader = 4,
    ChangedLeaderName = 5,
    Disbanded = 6,
    Motd = 7,
    SignedOn = 8,
    SignedOff = 9,
    GuildBankGoldDeposited = 10,
    GuildBankGoldWithdrawn = 11,
    TabUpdated = 12,
    TabInfo = 13,
    ItemMoved = 14,
    ItemDeposited = 15,
    ItemWithdrawn = 16,
    MoneyDeposited = 17,
    MoneyWithdrawn = 18,
}

impl GuildEvent {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildPermissions(pub u32);

impl GuildPermissions {
    pub const GUILD_RIGHT_EMPTY: u32 = 0x00000040;
    pub const GUILD_RIGHT_EVERYONE: u32 = 0x00000001;
    pub const GUILD_RIGHT_OFFICER: u32 = 0x00000002;
    pub const GUILD_RIGHT_GUILD: u32 = 0x00000004;
    pub const GUILD_RIGHT_REMOVE: u32 = 0x00000008;
    pub const GUILD_RIGHT_INVITE: u32 = 0x00000010;
    pub const GUILD_RIGHT_SETMOTD: u32 = 0x00000020;
    pub const GUILD_RIGHT_EDIT_PUBLIC_NOTE: u32 = 0x00000080;
    pub const GUILD_RIGHT_WITHDRAW_GOLD_LOCK: u32 = 0x00000100;
    pub const GUILD_RIGHT_WITHDRAW_REPAIR: u32 = 0x00000200;
    pub const GUILD_RIGHT_WITHDRAW_MONEY: u32 = 0x00000400;
    pub const GUILD_RIGHT_CREATE_GUILD_EVENT: u32 = 0x00000800;
    pub const GUILD_RIGHT_ALL: u32 = 0x00000FFF;

    pub fn has_right(&self, right: u32) -> bool {
        (self.0 & right) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildBankRights {
    None = 0,
    View = 1,
    Deposit = 2,
    Withdraw = 4,
}

#[derive(Debug, Clone)]
pub struct GuildBankTab {
    pub id: u8,
    pub name: String,
    pub icon: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct GuildLogEntry {
    pub event_type: u8,
    pub player_guid: ObjectGuid,
    pub param1: u32,
    pub param2: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct GuildMemberNote {
    pub guid: ObjectGuid,
    pub public_note: String,
    pub officer_note: String,
}

#[derive(Debug, Clone)]
pub struct GuildMemberUpdateNote {
    pub guid: ObjectGuid,
    pub note: String,
    pub is_officer: bool,
}
