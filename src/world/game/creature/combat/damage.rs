//! Damage Calculation for Creature Combat
//!
//! Implements the full Vanilla WoW 8-outcome melee hit table
//! and damage formulas including armor reduction.

use rand::Rng;

/// Full melee hit outcome enum (Vanilla WoW 8-outcome table)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeleeHitOutcome {
    Miss,
    Dodge,
    Parry,
    Block { blocked_amount: u32 },
    Hit,
    Crit,
    Glancing { reduction: f32 },
    Crushing,
}

/// Roll hit outcome using the single-roll combat table
///
/// Roll order:
///   1. Miss (5% base + level_diff * 1%)
///   2. Dodge (5% base, reduced by level_diff)
///   3. Parry (5% base, only if target can parry)
///   4. Block (5% base, only if target can block)
///   5. Glancing (10% if attacker >= 4 levels higher)
///   6. Crushing (15% if attacker >= 4 levels higher)
///   7. Crit (5% base + 2% per level_diff)
///   8. Normal Hit (remainder)
pub fn roll_melee_hit_outcome(
    attacker_level: u8,
    target_level: u8,
    target_can_parry: bool,
    target_can_block: bool,
) -> MeleeHitOutcome {
    let level_diff = attacker_level as i32 - target_level as i32;
    let mut rng = rand::thread_rng();
    let roll: f32 = rng.gen();
    let mut cumulative = 0.0;

    // Miss
    let miss_chance = (0.05 + level_diff as f32 * 0.01).clamp(0.0, 0.60);
    cumulative += miss_chance;
    if roll < cumulative {
        return MeleeHitOutcome::Miss;
    }

    // Dodge
    let dodge_chance = (0.05 - level_diff as f32 * 0.001).clamp(0.0, 0.20);
    cumulative += dodge_chance;
    if roll < cumulative {
        return MeleeHitOutcome::Dodge;
    }

    // Parry (only if target can parry — requires weapon/skill)
    if target_can_parry {
        let parry_chance = (0.05 - level_diff as f32 * 0.001).clamp(0.0, 0.15);
        cumulative += parry_chance;
        if roll < cumulative {
            return MeleeHitOutcome::Parry;
        }
    }

    // Block (only if target has shield equipped)
    if target_can_block {
        let block_chance: f32 = 0.05;
        cumulative += block_chance;
        if roll < cumulative {
            let blocked = rng.gen_range(20..=40);
            return MeleeHitOutcome::Block {
                blocked_amount: blocked,
            };
        }
    }

    // Glancing (attacker >= 4 levels higher)
    if level_diff >= 4 {
        cumulative += 0.10;
        if roll < cumulative {
            let reduction = rng.gen_range(0.10_f32..=0.40);
            return MeleeHitOutcome::Glancing { reduction };
        }
    }

    // Crushing (attacker >= 4 levels higher)
    if level_diff >= 4 {
        cumulative += 0.15;
        if roll < cumulative {
            return MeleeHitOutcome::Crushing;
        }
    }

    // Crit
    let crit_chance = (0.05 + level_diff as f32 * 0.02).clamp(0.0, 0.30);
    cumulative += crit_chance;
    if roll < cumulative {
        return MeleeHitOutcome::Crit;
    }

    // Normal hit
    MeleeHitOutcome::Hit
}

/// Apply hit outcome to base damage, returning final damage
pub fn apply_hit_outcome(base_damage: u32, outcome: &MeleeHitOutcome) -> u32 {
    match outcome {
        MeleeHitOutcome::Miss | MeleeHitOutcome::Dodge | MeleeHitOutcome::Parry => 0,
        MeleeHitOutcome::Block { blocked_amount } => base_damage.saturating_sub(*blocked_amount),
        MeleeHitOutcome::Hit => base_damage,
        MeleeHitOutcome::Crit => base_damage * 2,
        MeleeHitOutcome::Glancing { reduction } => {
            (base_damage as f32 * (1.0 - reduction)) as u32
        }
        MeleeHitOutcome::Crushing => (base_damage as f32 * 1.5) as u32,
    }
}

/// Calculate melee damage from player to creature
///
/// Formula:
/// ```text
/// base_damage = random(weapon_min, weapon_max)
/// armor_reduction = armor / (armor + 400 + 85 * attacker_level)
/// final_damage = base_damage * (1 - min(armor_reduction, 0.75))
/// ```
pub fn calculate_melee_damage(
    attacker_level: u8,
    weapon_min: u32,
    weapon_max: u32,
    target_armor: u32,
) -> u32 {
    let base_damage = if weapon_max > weapon_min {
        rand::thread_rng().gen_range(weapon_min..=weapon_max)
    } else {
        weapon_min
    };

    // Vanilla WoW armor reduction formula
    let armor_reduction =
        target_armor as f32 / (target_armor as f32 + 400.0 + 85.0 * attacker_level as f32);
    let armor_reduction = armor_reduction.min(0.75);
    let damage_multiplier = 1.0 - armor_reduction;

    (base_damage as f32 * damage_multiplier) as u32
}

/// Convert MeleeHitOutcome to hit info flags for SMSG_ATTACKERSTATEUPDATE
/// Matches MaNGOS UnitDefines.h flag values for 1.12.1 client
pub fn hit_outcome_to_hit_info(outcome: &MeleeHitOutcome) -> u32 {
    use crate::shared::messages::combat::HitInfo;

    let affects = HitInfo::AffectsVictim as u32;

    match outcome {
        // No AFFECTS_VICTIM for miss/dodge/parry (no hit animation on victim)
        MeleeHitOutcome::Miss => HitInfo::Miss as u32,
        MeleeHitOutcome::Dodge => HitInfo::NormalSwing as u32,
        MeleeHitOutcome::Parry => HitInfo::NormalSwing as u32,
        // All damage-dealing outcomes include AFFECTS_VICTIM
        MeleeHitOutcome::Block { .. } => affects,
        MeleeHitOutcome::Hit => affects,
        MeleeHitOutcome::Crit => affects | HitInfo::CriticalHit as u32,
        MeleeHitOutcome::Glancing { .. } => affects | HitInfo::Glancing as u32,
        MeleeHitOutcome::Crushing => affects | HitInfo::Crushing as u32,
    }
}

/// Convert MeleeHitOutcome to victim state for SMSG_ATTACKERSTATEUPDATE
/// Values from MaNGOS UnitDefines.h: UNAFFECTED=0, NORMAL=1, DODGE=2, PARRY=3, INTERRUPT=4, BLOCKS=5
pub fn hit_outcome_to_victim_state(outcome: &MeleeHitOutcome) -> u32 {
    use crate::shared::messages::combat::VictimState;

    match outcome {
        MeleeHitOutcome::Miss => VictimState::Intact as u32,
        MeleeHitOutcome::Dodge => VictimState::Dodge as u32,   // 2
        MeleeHitOutcome::Parry => VictimState::Parry as u32,   // 3
        MeleeHitOutcome::Block { .. } => VictimState::Block as u32, // 5
        MeleeHitOutcome::Hit => VictimState::Hit as u32,
        MeleeHitOutcome::Crit => VictimState::Hit as u32,
        MeleeHitOutcome::Glancing { .. } => VictimState::Hit as u32,
        MeleeHitOutcome::Crushing => VictimState::Hit as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_melee_damage_no_armor() {
        let damage = calculate_melee_damage(10, 10, 20, 0);
        assert!(damage >= 10 && damage <= 20);
    }

    #[test]
    fn test_calculate_melee_damage_with_armor() {
        let damage = calculate_melee_damage(10, 20, 20, 500);
        assert!(damage < 20);
        assert!(damage > 0);
    }

    #[test]
    fn test_calculate_melee_damage_armor_cap() {
        let damage = calculate_melee_damage(10, 100, 100, 10000);
        assert_eq!(damage, 25);
    }

    #[test]
    fn test_apply_hit_outcome_miss() {
        assert_eq!(apply_hit_outcome(100, &MeleeHitOutcome::Miss), 0);
    }

    #[test]
    fn test_apply_hit_outcome_dodge() {
        assert_eq!(apply_hit_outcome(100, &MeleeHitOutcome::Dodge), 0);
    }

    #[test]
    fn test_apply_hit_outcome_parry() {
        assert_eq!(apply_hit_outcome(100, &MeleeHitOutcome::Parry), 0);
    }

    #[test]
    fn test_apply_hit_outcome_block() {
        assert_eq!(
            apply_hit_outcome(100, &MeleeHitOutcome::Block { blocked_amount: 30 }),
            70
        );
    }

    #[test]
    fn test_apply_hit_outcome_crit() {
        assert_eq!(apply_hit_outcome(100, &MeleeHitOutcome::Crit), 200);
    }

    #[test]
    fn test_apply_hit_outcome_crushing() {
        assert_eq!(apply_hit_outcome(100, &MeleeHitOutcome::Crushing), 150);
    }

    #[test]
    fn test_apply_hit_outcome_glancing() {
        let dmg = apply_hit_outcome(100, &MeleeHitOutcome::Glancing { reduction: 0.25 });
        assert_eq!(dmg, 75);
    }

    #[test]
    fn test_roll_outcome_returns_valid() {
        let outcome = roll_melee_hit_outcome(10, 15, false, false);
        assert!(matches!(
            outcome,
            MeleeHitOutcome::Miss
                | MeleeHitOutcome::Dodge
                | MeleeHitOutcome::Hit
                | MeleeHitOutcome::Crit
                | MeleeHitOutcome::Glancing { .. }
                | MeleeHitOutcome::Crushing
        ));
    }
}
