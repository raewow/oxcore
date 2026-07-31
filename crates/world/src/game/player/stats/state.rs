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
    pub block_value: u32,

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

    /// Full mana regeneration rate in mana per second.
    pub mana_regen_base: f32,
    /// Mana regeneration rate while inside the 5-second rule, in mana per second.
    pub mana_regen_interrupt: f32,

    /// Set when stats need to be broadcast to client
    pub dirty: bool,
}

impl StatsState {
    /// Add `delta` to current health, clamping to [0, max_health]. Returns actual gain (negative = loss).
    /// Health-modify semantics — callers use the return value to decide broadcasts / death checks.
    pub fn modify_health(&mut self, delta: i32) -> i32 {
        if delta == 0 {
            return 0;
        }
        let current = self.health as i32;
        let val = current + delta;
        if val <= 0 {
            self.health = 0;
            return -current;
        }
        let max = self.max_health as i32;
        if val < max {
            self.health = val as u32;
            val - current
        } else if current != max {
            self.health = max as u32;
            max - current
        } else {
            0
        }
    }
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
            block_value: 0,

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

#[cfg(test)]
mod tests {
    use super::StatsState;

    fn stats_with_health(health: u32, max_health: u32) -> StatsState {
        StatsState {
            health,
            max_health,
            ..Default::default()
        }
    }

    #[test]
    fn modify_health_returns_zero_without_a_change() {
        let mut stats = stats_with_health(40, 100);

        assert_eq!(stats.modify_health(0), 0);
        assert_eq!(stats.health, 40);
    }

    #[test]
    fn modify_health_returns_actual_loss_when_depleted() {
        let mut stats = stats_with_health(40, 100);

        assert_eq!(stats.modify_health(-50), -40);
        assert_eq!(stats.health, 0);
    }

    #[test]
    fn modify_health_applies_in_range_delta() {
        let mut stats = stats_with_health(40, 100);

        assert_eq!(stats.modify_health(15), 15);
        assert_eq!(stats.health, 55);
        assert_eq!(stats.modify_health(-20), -20);
        assert_eq!(stats.health, 35);
    }

    #[test]
    fn modify_health_caps_gain_and_ignores_healing_at_full_health() {
        let mut stats = stats_with_health(90, 100);

        assert_eq!(stats.modify_health(50), 10);
        assert_eq!(stats.health, 100);
        assert_eq!(stats.modify_health(1), 0);
        assert_eq!(stats.health, 100);
    }
}
