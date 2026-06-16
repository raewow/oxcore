//! GameObject types, states, and flags
//!
//! Pure data types matching the MaNGOS C++ definitions.

/// GameObject types (31 total)
/// Matches GameobjectTypes from GameObjectDefines.h
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GameObjectType {
    Door = 0,
    Button = 1,
    QuestGiver = 2,
    Chest = 3,
    Binder = 4,
    Generic = 5,
    Trap = 6,
    Chair = 7,
    SpellFocus = 8,
    Text = 9,
    Goober = 10,
    Transport = 11,
    AreaDamage = 12,
    Camera = 13,
    MapObject = 14,
    MoTransport = 15,
    DuelArbiter = 16,
    FishingNode = 17,
    SummoningRitual = 18,
    Mailbox = 19,
    AuctionHouse = 20,
    GuardPost = 21,
    SpellCaster = 22,
    MeetingStone = 23,
    FlagStand = 24,
    FishingHole = 25,
    FlagDrop = 26,
    MiniGame = 27,
    LotteryKiosk = 28,
    CapturePoint = 29,
    AuraGenerator = 30,
}

impl From<u32> for GameObjectType {
    fn from(value: u32) -> Self {
        match value {
            0 => GameObjectType::Door,
            1 => GameObjectType::Button,
            2 => GameObjectType::QuestGiver,
            3 => GameObjectType::Chest,
            4 => GameObjectType::Binder,
            5 => GameObjectType::Generic,
            6 => GameObjectType::Trap,
            7 => GameObjectType::Chair,
            8 => GameObjectType::SpellFocus,
            9 => GameObjectType::Text,
            10 => GameObjectType::Goober,
            11 => GameObjectType::Transport,
            12 => GameObjectType::AreaDamage,
            13 => GameObjectType::Camera,
            14 => GameObjectType::MapObject,
            15 => GameObjectType::MoTransport,
            16 => GameObjectType::DuelArbiter,
            17 => GameObjectType::FishingNode,
            18 => GameObjectType::SummoningRitual,
            19 => GameObjectType::Mailbox,
            20 => GameObjectType::AuctionHouse,
            21 => GameObjectType::GuardPost,
            22 => GameObjectType::SpellCaster,
            23 => GameObjectType::MeetingStone,
            24 => GameObjectType::FlagStand,
            25 => GameObjectType::FishingHole,
            26 => GameObjectType::FlagDrop,
            27 => GameObjectType::MiniGame,
            28 => GameObjectType::LotteryKiosk,
            29 => GameObjectType::CapturePoint,
            30 => GameObjectType::AuraGenerator,
            _ => GameObjectType::Generic,
        }
    }
}

/// GameObject state
/// Matches GOState from GameObjectDefines.h
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GOState {
    /// Active - show in world as used (e.g. open door)
    Active = 0,
    /// Ready - show in world as ready (e.g. closed door)
    Ready = 1,
    /// Active alternative (e.g. door opened by cannon fire)
    ActiveAlternative = 2,
}

impl From<u8> for GOState {
    fn from(value: u8) -> Self {
        match value {
            0 => GOState::Active,
            1 => GOState::Ready,
            2 => GOState::ActiveAlternative,
            _ => GOState::Ready,
        }
    }
}

/// Loot state for GameObjects
/// Matches LootState from GameObjectDefines.h
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LootState {
    /// Not ready - initializing
    NotReady = 0,
    /// Ready - can be activated
    Ready = 1,
    /// Activated - currently in use
    Activated = 2,
    /// Just deactivated - transitioning back
    JustDeactivated = 3,
}

impl From<u8> for LootState {
    fn from(value: u8) -> Self {
        match value {
            0 => LootState::NotReady,
            1 => LootState::Ready,
            2 => LootState::Activated,
            3 => LootState::JustDeactivated,
            _ => LootState::NotReady,
        }
    }
}

/// GameObject flags
pub mod go_flags {
    pub const GO_FLAG_IN_USE: u32 = 0x00000001;
    pub const GO_FLAG_LOCKED: u32 = 0x00000002;
    pub const GO_FLAG_INTERACT_COND: u32 = 0x00000004;
    pub const GO_FLAG_TRANSPORT: u32 = 0x00000008;
    pub const GO_FLAG_NO_INTERACT: u32 = 0x00000010;
    pub const GO_FLAG_NODESPAWN: u32 = 0x00000020;
    pub const GO_FLAG_TRIGGERED: u32 = 0x00000040;
}

/// GameObject dynamic flags (low byte)
pub mod go_dyn_flags {
    pub const GO_DYNFLAG_LO_ACTIVATE: u32 = 0x01;
    pub const GO_DYNFLAG_LO_ANIMATE: u32 = 0x02;
    pub const GO_DYNFLAG_LO_NO_INTERACT: u32 = 0x04;
}
