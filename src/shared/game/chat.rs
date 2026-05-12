//! Shared chat types - used by both world and world
//!
//! This module contains protocol-level chat types that are shared
//! between the old world and new world implementations.

/// Chat message types (MaNGOS-compatible values)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ChatMsg {
    Addon = 0xFFFFFFFF,
    Say = 0x00,
    Party = 0x01,
    Raid = 0x02,
    Guild = 0x03,
    Officer = 0x04,
    Yell = 0x05,
    Whisper = 0x06,
    WhisperInform = 0x07,
    Emote = 0x08,
    TextEmote = 0x09,
    System = 0x0A,
    MonsterSay = 0x0B,
    MonsterYell = 0x0C,
    MonsterEmote = 0x0D,
    Channel = 0x0E,
    ChannelJoin = 0x0F,
    ChannelLeave = 0x10,
    ChannelList = 0x11,
    ChannelNotice = 0x12,
    ChannelNoticeUser = 0x13,
    Afk = 0x14,
    Dnd = 0x15,
    Ignored = 0x16,
    Skill = 0x17,
    Loot = 0x18,
    CombatMiscInfo = 0x19,
    MonsterWhisper = 0x1A,
    CombatSelfHits = 0x1B,
    CombatSelfMisses = 0x1C,
    CombatPetHits = 0x1D,
    CombatPetMisses = 0x1E,
    CombatPartyHits = 0x1F,
    CombatPartyMisses = 0x20,
    CombatFriendlyPlayerHits = 0x21,
    CombatFriendlyPlayerMisses = 0x22,
    CombatHostilePlayerHits = 0x23,
    CombatHostilePlayerMisses = 0x24,
    RaidLeader = 0x57,
    RaidWarning = 0x58,
    RaidBossWhisper = 0x59,
    RaidBossEmote = 0x5A,
    Filtered = 0x5B,
    Battleground = 0x5C,
    BattlegroundLeader = 0x5D,
}

impl ChatMsg {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0xFFFFFFFF => Some(ChatMsg::Addon),
            0x00 => Some(ChatMsg::Say),
            0x01 => Some(ChatMsg::Party),
            0x02 => Some(ChatMsg::Raid),
            0x03 => Some(ChatMsg::Guild),
            0x04 => Some(ChatMsg::Officer),
            0x05 => Some(ChatMsg::Yell),
            0x06 => Some(ChatMsg::Whisper),
            0x07 => Some(ChatMsg::WhisperInform),
            0x08 => Some(ChatMsg::Emote),
            0x09 => Some(ChatMsg::TextEmote),
            0x0A => Some(ChatMsg::System),
            0x0B => Some(ChatMsg::MonsterSay),
            0x0C => Some(ChatMsg::MonsterYell),
            0x0D => Some(ChatMsg::MonsterEmote),
            0x0E => Some(ChatMsg::Channel),
            0x0F => Some(ChatMsg::ChannelJoin),
            0x10 => Some(ChatMsg::ChannelLeave),
            0x11 => Some(ChatMsg::ChannelList),
            0x12 => Some(ChatMsg::ChannelNotice),
            0x13 => Some(ChatMsg::ChannelNoticeUser),
            0x14 => Some(ChatMsg::Afk),
            0x15 => Some(ChatMsg::Dnd),
            0x16 => Some(ChatMsg::Ignored),
            0x17 => Some(ChatMsg::Skill),
            0x18 => Some(ChatMsg::Loot),
            0x19 => Some(ChatMsg::CombatMiscInfo),
            0x1A => Some(ChatMsg::MonsterWhisper),
            0x1B => Some(ChatMsg::CombatSelfHits),
            0x1C => Some(ChatMsg::CombatSelfMisses),
            0x1D => Some(ChatMsg::CombatPetHits),
            0x1E => Some(ChatMsg::CombatPetMisses),
            0x1F => Some(ChatMsg::CombatPartyHits),
            0x20 => Some(ChatMsg::CombatPartyMisses),
            0x21 => Some(ChatMsg::CombatFriendlyPlayerHits),
            0x22 => Some(ChatMsg::CombatFriendlyPlayerMisses),
            0x23 => Some(ChatMsg::CombatHostilePlayerHits),
            0x24 => Some(ChatMsg::CombatHostilePlayerMisses),
            0x57 => Some(ChatMsg::RaidLeader),
            0x58 => Some(ChatMsg::RaidWarning),
            0x59 => Some(ChatMsg::RaidBossWhisper),
            0x5A => Some(ChatMsg::RaidBossEmote),
            0x5B => Some(ChatMsg::Filtered),
            0x5C => Some(ChatMsg::Battleground),
            0x5D => Some(ChatMsg::BattlegroundLeader),
            _ => None,
        }
    }
}

/// Language types (MaNGOS-compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Language {
    Universal = 0,
    Orcish = 1,
    Darnassian = 2,
    Taurahe = 3,
    Dwarvish = 6,
    Common = 7,
    Demonic = 8,
    Titan = 9,
    Thalassian = 10,
    Draconic = 11,
    Kalimag = 12,
    Gnomish = 13,
    Troll = 14,
    Gutterspeak = 33,
    Addon = 0xFFFFFFFF,
}

impl Language {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Language::Universal,
            1 => Language::Orcish,
            2 => Language::Darnassian,
            3 => Language::Taurahe,
            6 => Language::Dwarvish,
            7 => Language::Common,
            8 => Language::Demonic,
            9 => Language::Titan,
            10 => Language::Thalassian,
            11 => Language::Draconic,
            12 => Language::Kalimag,
            13 => Language::Gnomish,
            14 => Language::Troll,
            33 => Language::Gutterspeak,
            0xFFFFFFFF => Language::Addon,
            _ => Language::Universal,
        }
    }

    /// Get the primary racial language for a given faction
    /// Used for cross-faction chat translation
    pub fn for_faction(team: Team) -> Language {
        match team {
            Team::Alliance => Language::Common,
            Team::Horde => Language::Orcish,
            Team::None | Team::CrossFaction => Language::Universal,
        }
    }
}

/// Chat tags (for player status indicators)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ChatTag {
    None = 0x00,
    Afk = 0x01,
    Dnd = 0x02,
    Gm = 0x03,
}

impl ChatTag {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x01 => ChatTag::Afk,
            0x02 => ChatTag::Dnd,
            0x03 => ChatTag::Gm,
            _ => ChatTag::None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Default for ChatTag {
    fn default() -> Self {
        ChatTag::None
    }
}

/// Team (Alliance/Horde)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Team {
    None = 0,
    CrossFaction = 1,
    Horde = 67,
    Alliance = 469,
}

impl Team {
    pub fn from_race(race: u8) -> Self {
        match race {
            1 | 3 | 4 | 7 => Team::Alliance,
            2 | 5 | 6 | 8 => Team::Horde,
            _ => Team::None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Channel notification types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ChatNotify {
    JoinedNotice = 0x00,
    LeftNotice = 0x01,
    YouJoinedNotice = 0x02,
    YouLeftNotice = 0x03,
    WrongPasswordNotice = 0x04,
    NotMemberNotice = 0x05,
    NotModeratorNotice = 0x06,
    PasswordChangedNotice = 0x07,
    OwnerChangedNotice = 0x08,
    PlayerNotFoundNotice = 0x09,
    NotOwnerNotice = 0x0A,
    ChannelOwnerNotice = 0x0B,
    ModeChangeNotice = 0x0C,
    AnnouncementsOnNotice = 0x0D,
    AnnouncementsOffNotice = 0x0E,
    ModerationOnNotice = 0x0F,
    ModerationOffNotice = 0x10,
    MutedNotice = 0x11,
    PlayerKickedNotice = 0x12,
    BannedNotice = 0x13,
    PlayerBannedNotice = 0x14,
    PlayerUnbannedNotice = 0x15,
    PlayerNotBannedNotice = 0x16,
    PlayerAlreadyMemberNotice = 0x17,
    InviteNotice = 0x18,
    InviteWrongFactionNotice = 0x19,
    WrongFactionNotice = 0x1A,
    InvalidNameNotice = 0x1B,
    NotModeratedNotice = 0x1C,
    PlayerInvitedNotice = 0x1D,
    PlayerInviteBannedNotice = 0x1E,
    ThrottledNotice = 0x1F,
}

/// Range for emote messages
pub const EMOTE_RANGE: f32 = 25.0;

/// Range for text emote messages
pub const TEXT_EMOTE_RANGE: f32 = 25.0;

/// Result of a channel join operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelJoinResult {
    /// Successfully joined
    Joined { channel_id: u32 },
    /// Already a member
    AlreadyMember,
    /// Banned from channel
    Banned,
    /// Wrong password
    WrongPassword,
    /// Maximum channels reached
    MaxChannelsReached,
    /// Invalid channel name
    InvalidName,
}

/// Result of a channel leave operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelLeaveResult {
    /// Successfully left
    Left,
    /// Was not a member
    NotMember,
}
