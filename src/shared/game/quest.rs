#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QuestStatus {
    None = 0,
    Available = 1,
    InProgress = 2,
    Failed = 3,
    Completed = 4,
    Expired = 5,
}

impl QuestStatus {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestGiverStatus {
    NoInteraction = 0,
    Unavailable = 1,
    Lazy = 2,
    QuestList = 3,
    InProgress = 4,
    OnMachineNoInteract = 5,
    OnMachineInteract = 6,
}

impl QuestGiverStatus {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestFlags(pub u32);

impl QuestFlags {
    pub const NONE: u32 = 0x00000000;
    pub const STAFF: u32 = 0x00000001;
    pub const ACCEPTCOND: u32 = 0x00000002;
    pub const REWARDOLD: u32 = 0x00000004;
    pub const EXCLUSIVE: u32 = 0x00000008;
    pub const DAILY: u32 = 0x00000010;
    pub const REPEATABLE: u32 = 0x00000020;
    pub const AUTO_ACCEPT: u32 = 0x00000040;
    pub const NEEDS_MORE_LOOT_MOBS: u32 = 0x00000100;
    pub const COMPLETED_WHEN_REACH_RELEVANT_LEVEL: u32 = 0x00000200;
    pub const AUTO_TAKE: u32 = 0x00000400;
    pub const UPDATE_PHASE_SHIFT: u32 = 0x00000800;
    pub const SOUND_TIMER: u32 = 0x00001000;
    pub const ACTIVE_HIGHLIGHT: u32 = 0x00002000;
    pub const DISABLE_COMPLETION_SOUND: u32 = 0x00004000;
    pub const LAUNCH_GOSSIP_COMPLETE: u32 = 0x00008000;
    pub const REMOVED_ON_LOGOUT: u32 = 0x00010000;
    pub const DISPLAY_HELP_IN_COMPLETION: u32 = 0x00020000;

    pub fn has_flag(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestShareState {
    Default = 0,
    Sent = 1,
    Accepted = 2,
    Declined = 3,
}
