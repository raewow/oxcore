pub type GuildId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GuildRank {
    GuildMaster = 0,
    Officer = 1,
    Veteran = 2,
    Member = 3,
    Initiate = 4,
}

impl GuildRank {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(GuildRank::GuildMaster),
            1 => Some(GuildRank::Officer),
            2 => Some(GuildRank::Veteran),
            3 => Some(GuildRank::Member),
            4 => Some(GuildRank::Initiate),
            _ => None,
        }
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
pub enum GuildMemberUpdateNote {
    Public = 0,
    Officer = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildBankTab {
    Tab0 = 0,
    Tab1 = 1,
    Tab2 = 2,
    Tab3 = 3,
    Tab4 = 4,
    Tab5 = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankRights(pub u32);

impl GuildBankRights {
    pub const VIEW_TAB: u32 = 0x01;
    pub const DEPOSIT_ITEM: u32 = 0x02;
    pub const UPDATE_TEXT: u32 = 0x04;
    pub const WITHDRAW_ITEM: u32 = 0x08;

    pub fn has_right(&self, right: u32) -> bool {
        (self.0 & right) != 0
    }
}

#[derive(Debug, Clone)]
pub struct GuildLogEntry {
    pub id: u32,
    pub timestamp: u32,
    pub event_type: GuildEvent,
    pub guid: crate::shared::protocol::ObjectGuid,
    pub data: [u32; 4],
}

impl GuildLogEntry {
    pub fn new(event_type: GuildEvent) -> Self {
        Self {
            id: 0,
            timestamp: 0,
            event_type,
            guid: crate::shared::protocol::ObjectGuid::empty(),
            data: [0; 4],
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuildMemberNote {
    pub public_note: String,
    pub officer_note: String,
}
