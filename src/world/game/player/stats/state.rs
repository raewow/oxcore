//! Per-player stats state
//!
//! Embedded in the Player struct. Contains final computed values,
//! base values from DB, and modifier groups for the 4-tier formula.

use super::modifiers::{BaseModifierGroup, UnitModifierGroup};

/// Per-player stats state
#[derive(Debug, Clone)]
pub struct StatsState {
    // === Final computed values (sent to client) ===
    pub strength: u32,
    pub agility: u32,
    pub stamina: u32,
    pub intellect: u32,
    pub spirit: u32,

    pub health: u32,
    pub max_health: u32,
    pub mana: u32,
    pub max_mana: u32,

    // === Base values from DB (race/class/level) ===
    pub base_health: u32,
    pub base_mana: u32,

    // === Modifier groups ===
    pub unit_mods: UnitModifierGroup,
    pub base_mods: BaseModifierGroup,

    // === Derived combat stats ===
    pub melee_attack_power: i32,
    pub ranged_attack_power: i32,
    pub armor: u32,
    pub resistances: [u32; 7], // Physical(armor), Holy, Fire, Nature, Frost, Shadow, Arcane

    pub melee_crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub spell_crit_pct: f32,
    pub dodge_pct: f32,
    pub parry_pct: f32,
    pub block_pct: f32,

    /// Spell power per school [Physical, Holy, Fire, Nature, Frost, Shadow, Arcane]
    /// Computed from gear + auras. Used for spell damage/healing scaling.
    pub spell_power: [u32; 7],
    /// Healing power bonus (separate from spell power in vanilla)
    pub healing_power: u32,

    pub min_damage: f32,
    pub max_damage: f32,
    pub min_offhand_damage: f32,
    pub max_offhand_damage: f32,
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,

    pub mana_regen_base: f32,
    pub mana_regen_interrupt: f32,

    /// Set when stats need to be broadcast to client
    pub dirty: bool,
}

impl Default for StatsState {
    fn default() -> Self {
        Self {
            strength: 0,
            agility: 0,
            stamina: 0,
            intellect: 0,
            spirit: 0,

            health: 1,
            max_health: 1,
            mana: 0,
            max_mana: 0,

            base_health: 0,
            base_mana: 0,

            unit_mods: UnitModifierGroup::new(),
            base_mods: BaseModifierGroup::new(),

            melee_attack_power: 0,
            ranged_attack_power: 0,
            armor: 0,
            resistances: [0; 7],

            melee_crit_pct: 0.0,
            ranged_crit_pct: 0.0,
            spell_crit_pct: 0.0,
            dodge_pct: 0.0,
            parry_pct: 0.0,
            block_pct: 0.0,

            spell_power: [0; 7],
            healing_power: 0,

            min_damage: 0.0,
            max_damage: 0.0,
            min_offhand_damage: 0.0,
            max_offhand_damage: 0.0,
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,

            mana_regen_base: 0.0,
            mana_regen_interrupt: 0.0,

            dirty: true,
        }
    }
}
