#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InstanceResetFailReason {
    PlayerNotInInstance = 0,
    MapNotFound = 1,
    NoValidSocket = 2,
    PlayerAlive = 3,
    PlayerInCombat = 4,
    GuildChallengeActive = 5,
    GuildChallengeComplete = 6,
}

impl InstanceResetFailReason {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InstanceResetWarningType {
    InitialCD = 0,
    WipeCD = 1,
    InitialWipeCD = 2,
    EncounterWipeCD = 3,
    LongCD = 4,
    ResetNow = 5,
}

impl InstanceResetWarningType {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone)]
pub struct InstanceBind {
    pub map_id: u32,
    pub difficulty: u32,
    pub reset_time: u32,
    pub max_reset_time: u32,
    pub completed_encounters: u32,
    pub is_persistent: bool,
}

impl InstanceBind {
    pub fn new(map_id: u32, difficulty: u32) -> Self {
        Self {
            map_id,
            difficulty,
            reset_time: 0,
            max_reset_time: 0,
            completed_encounters: 0,
            is_persistent: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstanceSave {
    pub map_id: u32,
    pub difficulty: u32,
    pub boss_encounters: Vec<BossEncounter>,
    pub reset_time: u32,
    pub max_reset_time: u32,
}

impl InstanceSave {
    pub fn new(map_id: u32, difficulty: u32) -> Self {
        Self {
            map_id,
            difficulty,
            boss_encounters: Vec::new(),
            reset_time: 0,
            max_reset_time: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BossEncounter {
    pub credit_type: u32,
    pub credit_entry: u32,
    pub last_enemy_guid: crate::shared::protocol::ObjectGuid,
    pub progress: u32,
    pub completed_encounter: bool,
}

impl BossEncounter {
    pub fn new(credit_entry: u32) -> Self {
        Self {
            credit_type: 0,
            credit_entry,
            last_enemy_guid: crate::shared::protocol::ObjectGuid::empty(),
            progress: 0,
            completed_encounter: false,
        }
    }
}
