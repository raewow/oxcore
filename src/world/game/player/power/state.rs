//! Per-player power state
//!
//! Embedded in the Player struct. Contains current/max power values
//! and regeneration state (5-second rule, etc.).

/// Power type for a unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerType {
    Mana = 0,
    Rage = 1,
    Focus = 2,
    Energy = 3,
    Happiness = 4,
}

impl PowerType {
    /// Get primary power type for a class
    pub fn for_class(class: u8) -> Self {
        match class {
            1 => PowerType::Rage,   // Warrior
            4 => PowerType::Energy, // Rogue
            _ => PowerType::Mana,   // All other classes
        }
    }

    /// Get power type from u8 value
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PowerType::Mana),
            1 => Some(PowerType::Rage),
            2 => Some(PowerType::Focus),
            3 => Some(PowerType::Energy),
            4 => Some(PowerType::Happiness),
            _ => None,
        }
    }

    /// Get u8 value for power type
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Per-player power state
#[derive(Debug, Clone)]
pub struct PowerState {
    /// Primary power type for this player's class
    pub power_type: PowerType,

    /// Current power values (indexed by PowerType)
    pub current: [u32; 5],

    /// Max power values (indexed by PowerType)
    pub max: [u32; 5],

    /// Regen accumulator (fractional regen carried between ticks)
    pub regen_accumulator: f32,

    /// Timestamp of last spell cast (for 5-second rule)
    /// Set to current time when any mana-costing spell finishes casting
    pub last_mana_use_time: u64,

    /// Whether spirit-based regen is active (5-second rule)
    /// True = 5 seconds have passed since last mana use
    pub spirit_regen_active: bool,

    /// Eating/drinking state (from auras)
    pub is_eating: bool,
    pub is_drinking: bool,

    /// MP5 from gear (mana per 5 seconds, ignores 5-second rule)
    pub mp5_from_gear: f32,

    /// Percentage of spirit regen that works while casting
    /// From talents/auras like Meditation, Arcane Meditation
    pub casting_regen_pct: f32,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            power_type: PowerType::Mana,
            current: [0; 5],
            max: [0; 5],
            regen_accumulator: 0.0,
            last_mana_use_time: 0,
            spirit_regen_active: true,
            is_eating: false,
            is_drinking: false,
            mp5_from_gear: 0.0,
            casting_regen_pct: 0.0,
        }
    }
}

impl PowerState {
    /// Get current power for a specific type
    pub fn get_current(&self, power_type: PowerType) -> u32 {
        self.current[power_type as usize]
    }

    /// Get max power for a specific type
    pub fn get_max(&self, power_type: PowerType) -> u32 {
        self.max[power_type as usize]
    }

    /// Set current power for a specific type (clamped to max)
    pub fn set_current(&mut self, power_type: PowerType, value: u32) {
        let max = self.max[power_type as usize];
        self.current[power_type as usize] = value.min(max);
    }

    /// Set max power for a specific type
    pub fn set_max(&mut self, power_type: PowerType, value: u32) {
        self.max[power_type as usize] = value;
        // Clamp current if it exceeds new max
        if self.current[power_type as usize] > value {
            self.current[power_type as usize] = value;
        }
    }

    /// Modify power by a delta (can be negative)
    pub fn modify(&mut self, power_type: PowerType, delta: i32) {
        let current = self.current[power_type as usize] as i32;
        let max = self.max[power_type as usize] as i32;
        let new_value = (current + delta).max(0).min(max) as u32;
        self.current[power_type as usize] = new_value;
    }

    /// Check if player has enough power
    pub fn has_enough(&self, power_type: PowerType, amount: u32) -> bool {
        self.current[power_type as usize] >= amount
    }

    /// Consume power (returns false if not enough)
    pub fn consume(&mut self, power_type: PowerType, amount: u32) -> bool {
        if self.has_enough(power_type, amount) {
            self.current[power_type as usize] -= amount;
            true
        } else {
            false
        }
    }

    /// Restore power (capped at max)
    pub fn restore(&mut self, power_type: PowerType, amount: u32) {
        let new_value = self.current[power_type as usize] + amount;
        self.current[power_type as usize] = new_value.min(self.max[power_type as usize]);
    }

    /// Get max mana (convenience method)
    pub fn max_mana(&self) -> u32 {
        self.get_max(PowerType::Mana)
    }

    /// Set mana (convenience method)
    pub fn set_mana(&mut self, value: u32) {
        self.set_current(PowerType::Mana, value);
    }
}
