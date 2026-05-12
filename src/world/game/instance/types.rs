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
    General = 0,  // Generic failure
    Offline = 1,  // Group member offline in instance
    Zoning = 2,   // Group member zoning into instance
    Silently = 3, // Silent failure (no message shown)
}

/// Instance state - represents a dungeon/raid instance
#[derive(Debug, Clone)]
pub struct InstanceState {
    pub map_id: u32,
    pub instance_id: u32,
    pub difficulty: u8, // 0 = normal, 1 = heroic (for dungeons)
    pub permanent: bool,
    pub reset_time: u64,                // Unix timestamp
    pub created_time: u64,              // Unix timestamp
    pub completed_encounters: Vec<u32>, // Encounter IDs that have been completed
}

/// Instance binding for a player or group
#[derive(Debug, Clone)]
pub struct InstanceBinding {
    pub map_id: u32,
    pub instance_id: u32,
    pub permanent: bool,
    pub reset_time: u64, // Unix timestamp
}
