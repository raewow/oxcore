#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendStatus {
    Offline = 0,
    Online = 1,
    Afk = 2,
    Unk3 = 3,
    Dnd = 4,
}

impl FriendStatus {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(FriendStatus::Offline),
            1 => Some(FriendStatus::Online),
            2 => Some(FriendStatus::Afk),
            3 => Some(FriendStatus::Unk3),
            4 => Some(FriendStatus::Dnd),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialFlag {
    Friend = 0x01,
    Ignored = 0x02,
    Muted = 0x04,
}

impl SocialFlag {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(SocialFlag::Friend),
            0x02 => Some(SocialFlag::Ignored),
            0x04 => Some(SocialFlag::Muted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FriendInfo {
    pub status: FriendStatus,
    pub flags: u8,
    pub area: u32,
    pub level: u32,
    pub class: u32,
}

impl FriendInfo {
    pub fn new(flags: u8) -> Self {
        Self {
            status: FriendStatus::Offline,
            flags,
            area: 0,
            level: 0,
            class: 0,
        }
    }

    pub fn has_flag(&self, flag: SocialFlag) -> bool {
        (self.flags & flag.as_u8()) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FriendsResult {
    DbError = 0x00,
    ListFull = 0x01,
    Online = 0x02,
    Offline = 0x03,
    NotFound = 0x04,
    Removed = 0x05,
    AddedOnline = 0x06,
    AddedOffline = 0x07,
    Already = 0x08,
    Self_ = 0x09,
    Enemy = 0x0A,
    IgnoreFull = 0x0B,
    IgnoreSelf = 0x0C,
    IgnoreNotFound = 0x0D,
    IgnoreAlready = 0x0E,
    IgnoreAdded = 0x0F,
    IgnoreRemoved = 0x10,
    IgnoreAmbiguous = 0x11,
    MuteFull = 0x12,
    MuteSelf = 0x13,
    MuteNotFound = 0x14,
    MuteAlready = 0x15,
    MuteAdded = 0x16,
    MuteRemoved = 0x17,
    MuteAmbiguous = 0x18,
    Unk7 = 0x19,
    Unknown = 0x1A,
}

impl FriendsResult {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

pub const SOCIALMGR_FRIEND_LIMIT: usize = 50;
pub const SOCIALMGR_IGNORE_LIMIT: usize = 25;
