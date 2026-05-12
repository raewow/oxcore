//! Chat system types (world specific implementation types)
//!
//! Protocol-level types (ChatMsg, ChatTag, Language, Team, ChatNotify) are
//! in shared::game::chat and should be imported from there.
//! This module contains only implementation-specific types.

use crate::shared::game::chat::{
    ChannelJoinResult, ChannelLeaveResult, ChatMsg, ChatNotify, ChatTag, Language, Team,
    EMOTE_RANGE, TEXT_EMOTE_RANGE,
};
use crate::shared::protocol::ObjectGuid;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

// ============================================================================
// Channel IDs (DBC) - Implementation specific
// ============================================================================

/// Channel IDs (DBC)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChannelId {
    General = 1,
    Trade = 2,
    LocalDefense = 22,
    WorldDefense = 23,
    GuildRecruitment = 25,
    LookingForGroup = 26,
}

/// Channel flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelFlags {
    None = 0x00,
    Custom = 0x01,
    Trade = 0x04,
    NotLfg = 0x08,
    General = 0x10,
    City = 0x20,
    Lfg = 0x40,
    Voice = 0x80,
}

// ============================================================================
// Channel Member Types
// ============================================================================

/// Channel member flags - bitflags that can be combined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ChannelMemberFlags(u8);

impl ChannelMemberFlags {
    pub const NONE: ChannelMemberFlags = ChannelMemberFlags(0x00);
    pub const OWNER: ChannelMemberFlags = ChannelMemberFlags(0x01);
    pub const MODERATOR: ChannelMemberFlags = ChannelMemberFlags(0x02);
    pub const VOICED: ChannelMemberFlags = ChannelMemberFlags(0x04);
    pub const MUTED: ChannelMemberFlags = ChannelMemberFlags(0x08);
    pub const CUSTOM: ChannelMemberFlags = ChannelMemberFlags(0x10);
    pub const MIC_MUTED: ChannelMemberFlags = ChannelMemberFlags(0x20);

    pub fn has_flag(&self, flag: ChannelMemberFlags) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn set_flag(&mut self, flag: ChannelMemberFlags, set: bool) {
        if set {
            self.0 |= flag.0;
        } else {
            self.0 &= !flag.0;
        }
    }

    pub fn from_u8(value: u8) -> Self {
        ChannelMemberFlags(value)
    }

    pub fn as_u8(self) -> u8 {
        self.0
    }

    pub fn is_owner(&self) -> bool {
        self.has_flag(ChannelMemberFlags::OWNER)
    }

    pub fn is_moderator(&self) -> bool {
        self.has_flag(ChannelMemberFlags::MODERATOR)
    }

    pub fn is_muted(&self) -> bool {
        self.has_flag(ChannelMemberFlags::MUTED)
    }
}

impl Default for ChannelMemberFlags {
    fn default() -> Self {
        ChannelMemberFlags::NONE
    }
}

/// Channel member info
#[derive(Debug, Clone)]
pub struct ChannelMember {
    pub guid: ObjectGuid,
    pub flags: ChannelMemberFlags,
}

impl ChannelMember {
    pub fn new(guid: ObjectGuid) -> Self {
        Self {
            guid,
            flags: ChannelMemberFlags::NONE,
        }
    }
}

// ============================================================================
// Constants
// ============================================================================

/// Constants for chat ranges (in yards)
pub const SAY_RANGE: f32 = 25.0;
pub const YELL_RANGE: f32 = 300.0;
// EMOTE_RANGE and TEXT_EMOTE_RANGE moved to shared::game::chat

/// Constants for message validation
pub const MAX_MESSAGE_LENGTH: usize = 255;
pub const MAX_CHANNELS_PER_PLAYER: usize = 20;
pub const MAX_CHANNEL_NAME_LENGTH: usize = 31;

/// Flood protection defaults (can be overridden by config)
pub const DEFAULT_FLOOD_MESSAGE_COUNT: u32 = 10;
pub const DEFAULT_FLOOD_WINDOW_SECS: u64 = 10;
pub const DEFAULT_FLOOD_MUTE_SECS: u64 = 60;

// ============================================================================
// System-Specific Types (world additions)
// ============================================================================

/// Flood protection configuration
#[derive(Debug, Clone)]
pub struct FloodConfig {
    pub max_messages: u32,
    pub window_secs: u64,
    pub mute_duration_secs: u64,
}

impl Default for FloodConfig {
    fn default() -> Self {
        Self {
            max_messages: DEFAULT_FLOOD_MESSAGE_COUNT,
            window_secs: DEFAULT_FLOOD_WINDOW_SECS,
            mute_duration_secs: DEFAULT_FLOOD_MUTE_SECS,
        }
    }
}

/// Per-player flood state
#[derive(Debug, Clone)]
pub struct FloodState {
    pub message_timestamps: VecDeque<Instant>,
    pub mute_end_time: Option<Instant>,
}

impl FloodState {
    pub fn new() -> Self {
        Self {
            message_timestamps: VecDeque::new(),
            mute_end_time: None,
        }
    }
}

/// Channel data with members (for DashMap storage)
#[derive(Debug, Clone)]
pub struct ChannelData {
    pub channel: CachedChannel,
    pub members: HashMap<ObjectGuid, CachedChannelMember>,
    pub banned: HashSet<ObjectGuid>,
}

impl ChannelData {
    pub fn new(channel: CachedChannel) -> Self {
        Self {
            channel,
            members: HashMap::new(),
            banned: HashSet::new(),
        }
    }
}

/// Errors that can occur during chat operations
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChatError {
    #[error("Rate limited")]
    RateLimited,
    #[error("Muted for {remaining_secs} seconds")]
    Muted { remaining_secs: u64 },
    #[error("Empty message")]
    EmptyMessage,
    #[error("Message too long")]
    MessageTooLong,
    #[error("Invalid characters")]
    InvalidCharacters,
    #[error("Not in channel")]
    NotInChannel,
    #[error("Not in group")]
    NotInGroup,
    #[error("Not in guild")]
    NotInGuild,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Banned from channel")]
    BannedFromChannel,
    #[error("Wrong password")]
    WrongPassword,
    #[error("Already in channel")]
    AlreadyInChannel,
    #[error("Not owner")]
    NotOwner,
    #[error("Not moderator")]
    NotModerator,
    #[error("Target not found")]
    TargetNotFound,
    #[error("Target ignoring")]
    TargetIgnoring,
    #[error("Cross faction disabled")]
    CrossFactionDisabled,
    #[error("Muted in channel")]
    MutedInChannel,
    #[error("Max channels reached")]
    MaxChannelsReached,
    #[error("Invalid channel name")]
    InvalidChannelName,
    #[error("No permission")]
    NoPermission,
}

/// Cached channel data for system
#[derive(Debug, Clone)]
pub struct CachedChannel {
    pub id: u32,
    pub name: String,
    pub password: String,
    pub flags: u8,
    pub owner_guid: ObjectGuid,
    pub announce: bool,
    pub moderate: bool,
    pub team: Team,
}

impl CachedChannel {
    pub fn new(id: u32, name: String, team: Team) -> Self {
        let flags = if Self::is_builtin_channel(&name) {
            Self::get_builtin_flags(&name)
        } else {
            ChannelFlags::Custom as u8
        };

        Self {
            id,
            name,
            password: String::new(),
            flags,
            owner_guid: ObjectGuid::empty(),
            announce: true,
            moderate: false,
            team,
        }
    }

    pub fn is_builtin_channel(name: &str) -> bool {
        let lower = name.to_lowercase();
        matches!(
            lower.as_str(),
            "general"
                | "trade"
                | "localdefense"
                | "worlddefense"
                | "guildrecruitment"
                | "lookingforgroup"
        )
    }

    fn get_builtin_flags(name: &str) -> u8 {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "general" => ChannelFlags::General as u8 | ChannelFlags::NotLfg as u8,
            "trade" => {
                ChannelFlags::Trade as u8
                    | ChannelFlags::General as u8
                    | ChannelFlags::NotLfg as u8
                    | ChannelFlags::City as u8
            }
            "localdefense" => ChannelFlags::General as u8 | ChannelFlags::NotLfg as u8,
            "worlddefense" => ChannelFlags::General as u8,
            "guildrecruitment" => {
                ChannelFlags::General as u8 | ChannelFlags::NotLfg as u8 | ChannelFlags::City as u8
            }
            "lookingforgroup" => ChannelFlags::Lfg as u8 | ChannelFlags::General as u8,
            _ => ChannelFlags::Custom as u8,
        }
    }

    pub fn is_constant(&self) -> bool {
        Self::is_builtin_channel(&self.name)
    }
}

/// Cached channel member data
#[derive(Debug, Clone)]
pub struct CachedChannelMember {
    pub guid: ObjectGuid,
    pub flags: ChannelMemberFlags,
}

impl CachedChannelMember {
    pub fn new(guid: ObjectGuid) -> Self {
        Self {
            guid,
            flags: ChannelMemberFlags::NONE,
        }
    }

    pub fn is_owner(&self) -> bool {
        self.flags.is_owner()
    }

    pub fn is_moderator(&self) -> bool {
        self.flags.is_moderator()
    }

    pub fn is_muted(&self) -> bool {
        self.flags.is_muted()
    }
}

/// Get chat range for message type
pub fn get_chat_range(msgtype: ChatMsg) -> f32 {
    match msgtype {
        ChatMsg::Say => SAY_RANGE,
        ChatMsg::Yell => YELL_RANGE,
        ChatMsg::Emote => EMOTE_RANGE,
        ChatMsg::TextEmote => TEXT_EMOTE_RANGE,
        _ => 0.0,
    }
}
