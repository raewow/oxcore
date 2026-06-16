// Instance types and structures

/// Instance reset method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceResetMethod {
    Manual = 0,
    Expire = 1,
    Reset = 2,
}

/// Instance reset warning type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceResetWarningType {
    Hours1 = 1,
    Hours30Min = 2,
    Hours15Min = 3,
    Expired = 4,
}

/// Instance reset failure reason (sent to client)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InstanceResetFailReason {
    General = 0,
    Offline = 1,
    Zoning = 2,
    Silently = 3,
}

/// Instance state - represents a dungeon/raid instance
#[derive(Debug, Clone)]
pub struct InstanceState {
    pub map_id: u32,
    pub instance_id: u32,
    pub difficulty: u8,
    pub permanent: bool,
    pub reset_time: u64,
    pub created_time: u64,
    pub completed_encounters: Vec<u32>,
}

/// Instance binding for a player or group
#[derive(Debug, Clone)]
pub struct InstanceBinding {
    pub map_id: u32,
    pub instance_id: u32,
    pub permanent: bool,
    pub reset_time: u64,
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
    pub id: u32,
    pub name: String,
    pub is_completed: bool,
    pub completion_time: u64,
}
