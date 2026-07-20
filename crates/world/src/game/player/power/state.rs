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

    /// Flat health regeneration from auras, in health per five seconds.
    pub health_regen_per_5: f32,

    /// Combined multiplier from health-regen-percent auras.
    pub health_regen_multiplier: f32,

    /// Percentage of spirit regen that works while casting
    /// From talents/auras like Meditation, Arcane Meditation
    pub casting_regen_pct: f32,

    /// Fractional health regen carried between ticks (Player::m_carryHealthRegen).
    /// Accumulates sub-integer regen so nothing is silently lost.
    pub carry_health_regen: f32,
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
            health_regen_per_5: 0.0,
            health_regen_multiplier: 1.0,
            casting_regen_pct: 0.0,
            carry_health_regen: 0.0,
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

    /// Add `delta` to current power, clamping to [0, max]. Returns the actual gain (negative = loss).
    /// Matches C++ Unit::ModifyPower — callers use the return value to decide whether to broadcast.
    pub fn modify_power(&mut self, power_type: PowerType, delta: i32) -> i32 {
        if delta == 0 {
            return 0;
        }
        let idx = power_type as usize;
        let current = self.current[idx] as i32;
        let val = current + delta;
        if val <= 0 {
            self.current[idx] = 0;
            return -current;
        }
        let max = self.max[idx] as i32;
        if val < max {
            self.current[idx] = val as u32;
            val - current
        } else if current != max {
            self.current[idx] = max as u32;
            max - current
        } else {
            0
        }
    }

    /// Switch the active power type (shapeshift / form change).
    /// Resets current and max power for the new type per C++ Unit::SetPowerType.
    /// The caller (system layer) is responsible for broadcasting UNIT_FIELD_BYTES_0.
    pub fn set_power_type(&mut self, new_type: PowerType) {
        use super::regen::{MAX_ENERGY, MAX_FOCUS, MAX_HAPPINESS, MAX_RAGE};
        self.power_type = new_type;
        match new_type {
            PowerType::Mana => {
                // Mana max/current are managed by StatsSystem — no reset here.
            }
            PowerType::Rage => {
                self.max[PowerType::Rage as usize] = MAX_RAGE;
                self.current[PowerType::Rage as usize] = 0;
            }
            PowerType::Focus => {
                self.max[PowerType::Focus as usize] = MAX_FOCUS;
                self.current[PowerType::Focus as usize] = MAX_FOCUS;
            }
            PowerType::Energy => {
                self.max[PowerType::Energy as usize] = MAX_ENERGY;
                self.current[PowerType::Energy as usize] = 0;
            }
            PowerType::Happiness => {
                self.max[PowerType::Happiness as usize] = MAX_HAPPINESS;
                self.current[PowerType::Happiness as usize] = MAX_HAPPINESS;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PowerState, PowerType};
    use crate::game::player::power::regen::{MAX_ENERGY, MAX_FOCUS, MAX_HAPPINESS, MAX_RAGE};

    fn state(power_type: PowerType, current: u32, max: u32) -> PowerState {
        let mut state = PowerState::default();
        let idx = power_type as usize;
        state.current[idx] = current;
        state.max[idx] = max;
        state
    }

    #[test]
    fn modify_power_zero_delta_is_a_noop() {
        let mut state = state(PowerType::Mana, 50, 100);

        assert_eq!(state.modify_power(PowerType::Mana, 0), 0);
        assert_eq!(state.get_current(PowerType::Mana), 50);
    }

    #[test]
    fn modify_power_applies_in_range_gain_and_loss() {
        let mut state = state(PowerType::Mana, 50, 100);

        assert_eq!(state.modify_power(PowerType::Mana, 20), 20);
        assert_eq!(state.get_current(PowerType::Mana), 70);
        assert_eq!(state.modify_power(PowerType::Mana, -30), -30);
        assert_eq!(state.get_current(PowerType::Mana), 40);
    }

    #[test]
    fn modify_power_depletes_at_zero_and_returns_actual_loss() {
        let mut state = state(PowerType::Rage, 50, 100);

        assert_eq!(state.modify_power(PowerType::Rage, -50), -50);
        assert_eq!(state.get_current(PowerType::Rage), 0);

        state.current[PowerType::Rage as usize] = 50;
        assert_eq!(state.modify_power(PowerType::Rage, -75), -50);
        assert_eq!(state.get_current(PowerType::Rage), 0);
    }

    #[test]
    fn modify_power_caps_at_max_and_is_a_noop_when_full() {
        let mut state = state(PowerType::Energy, 70, 100);

        assert_eq!(state.modify_power(PowerType::Energy, 30), 30);
        assert_eq!(state.get_current(PowerType::Energy), 100);
        assert_eq!(state.modify_power(PowerType::Energy, 50), 0);
        assert_eq!(state.get_current(PowerType::Energy), 100);
    }

    #[test]
    fn set_power_type_resets_rage_and_energy() {
        let mut state = PowerState::default();
        state.current[PowerType::Rage as usize] = 300;
        state.current[PowerType::Energy as usize] = 70;

        state.set_power_type(PowerType::Rage);
        assert_eq!(state.power_type, PowerType::Rage);
        assert_eq!(state.get_max(PowerType::Rage), MAX_RAGE);
        assert_eq!(state.get_current(PowerType::Rage), 0);

        state.set_power_type(PowerType::Energy);
        assert_eq!(state.power_type, PowerType::Energy);
        assert_eq!(state.get_max(PowerType::Energy), MAX_ENERGY);
        assert_eq!(state.get_current(PowerType::Energy), 0);
    }

    #[test]
    fn set_power_type_fills_focus_and_happiness() {
        let mut state = PowerState::default();

        state.set_power_type(PowerType::Focus);
        assert_eq!(state.get_max(PowerType::Focus), MAX_FOCUS);
        assert_eq!(state.get_current(PowerType::Focus), MAX_FOCUS);

        state.set_power_type(PowerType::Happiness);
        assert_eq!(state.get_max(PowerType::Happiness), MAX_HAPPINESS);
        assert_eq!(state.get_current(PowerType::Happiness), MAX_HAPPINESS);
    }

    #[test]
    fn set_power_type_preserves_mana_values() {
        let mut state = PowerState::default();
        state.max[PowerType::Mana as usize] = 500;
        state.current[PowerType::Mana as usize] = 250;

        state.set_power_type(PowerType::Mana);

        assert_eq!(state.power_type, PowerType::Mana);
        assert_eq!(state.get_max(PowerType::Mana), 500);
        assert_eq!(state.get_current(PowerType::Mana), 250);
    }
}
