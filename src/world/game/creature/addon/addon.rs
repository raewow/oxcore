/// Creature addon - visual customization
#[derive(Debug, Clone, Default)]
pub struct CreatureAddon {
    /// Mount display ID (0 = no mount)
    pub mount: u32,
    /// Bytes1: stand state, pet talents, visibility, animation
    pub bytes1: u32,
    /// Bytes2: sheath state, flags, etc.
    pub bytes2: u32,
    /// Emote to play
    pub emote: u32,
    /// Auras to apply (space-separated spell IDs in DB)
    pub auras: Vec<u32>,
}

impl CreatureAddon {
    /// Get stand state from bytes1
    pub fn stand_state(&self) -> u8 {
        (self.bytes1 & 0xFF) as u8
    }

    /// Get sheath state from bytes2
    pub fn sheath_state(&self) -> u8 {
        (self.bytes2 & 0xFF) as u8
    }

    /// Check if has mount
    pub fn has_mount(&self) -> bool {
        self.mount != 0
    }

    /// Check if has emote
    pub fn has_emote(&self) -> bool {
        self.emote != 0
    }
}

/// Stand states
pub mod stand_state {
    pub const STAND: u8 = 0;
    pub const SIT: u8 = 1;
    pub const SIT_CHAIR: u8 = 2;
    pub const SLEEP: u8 = 3;
    pub const SIT_LOW_CHAIR: u8 = 4;
    pub const SIT_MEDIUM_CHAIR: u8 = 5;
    pub const SIT_HIGH_CHAIR: u8 = 6;
    pub const DEAD: u8 = 7;
    pub const KNEEL: u8 = 8;
    pub const SUBMERGED: u8 = 9;
}

/// Sheath states
pub mod sheath_state {
    pub const UNARMED: u8 = 0;
    pub const MELEE: u8 = 1;
    pub const RANGED: u8 = 2;
}
