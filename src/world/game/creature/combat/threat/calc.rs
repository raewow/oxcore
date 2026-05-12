//! Threat Calculation Helper
//!
//! Calculates final threat with all modifiers:
//! - Base threat amount
//! - Critical strike multipliers
//! - School-based threat modifiers
//! - Spell attribute checks (ATTRIBUTES_EX, ATTRIBUTES_EX2, ATTRIBUTES_EX4)
//! - Aura-based threat modifiers from caster
//! - Aura-based critical threat modifiers from caster

/// Spell school mask constants
pub const SPELL_SCHOOL_MASK_NORMAL: u32 = 0x01;
pub const SPELL_SCHOOL_MASK_HOLY: u32 = 0x02;
pub const SPELL_SCHOOL_MASK_FIRE: u32 = 0x04;
pub const SPELL_SCHOOL_MASK_NATURE: u32 = 0x08;
pub const SPELL_SCHOOL_MASK_FROST: u32 = 0x10;
pub const SPELL_SCHOOL_MASK_SHADOW: u32 = 0x20;
pub const SPELL_SCHOOL_MASK_ARCANE: u32 = 0x40;

/// Spell attribute flags (simplified)
pub const SPELL_ATTR_EX_NO_THREAT: u32 = 0x00020000;
pub const SPELL_ATTR_EX2_NO_INITIAL_AGGRO: u32 = 0x00002000;
pub const SPELL_ATTR_EX4_NO_HARMFUL_THREAT: u32 = 0x01000000;

/// Threat calculation helper
pub struct ThreatCalcHelper;

impl ThreatCalcHelper {
    /// Calculate final threat with all modifiers
    ///
    /// Takes into account:
    /// - Base threat amount
    /// - Critical strike multipliers
    /// - School-based threat modifiers
    /// - Spell attribute checks
    /// - Aura-based threat modifiers from caster
    /// - Aura-based critical threat modifiers from caster
    pub fn calc_threat(
        base_threat: f32,
        is_crit: bool,
        school_mask: u32,
        spell_id: Option<u32>,
        spell_attributes_ex: Option<u32>,
        spell_attributes_ex2: Option<u32>,
        spell_attributes_ex4: Option<u32>,
        aura_modifier: Option<f32>,
        crit_aura_modifier: Option<f32>,
    ) -> f32 {
        let mut threat = base_threat;

        // Check spell attributes for threat modifiers
        if let Some(_) = spell_id {
            // SPELL_ATTR_EX_NO_THREAT - spell generates no threat
            if let Some(attr_ex) = spell_attributes_ex {
                if attr_ex & SPELL_ATTR_EX_NO_THREAT != 0 {
                    return 0.0;
                }
            }

            // SPELL_ATTR_EX2_NO_INITIAL_AGGRO - doesn't pull aggro
            if let Some(attr_ex2) = spell_attributes_ex2 {
                if attr_ex2 & SPELL_ATTR_EX2_NO_INITIAL_AGGRO != 0 {
                    // Reduces threat significantly but doesn't zero it
                    threat *= 0.1;
                }
            }

            // SPELL_ATTR_EX4_NO_HARMFUL_THREAT - no threat from hostile spells
            if let Some(attr_ex4) = spell_attributes_ex4 {
                if attr_ex4 & SPELL_ATTR_EX4_NO_HARMFUL_THREAT != 0 {
                    threat = 0.0;
                }
            }
        }

        // Apply critical strike multiplier
        if is_crit {
            let crit_modifier = crit_aura_modifier.unwrap_or(2.0); // Default 2x for crits
            threat *= crit_modifier;
        }

        // Apply school-based modifiers (e.g., Nature threat reduction)
        threat *= Self::get_school_threat_modifier(school_mask);

        // Apply aura modifiers from caster (e.g., Blessing of Salvation, Righteous Fury)
        if let Some(modifier) = aura_modifier {
            threat *= modifier;
        }

        threat
    }

    /// Simplified threat calculation for damage
    pub fn calc_damage_threat(
        damage: u32,
        threat_modifier: f32,
        is_crit: bool,
        crit_aura_modifier: Option<f32>,
    ) -> f32 {
        let mut threat = damage as f32 * threat_modifier;

        if is_crit {
            let crit_mod = crit_aura_modifier.unwrap_or(2.0);
            threat *= crit_mod;
        }

        threat
    }

    /// Calculate threat from healing done
    ///
    /// Healing threat is typically 50% of healing amount, but can be modified by:
    /// - Caster's threat reduction auras (e.g., Tranquil Air Totem)
    /// - Spell-specific modifiers
    pub fn calc_heal_threat(
        heal_amount: u32,
        spell_id: Option<u32>,
        spell_attributes_ex: Option<u32>,
        spell_attributes_ex2: Option<u32>,
        spell_attributes_ex4: Option<u32>,
        aura_modifier: Option<f32>,
    ) -> f32 {
        let base_threat = heal_amount as f32 * 0.5;

        Self::calc_threat(
            base_threat,
            false, // healing doesn't crit for threat
            SPELL_SCHOOL_MASK_NORMAL,
            spell_id,
            spell_attributes_ex,
            spell_attributes_ex2,
            spell_attributes_ex4,
            aura_modifier,
            None, // no crit modifier for healing
        )
    }

    /// Get threat modifier for spell school
    fn get_school_threat_modifier(school_mask: u32) -> f32 {
        // Most schools have 1.0 modifier
        // Some specific schools might have different modifiers
        // (this is where you'd implement specific school threat reductions)
        let _ = school_mask; // Unused for now
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_damage_threat() {
        let threat = ThreatCalcHelper::calc_damage_threat(100, 1.0, false, None);
        assert_eq!(threat, 100.0);
    }

    #[test]
    fn test_crit_threat() {
        let threat = ThreatCalcHelper::calc_damage_threat(100, 1.0, true, None);
        assert_eq!(threat, 200.0); // 2x default crit modifier
    }

    #[test]
    fn test_custom_crit_modifier() {
        let threat = ThreatCalcHelper::calc_damage_threat(100, 1.0, true, Some(1.5));
        assert_eq!(threat, 150.0); // 1.5x crit modifier
    }

    #[test]
    fn test_healing_threat() {
        let threat = ThreatCalcHelper::calc_heal_threat(100, None, None, None, None, None);
        assert_eq!(threat, 50.0); // 0.5x healing
    }

    #[test]
    fn test_no_threat_attribute() {
        let threat = ThreatCalcHelper::calc_threat(
            100.0,
            false,
            SPELL_SCHOOL_MASK_NORMAL,
            Some(1), // spell_id
            Some(SPELL_ATTR_EX_NO_THREAT),
            None,
            None,
            None,
            None,
        );
        assert_eq!(threat, 0.0);
    }

    #[test]
    fn test_no_initial_aggro_attribute() {
        let threat = ThreatCalcHelper::calc_threat(
            100.0,
            false,
            SPELL_SCHOOL_MASK_NORMAL,
            Some(1), // spell_id
            None,
            Some(SPELL_ATTR_EX2_NO_INITIAL_AGGRO),
            None,
            None,
            None,
        );
        assert_eq!(threat, 10.0); // 10% of normal
    }

    #[test]
    fn test_aura_modifier() {
        let threat = ThreatCalcHelper::calc_threat(
            100.0,
            false,
            SPELL_SCHOOL_MASK_NORMAL,
            None,
            None,
            None,
            None,
            Some(0.7), // 30% threat reduction (Blessing of Salvation)
            None,
        );
        assert_eq!(threat, 70.0);
    }
}
