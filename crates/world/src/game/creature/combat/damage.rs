//! Damage Calculation for Creature Combat
//!
//! Hit-table and damage formulas for a creature auto-attacking a player
//! (the reverse direction — a player attacking a creature — goes through the
//! shared skill-based hit table in `game::combat::hit_table` instead, since
//! the outcome roll is a single method for any attacker/defender pair, not
//! one formula for players and another for creatures).

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct CreatureMeleeChances {
    miss: f32,
    dodge: f32,
    parry: f32,
    block: f32,
    crit: f32,
    crushing: f32,
}

fn creature_melee_chances_vs_player(
    attacker_level: u8,
    target_level: u8,
    target_can_parry: bool,
    target_can_block: bool,
) -> CreatureMeleeChances {
    let skill_diff = (attacker_level as i32 - target_level as i32) * 5;
    let avoidance = (5.0 - skill_diff as f32 * 0.04).max(0.0);
    let crushing = if attacker_level >= target_level.saturating_add(3) {
        ((skill_diff as f32 * 2.0) - 15.0).max(15.0)
    } else {
        0.0
    };

    CreatureMeleeChances {
        miss: avoidance.min(60.0),
        dodge: avoidance,
        parry: if target_can_parry { avoidance } else { 0.0 },
        block: if target_can_block { avoidance } else { 0.0 },
        crit: (5.0 + skill_diff as f32 * 0.04).max(0.0),
        crushing,
    }
}

/// Roll creature auto-attack outcome against a player target.
///
/// Uses weapon-vs-defense skill deltas. For NPCs attacking players both skills
/// are effectively level * 5, so each level is a 5 skill-point delta:
/// miss/crit/avoidance shift by 0.2 percentage points per level. NPCs do not
/// produce glancing blows against players; crushing starts when the NPC is at
/// least 3 levels higher.
pub fn roll_creature_melee_hit_outcome(
    attacker_level: u8,
    target_level: u8,
    target_can_parry: bool,
    target_can_block: bool,
) -> MeleeHitOutcome {
    let chances = creature_melee_chances_vs_player(
        attacker_level,
        target_level,
        target_can_parry,
        target_can_block,
    );
    let mut rng = rand::thread_rng();
    let roll: f32 = rng.gen_range(0.0..100.0);
    let mut cumulative = 0.0;

    cumulative += chances.miss;
    if roll < cumulative {
        return MeleeHitOutcome::Miss;
    }

    cumulative += chances.dodge;
    if roll < cumulative {
        return MeleeHitOutcome::Dodge;
    }

    if chances.parry > 0.0 {
        cumulative += chances.parry;
        if roll < cumulative {
            return MeleeHitOutcome::Parry;
        }
    }

    if chances.block > 0.0 {
        cumulative += chances.block;
        if roll < cumulative {
            let blocked = rng.gen_range(20..=40);
            return MeleeHitOutcome::Block {
                blocked_amount: blocked,
            };
        }
    }

    cumulative += chances.crit;
    if roll < cumulative {
        return MeleeHitOutcome::Crit;
    }

    if chances.crushing > 0.0 {
        cumulative += chances.crushing;
        if roll < cumulative {
            return MeleeHitOutcome::Crushing;
        }
    }

    MeleeHitOutcome::Hit
}

/// Apply hit outcome to base damage, returning final damage
pub fn apply_hit_outcome(base_damage: u32, outcome: &MeleeHitOutcome) -> u32 {
    match outcome {
        MeleeHitOutcome::Miss | MeleeHitOutcome::Dodge | MeleeHitOutcome::Parry => 0,
        MeleeHitOutcome::Block { blocked_amount } => base_damage.saturating_sub(*blocked_amount),
        MeleeHitOutcome::Hit => base_damage,
        MeleeHitOutcome::Crit => base_damage * 2,
        MeleeHitOutcome::Glancing { reduction } => (base_damage as f32 * (1.0 - reduction)) as u32,
        MeleeHitOutcome::Crushing => (base_damage as f32 * 1.5) as u32,
    }
}

/// Calculate melee damage from player to creature
///
/// Formula:
/// ```text
/// base_damage = random(weapon_min, weapon_max)
/// armor_reduction = vanilla CalcArmorReducedDamage (see combat::armor_reduction_fraction)
/// final_damage = base_damage * (1 - armor_reduction)
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

    let damage_multiplier =
        1.0 - crate::game::combat::armor_reduction_fraction(target_armor, attacker_level);

    (base_damage as f32 * damage_multiplier) as u32
}

/// Convert MeleeHitOutcome to hit info flags for SMSG_ATTACKERSTATEUPDATE
pub fn hit_outcome_to_hit_info(outcome: &MeleeHitOutcome) -> u32 {
    use oxcore_shared::messages::combat::HitInfo;

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
pub fn hit_outcome_to_victim_state(outcome: &MeleeHitOutcome) -> u32 {
    use oxcore_shared::messages::combat::VictimState;

    match outcome {
        MeleeHitOutcome::Miss => VictimState::Intact as u32,
        MeleeHitOutcome::Dodge => VictimState::Dodge as u32, // 2
        MeleeHitOutcome::Parry => VictimState::Parry as u32, // 3
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
    fn creature_vs_player_same_level_uses_five_percent_base_table() {
        let chances = creature_melee_chances_vs_player(60, 60, true, true);

        assert_eq!(chances.miss, 5.0);
        assert_eq!(chances.dodge, 5.0);
        assert_eq!(chances.parry, 5.0);
        assert_eq!(chances.block, 5.0);
        assert_eq!(chances.crit, 5.0);
        assert_eq!(chances.crushing, 0.0);
    }

    #[test]
    fn creature_vs_player_level_delta_changes_by_skill_points_not_whole_percent_per_level() {
        let chances = creature_melee_chances_vs_player(61, 60, false, false);

        assert!((chances.miss - 4.8).abs() < f32::EPSILON);
        assert!((chances.dodge - 4.8).abs() < f32::EPSILON);
        assert!((chances.crit - 5.2).abs() < f32::EPSILON);
    }

    #[test]
    fn creature_vs_player_has_no_glancing_and_crushes_at_three_levels_up() {
        let chances_two_up = creature_melee_chances_vs_player(62, 60, false, false);
        let chances_three_up = creature_melee_chances_vs_player(63, 60, false, false);

        assert_eq!(chances_two_up.crushing, 0.0);
        assert_eq!(chances_three_up.crushing, 15.0);
    }
}
