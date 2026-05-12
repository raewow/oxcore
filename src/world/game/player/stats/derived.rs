//! Derived stat calculations (pure functions)
//!
//! All formulas ported from server/src/world/game/stats/ (health_mana.rs, attack_power.rs, crit_dodge.rs)
//! These are pure functions with no side effects.

// Class constants
const CLASS_WARRIOR: u8 = 1;
const CLASS_PALADIN: u8 = 2;
const CLASS_HUNTER: u8 = 3;
const CLASS_ROGUE: u8 = 4;
const CLASS_PRIEST: u8 = 5;
const CLASS_SHAMAN: u8 = 7;
const CLASS_MAGE: u8 = 8;
const CLASS_WARLOCK: u8 = 9;
const CLASS_DRUID: u8 = 11;

// === Health / Mana ===

/// Health bonus from stamina
/// First 20 stamina: 1 HP per point
/// Additional stamina: 10 HP per point
pub fn health_bonus_from_stamina(stamina: f32) -> f32 {
    let base_stam = if stamina < 20.0 { stamina } else { 20.0 };
    let more_stam = stamina - base_stam;
    base_stam + (more_stam * 10.0)
}

/// Mana bonus from intellect
/// First 20 intellect: 1 mana per point
/// Additional intellect: 15 mana per point
pub fn mana_bonus_from_intellect(intellect: f32) -> f32 {
    let base_int = if intellect < 20.0 { intellect } else { 20.0 };
    let more_int = intellect - base_int;
    base_int + (more_int * 15.0)
}

// === Attack Power ===

/// Calculate melee attack power from stats (class-specific)
pub fn calculate_melee_ap(class: u8, level: u8, strength: f32, agility: f32) -> f32 {
    let level = level as f32;
    match class {
        CLASS_WARRIOR | CLASS_PALADIN => level * 3.0 + strength * 2.0 - 20.0,
        CLASS_ROGUE | CLASS_HUNTER => level * 2.0 + strength + agility - 20.0,
        CLASS_SHAMAN => level * 2.0 + strength * 2.0 - 20.0,
        CLASS_DRUID => strength * 2.0 - 20.0,
        CLASS_MAGE | CLASS_PRIEST | CLASS_WARLOCK => strength - 10.0,
        _ => strength - 10.0,
    }
}

/// Calculate ranged attack power from stats (class-specific)
pub fn calculate_ranged_ap(class: u8, level: u8, agility: f32) -> f32 {
    let level = level as f32;
    match class {
        CLASS_HUNTER => level * 2.0 + agility * 2.0 - 10.0,
        CLASS_ROGUE | CLASS_WARRIOR => level + agility - 10.0,
        _ => agility - 10.0,
    }
}

/// Get AP damage modifier: AP / 14.0 * weapon_speed_seconds
pub fn ap_damage_modifier(attack_power: f32, weapon_speed_ms: u32) -> f32 {
    let speed_seconds = weapon_speed_ms as f32 / 1000.0;
    (attack_power / 14.0) * speed_seconds
}

// === Crit ===

/// Linear interpolation helper for agility-based ratings
fn interpolate_rate(class_rates: (f32, f32), level: f32) -> f32 {
    let (val_l1, val_l60) = class_rates;
    if level <= 1.0 {
        val_l1
    } else if level >= 60.0 {
        val_l60
    } else {
        val_l1 * (60.0 - level) / 59.0 + val_l60 * (level - 1.0) / 59.0
    }
}

/// Melee crit percentage from agility
pub fn melee_crit_from_agility(class: u8, level: u8, agility: f32) -> f32 {
    let rates = match class {
        CLASS_PALADIN | CLASS_SHAMAN | CLASS_DRUID => (4.6, 20.0),
        CLASS_MAGE => (12.9, 20.0),
        CLASS_ROGUE => (2.2, 29.0),
        CLASS_HUNTER => (3.5, 53.0),
        CLASS_PRIEST => (11.0, 20.0),
        CLASS_WARLOCK => (8.4, 20.0),
        CLASS_WARRIOR => (3.9, 20.0),
        _ => return 0.0,
    };

    let class_rate = interpolate_rate(rates, level as f32);
    if class_rate <= 0.0 {
        return 0.0;
    }
    agility / class_rate
}

/// Ranged crit percentage from agility (only hunter in vanilla)
pub fn ranged_crit_from_agility(class: u8, level: u8, agility: f32) -> f32 {
    let rates = match class {
        CLASS_HUNTER => (3.5, 53.0),
        _ => return 0.0,
    };

    let class_rate = interpolate_rate(rates, level as f32);
    if class_rate <= 0.0 {
        return 0.0;
    }
    agility / class_rate
}

/// Class base melee crit percentage (inherent before agility)
pub fn class_base_crit(class: u8) -> f32 {
    match class {
        CLASS_DRUID => 0.9,
        CLASS_MAGE => 3.2,
        CLASS_PALADIN => 0.7,
        CLASS_PRIEST => 3.0,
        CLASS_SHAMAN => 1.7,
        CLASS_WARLOCK => 2.0,
        _ => 0.0, // Warrior, Rogue, Hunter
    }
}

// === Spell Crit ===

/// Spell crit percentage from intellect (class-specific)
/// In vanilla, spell crit scales from intellect similarly to melee crit from agility
pub fn spell_crit_from_intellect(class: u8, level: u8, intellect: f32) -> f32 {
    let rates = match class {
        CLASS_MAGE => (11.35, 59.5),
        CLASS_PRIEST => (12.5, 59.56),
        CLASS_WARLOCK => (8.085, 60.6),
        CLASS_DRUID => (12.5, 60.0),
        CLASS_SHAMAN => (11.55, 60.0),
        CLASS_PALADIN => (12.5, 54.0),
        _ => return 0.0, // Non-caster classes don't get spell crit from int
    };

    let class_rate = interpolate_rate(rates, level as f32);
    if class_rate <= 0.0 {
        return 0.0;
    }
    intellect / class_rate
}

/// Class base spell crit percentage (inherent before intellect)
pub fn class_base_spell_crit(class: u8) -> f32 {
    match class {
        CLASS_MAGE => 0.2,
        CLASS_PRIEST => 0.8,
        CLASS_WARLOCK => 1.7,
        CLASS_DRUID => 1.8,
        CLASS_SHAMAN => 2.2,
        CLASS_PALADIN => 3.3,
        _ => 0.0,
    }
}

// === Dodge ===

/// Dodge percentage from agility
pub fn dodge_from_agility(class: u8, level: u8, agility: f32) -> f32 {
    let rates = match class {
        CLASS_PALADIN | CLASS_SHAMAN | CLASS_DRUID => (4.6, 20.0),
        CLASS_MAGE => (12.9, 20.0),
        CLASS_ROGUE => (1.1, 14.5),
        CLASS_HUNTER => (1.8, 26.5),
        CLASS_PRIEST => (11.0, 20.0),
        CLASS_WARLOCK => (8.4, 20.0),
        CLASS_WARRIOR => (3.9, 20.0),
        _ => return 0.0,
    };

    let class_rate = interpolate_rate(rates, level as f32);
    if class_rate <= 0.0 {
        return 0.0;
    }
    agility / class_rate
}

/// Class base dodge percentage
pub fn class_base_dodge(class: u8) -> f32 {
    // Same values as base crit in vanilla
    class_base_crit(class)
}

// === Armor ===

/// Armor bonus from agility (2 armor per point)
pub fn armor_from_agility(agility: f32) -> f32 {
    agility * 2.0
}

// === Mana Regen ===

/// Mana regen per 5 seconds from spirit (class-specific)
pub fn mana_regen_from_spirit(class: u8, spirit: f32) -> f32 {
    match class {
        CLASS_MAGE | CLASS_PRIEST | CLASS_WARLOCK => (spirit / 4.0 + 12.5).max(0.0),
        CLASS_DRUID | CLASS_HUNTER | CLASS_PALADIN | CLASS_SHAMAN | CLASS_ROGUE => {
            (spirit / 5.0 + 15.0).max(0.0)
        }
        CLASS_WARRIOR => (spirit * 1.26 - 22.6).max(0.0),
        _ => 0.0,
    }
}

// === Power Type ===

/// Get power type for class (0=mana, 1=rage, 3=energy)
pub fn power_type_for_class(class: u8) -> u8 {
    match class {
        CLASS_WARRIOR => 1, // Rage
        CLASS_ROGUE => 3,   // Energy
        _ => 0,             // Mana
    }
}

/// Get default max power for non-mana power types
pub fn base_max_power(power_type: u8) -> u32 {
    match power_type {
        1 => 1000, // Rage (displayed as 100 in client, stored as 1000)
        3 => 100,  // Energy
        _ => 0,    // Mana comes from intellect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_from_stamina() {
        assert_eq!(health_bonus_from_stamina(10.0), 10.0);
        assert_eq!(health_bonus_from_stamina(20.0), 20.0);
        assert_eq!(health_bonus_from_stamina(30.0), 120.0); // 20 + 10*10
        assert_eq!(health_bonus_from_stamina(50.0), 320.0); // 20 + 30*10
    }

    #[test]
    fn test_mana_from_intellect() {
        assert_eq!(mana_bonus_from_intellect(10.0), 10.0);
        assert_eq!(mana_bonus_from_intellect(20.0), 20.0);
        assert_eq!(mana_bonus_from_intellect(30.0), 170.0); // 20 + 10*15
    }

    #[test]
    fn test_melee_ap_warrior() {
        // Level 1 warrior with 20 STR: 1*3 + 20*2 - 20 = 23
        assert_eq!(calculate_melee_ap(1, 1, 20.0, 10.0), 23.0);
        // Level 60 warrior with 100 STR: 60*3 + 100*2 - 20 = 360
        assert_eq!(calculate_melee_ap(1, 60, 100.0, 50.0), 360.0);
    }

    #[test]
    fn test_melee_ap_rogue() {
        // Level 1 rogue with 15 STR, 20 AGI: 1*2 + 15 + 20 - 20 = 17
        assert_eq!(calculate_melee_ap(4, 1, 15.0, 20.0), 17.0);
    }

    #[test]
    fn test_power_type() {
        assert_eq!(power_type_for_class(1), 1); // Warrior = rage
        assert_eq!(power_type_for_class(4), 3); // Rogue = energy
        assert_eq!(power_type_for_class(8), 0); // Mage = mana
    }
}
