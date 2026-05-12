//! Unit modifier system for stats
//!
//! Ported from server/src/world/game/stats/unit_mod.rs and base_mod.rs
//! Implements the 4-tier modifier formula: ((base_value * base_pct) + total_value) * total_pct

/// Modifier application types
/// Defines how modifiers are applied to base values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum UnitModifierType {
    /// Flat base value modifier (added to base before percentage)
    BaseValue = 0,
    /// Percentage multiplier for base value (multiplies base + BASE_VALUE)
    BasePct = 1,
    /// Flat total value modifier (added after base percentage)
    TotalValue = 2,
    /// Percentage multiplier for total value (final multiplier)
    TotalPct = 3,
}

impl UnitModifierType {
    pub const COUNT: usize = 4;
}

/// Unit modifier groups
/// Each group represents a different stat/resistance/power type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum UnitMods {
    // Stats (must be in order matching stat indices: 0-4)
    StatStrength = 0,
    StatAgility = 1,
    StatStamina = 2,
    StatIntellect = 3,
    StatSpirit = 4,

    // Health
    Health = 5,

    // Powers (must be in order matching power types)
    Mana = 6,
    Rage = 7,
    Focus = 8,
    Energy = 9,
    Happiness = 10,

    // Resistances (must be in order matching spell schools: 0-6)
    Armor = 11,
    ResistanceHoly = 12,
    ResistanceFire = 13,
    ResistanceNature = 14,
    ResistanceFrost = 15,
    ResistanceShadow = 16,
    ResistanceArcane = 17,

    // Damage modifiers
    DamageMainhand = 18,
    DamageOffhand = 19,
    DamageRanged = 20,
    DamagePhysical = 21,
}

impl UnitMods {
    pub const COUNT: usize = 22;

    /// Start of stat modifiers
    pub const STAT_START: u8 = UnitMods::StatStrength as u8;
    /// End of stat modifiers (exclusive)
    pub const STAT_END: u8 = UnitMods::StatSpirit as u8 + 1;

    /// Start of resistance modifiers
    pub const RESISTANCE_START: u8 = UnitMods::Armor as u8;

    /// Get UnitMod for a stat index (0=STR, 1=AGI, 2=STA, 3=INT, 4=SPI)
    pub fn from_stat(stat: u8) -> Option<Self> {
        match stat {
            0 => Some(Self::StatStrength),
            1 => Some(Self::StatAgility),
            2 => Some(Self::StatStamina),
            3 => Some(Self::StatIntellect),
            4 => Some(Self::StatSpirit),
            _ => None,
        }
    }

    /// Get UnitMod for a power type (0=mana, 1=rage, 2=focus, 3=energy, 4=happiness)
    pub fn from_power(power: u8) -> Option<Self> {
        match power {
            0 => Some(Self::Mana),
            1 => Some(Self::Rage),
            2 => Some(Self::Focus),
            3 => Some(Self::Energy),
            4 => Some(Self::Happiness),
            _ => None,
        }
    }

    /// Get UnitMod for a spell school resistance (0=physical/armor, 1=holy, ..., 6=arcane)
    pub fn from_resistance(school: u8) -> Option<Self> {
        match school {
            0 => Some(Self::Armor),
            1 => Some(Self::ResistanceHoly),
            2 => Some(Self::ResistanceFire),
            3 => Some(Self::ResistanceNature),
            4 => Some(Self::ResistanceFrost),
            5 => Some(Self::ResistanceShadow),
            6 => Some(Self::ResistanceArcane),
            _ => None,
        }
    }
}

/// Storage for unit modifiers
/// 2D array: [UnitMods][UnitModifierType]
/// Formula: ((base_value * base_pct) + total_value) * total_pct
#[derive(Debug, Clone)]
pub struct UnitModifierGroup {
    modifiers: [[f32; UnitModifierType::COUNT]; UnitMods::COUNT],
}

impl UnitModifierGroup {
    /// Create a new modifier group with default values
    /// BASE_VALUE and TOTAL_VALUE default to 0.0
    /// BASE_PCT and TOTAL_PCT default to 1.0
    pub fn new() -> Self {
        let mut modifiers = [[0.0f32; UnitModifierType::COUNT]; UnitMods::COUNT];

        // Initialize all BASE_PCT and TOTAL_PCT to 1.0
        for unit_mod in 0..UnitMods::COUNT {
            modifiers[unit_mod][UnitModifierType::BasePct as usize] = 1.0;
            modifiers[unit_mod][UnitModifierType::TotalPct as usize] = 1.0;
        }

        // Offhand damage modifier defaults to 50% (0.5)
        modifiers[UnitMods::DamageOffhand as usize][UnitModifierType::TotalPct as usize] = 0.5;

        Self { modifiers }
    }

    /// Get modifier value
    pub fn get_modifier_value(&self, unit_mod: UnitMods, modifier_type: UnitModifierType) -> f32 {
        let mod_idx = unit_mod as usize;
        let type_idx = modifier_type as usize;

        if mod_idx >= UnitMods::COUNT || type_idx >= UnitModifierType::COUNT {
            return 0.0;
        }

        let value = self.modifiers[mod_idx][type_idx];

        // TOTAL_PCT cannot be <= 0.0
        if modifier_type == UnitModifierType::TotalPct && value <= 0.0 {
            return 0.0;
        }

        value
    }

    /// Set modifier value directly
    pub fn set_modifier_value(
        &mut self,
        unit_mod: UnitMods,
        modifier_type: UnitModifierType,
        value: f32,
    ) {
        let mod_idx = unit_mod as usize;
        let type_idx = modifier_type as usize;

        if mod_idx < UnitMods::COUNT && type_idx < UnitModifierType::COUNT {
            self.modifiers[mod_idx][type_idx] = value;
        }
    }

    /// Handle stat modifier (add/subtract for flat values, apply percentage for PCT)
    pub fn handle_stat_modifier(
        &mut self,
        unit_mod: UnitMods,
        modifier_type: UnitModifierType,
        amount: f32,
        apply: bool,
    ) -> bool {
        let mod_idx = unit_mod as usize;
        let type_idx = modifier_type as usize;

        if mod_idx >= UnitMods::COUNT || type_idx >= UnitModifierType::COUNT {
            return false;
        }

        match modifier_type {
            UnitModifierType::BaseValue | UnitModifierType::TotalValue => {
                if apply {
                    self.modifiers[mod_idx][type_idx] += amount;
                } else {
                    self.modifiers[mod_idx][type_idx] -= amount;
                }
            }
            UnitModifierType::BasePct | UnitModifierType::TotalPct => {
                // Percentage modifiers: matches C++ ApplyPercentModFloatVar
                let mut amount = amount;
                if amount == -100.0 {
                    amount = -99.99;
                }

                if apply {
                    let multiplier = (100.0 + amount) / 100.0;
                    self.modifiers[mod_idx][type_idx] *= multiplier;
                } else {
                    let multiplier = (100.0 + amount) / 100.0;
                    if multiplier != 0.0 {
                        self.modifiers[mod_idx][type_idx] *= 100.0 / multiplier;
                    }
                }
            }
        }

        true
    }

    /// Calculate total value using the modifier formula
    /// Formula: ((base_value * base_pct) + total_value) * total_pct
    pub fn calculate_total_value(&self, unit_mod: UnitMods, base_value: f32) -> f32 {
        let base_val =
            self.get_modifier_value(unit_mod, UnitModifierType::BaseValue) + base_value;
        let base_pct = self.get_modifier_value(unit_mod, UnitModifierType::BasePct);
        let total_val = self.get_modifier_value(unit_mod, UnitModifierType::TotalValue);
        let total_pct = self.get_modifier_value(unit_mod, UnitModifierType::TotalPct);

        ((base_val * base_pct) + total_val) * total_pct
    }
}

impl Default for UnitModifierGroup {
    fn default() -> Self {
        Self::new()
    }
}

// === Base Modifier Group (for crit/dodge/block) ===

/// Base modifier groups
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BaseModGroup {
    CritPercentage = 0,
    RangedCritPercentage = 1,
    ShieldBlockValue = 2,
}

impl BaseModGroup {
    pub const COUNT: usize = 3;
}

/// Base modifier types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BaseModType {
    FlatMod = 0,
    PctMod = 1,
}

impl BaseModType {
    pub const COUNT: usize = 2;
}

/// Storage for base modifiers (crit, ranged crit, shield block)
/// Formula: FLAT_MOD * PCT_MOD
#[derive(Debug, Clone)]
pub struct BaseModifierGroup {
    modifiers: [[f32; BaseModType::COUNT]; BaseModGroup::COUNT],
}

impl BaseModifierGroup {
    pub fn new() -> Self {
        let mut modifiers = [[0.0f32; BaseModType::COUNT]; BaseModGroup::COUNT];

        // Initialize all PCT_MOD to 1.0
        for group in 0..BaseModGroup::COUNT {
            modifiers[group][BaseModType::PctMod as usize] = 1.0;
        }

        Self { modifiers }
    }

    pub fn get_modifier_value(&self, group: BaseModGroup, mod_type: BaseModType) -> f32 {
        let group_idx = group as usize;
        let type_idx = mod_type as usize;

        if group_idx >= BaseModGroup::COUNT || type_idx >= BaseModType::COUNT {
            return 0.0;
        }

        let value = self.modifiers[group_idx][type_idx];

        if mod_type == BaseModType::PctMod && value <= 0.0 {
            return 0.0;
        }

        value
    }

    pub fn handle_base_mod_value(
        &mut self,
        group: BaseModGroup,
        mod_type: BaseModType,
        amount: f32,
        apply: bool,
    ) -> bool {
        let group_idx = group as usize;
        let type_idx = mod_type as usize;

        if group_idx >= BaseModGroup::COUNT || type_idx >= BaseModType::COUNT {
            return false;
        }

        match mod_type {
            BaseModType::FlatMod => {
                if apply {
                    self.modifiers[group_idx][type_idx] += amount;
                } else {
                    self.modifiers[group_idx][type_idx] -= amount;
                }
            }
            BaseModType::PctMod => {
                let mut amount = amount;
                if amount == -100.0 {
                    amount = -99.99;
                }

                if apply {
                    let multiplier = (100.0 + amount) / 100.0;
                    self.modifiers[group_idx][type_idx] *= multiplier;
                } else {
                    let multiplier = (100.0 + amount) / 100.0;
                    if multiplier != 0.0 {
                        self.modifiers[group_idx][type_idx] *= 100.0 / multiplier;
                    }
                }
            }
        }

        true
    }

    /// Calculate total: FLAT_MOD * PCT_MOD
    pub fn get_total_base_mod_value(&self, group: BaseModGroup) -> f32 {
        let pct_mod = self.get_modifier_value(group, BaseModType::PctMod);
        if pct_mod <= 0.0 {
            return 0.0;
        }

        let flat_mod = self.get_modifier_value(group, BaseModType::FlatMod);
        flat_mod * pct_mod
    }
}

impl Default for BaseModifierGroup {
    fn default() -> Self {
        Self::new()
    }
}
