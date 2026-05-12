#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootMethod {
    FreeForAll = 0,
    GroupLoot = 1,
    NeedBeforeGreed = 2,
    MasterLoot = 3,
    RoundRobin = 4,
}

impl LootMethod {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LootThreshold {
    Threshold2 = 2, // Uncommon (green)
    Threshold3 = 3, // Rare (blue)
    Threshold4 = 4, // Epic (purple)
    Threshold5 = 5, // Legendary (orange)
}

impl LootThreshold {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GroupLootType {
    Normal = 0,
    Ffa = 1,
    RoundRobin = 2,
    MasterLoot = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Offline = 0,
    Online = 1,
    LeftGroup = 2,
    Unknown = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GroupUpdateFlags {
    None = 0,
    Guild = 1,
    Status = 2,
    CurHp = 4,
    CurMana = 8,
    Level = 16,
    ZoneId = 32,
    Position = 64,
    Auras = 128,
    Health = 256,
    Mana = 512,
    Power = 1024,
    MaxHealth = 2048,
    MaxMana = 4096,
    Dominating = 8192,
    TargetGuid = 16384,
    DamageDone = 32768,
    DamageDoneMeleepct = 65536,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitedBy {
    None = 0,
    Player = 1,
    Lfg = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubGroup {
    Main = 0,
    First = 1,
    Second = 2,
    Third = 3,
    Fourth = 4,
    Fifth = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupType {
    Normal = 0,
    Battleground = 1,
    Raid = 2,
    Lfg = 4,
}

#[derive(Debug, Clone)]
pub struct GroupMember {
    pub guid: crate::shared::protocol::ObjectGuid,
    pub name: String,
    pub class: u8,
    pub race: u8,
    pub level: u8,
    pub area: u32,
    pub status: MemberStatus,
    pub subgroup: SubGroup,
    pub is_assistant: bool,
    pub is_main_assistant: bool,
    pub is_ml: bool,
}

impl GroupMember {
    pub fn new(guid: crate::shared::protocol::ObjectGuid) -> Self {
        Self {
            guid,
            name: String::new(),
            class: 0,
            race: 0,
            level: 0,
            area: 0,
            status: MemberStatus::Offline,
            subgroup: SubGroup::Main,
            is_assistant: false,
            is_main_assistant: false,
            is_ml: false,
        }
    }
}
