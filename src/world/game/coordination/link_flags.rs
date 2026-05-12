use crate::shared::protocol::ObjectGuid;

bitflags::bitflags! {
    /// Creature linking flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LinkFlags: u32 {
        /// Master->Slave: when master aggros, slave aggros same target
        const AGGRO_ON_AGGRO = 0x0001;
        /// Slave->Master: when slave aggros, master aggros same target
        const TO_AGGRO_ON_AGGRO = 0x0002;
        /// When master respawns, slave respawns
        const RESPAWN_ON_RESPAWN = 0x0004;
        /// When slave respawns, master respawns
        const TO_RESPAWN_ON_RESPAWN = 0x0008;
        /// When master dies, slave dies
        const DIE_ON_MASTER_DEATH = 0x0010;
        /// When master evades, slave evades
        const EVADE_ON_MASTER_EVADE = 0x0020;
        /// Ignore if master is dead
        const NO_DEAD_MASTER = 0x0040;
        /// Use distance check instead of instance-wide
        const CHECK_DISTANCE = 0x0080;
    }
}

/// Link event types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkEvent {
    Aggro { target: ObjectGuid },
    Death,
    Respawn,
    Evade,
    EnterCombat { target: ObjectGuid },
    LeaveCombat,
}
