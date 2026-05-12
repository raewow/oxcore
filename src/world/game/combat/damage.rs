//! Damage Calculation - Physical and spell damage formulas
//!
//! Includes AP scaling, armor reduction, and outcome modifiers.

use super::hit_table::CombatSnapshot;
use super::state::{AttackHand, AttackOutcome, DamageResult};

/// Calculate physical damage for an auto-attack
pub fn calculate_melee_damage(
    snapshot: &CombatSnapshot,
    outcome: AttackOutcome,
    weapon_min: f32,
    weapon_max: f32,
    weapon_speed_ms: u32, // in milliseconds
) -> DamageResult {
    let weapon_speed = weapon_speed_ms as f32 / 1000.0;

    // Base weapon damage (random between min and max)
    let base_weapon_dmg = weapon_min + rand::random::<f32>() * (weapon_max - weapon_min);

    // AP contribution: (AP / 14) * weapon_speed
    let ap_contribution = snapshot.attacker_ap as f32 / 14.0 * weapon_speed;

    let mut damage = base_weapon_dmg + ap_contribution;

    // Off-hand penalty: 50% damage
    if snapshot.hand == AttackHand::OffHand {
        damage *= 0.5;
    }

    // Apply outcome modifiers
    let (final_damage, blocked) = match outcome {
        AttackOutcome::Miss | AttackOutcome::Dodge | AttackOutcome::Parry => {
            return DamageResult::no_damage(outcome, snapshot.hand);
        }

        AttackOutcome::Glancing => {
            // Glancing blow penalty: 65-85% damage depending on skill diff
            let skill_diff =
                snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
            // Formula from vanilla: low = 1.3 - 0.05 * skill_diff, high = 1.2 - 0.03 * skill_diff
            let low = (1.3 - 0.05 * skill_diff as f32).clamp(0.01, 0.91);
            let high = (1.2 - 0.03 * skill_diff as f32).clamp(0.2, 0.99);
            let multiplier = low + rand::random::<f32>() * (high - low);
            (damage * multiplier, 0u32)
        }

        AttackOutcome::Block => {
            // Blocked damage: reduce by block_value
            let blocked_amount = snapshot.defender_block_value.min(damage as u32);
            (damage - blocked_amount as f32, blocked_amount)
        }

        AttackOutcome::CriticalHit => {
            // Critical strike: 200% damage (melee)
            (damage * 2.0, 0u32)
        }

        AttackOutcome::CrushingBlow => {
            // Crushing blow: 150% damage
            (damage * 1.5, 0u32)
        }

        AttackOutcome::NormalHit => (damage, 0u32),
    };

    // Apply armor reduction
    let after_armor = apply_armor_reduction(
        final_damage,
        snapshot.defender_armor,
        snapshot.attacker_level,
    );

    DamageResult {
        outcome,
        damage: after_armor.max(0.0) as u32,
        absorbed: 0,
        resisted: 0,
        blocked,
        overkill: 0,
        damage_school: 0, // Physical
        hand: snapshot.hand,
    }
}

/// Calculate ranged (auto-shot) damage
pub fn calculate_ranged_damage(
    snapshot: &CombatSnapshot,
    outcome: AttackOutcome,
    weapon_min: f32,
    weapon_max: f32,
    weapon_speed_ms: u32,
    ammo_dps: f32,
) -> DamageResult {
    let weapon_speed = weapon_speed_ms as f32 / 1000.0;

    // Base weapon damage
    let base_weapon_dmg = weapon_min + rand::random::<f32>() * (weapon_max - weapon_min);

    // AP contribution for ranged
    let ap_contribution = snapshot.attacker_ap as f32 / 14.0 * weapon_speed;

    // Ammo contribution
    let ammo_damage = ammo_dps * weapon_speed;

    let damage = base_weapon_dmg + ap_contribution + ammo_damage;

    // Apply outcome modifiers (ranged has no glancing or crushing)
    let final_damage = match outcome {
        AttackOutcome::Miss | AttackOutcome::Dodge | AttackOutcome::Parry => {
            return DamageResult::no_damage(outcome, snapshot.hand);
        }

        AttackOutcome::Block => {
            // Ranged attacks cannot be blocked in vanilla
            damage
        }

        AttackOutcome::CriticalHit => {
            // Ranged crit: 200% damage
            damage * 2.0
        }

        _ => damage,
    };

    // Apply armor reduction
    let after_armor = apply_armor_reduction(
        final_damage,
        snapshot.defender_armor,
        snapshot.attacker_level,
    );

    DamageResult {
        outcome,
        damage: after_armor.max(0.0) as u32,
        absorbed: 0,
        resisted: 0,
        blocked: 0, // Ranged can't be blocked
        overkill: 0,
        damage_school: 0,
        hand: snapshot.hand,
    }
}

/// Armor damage reduction formula (vanilla WoW 1.12)
///
/// Formula: reduction% = armor / (armor + 400 + 85 * attacker_level)
/// Capped at 75% reduction
pub fn apply_armor_reduction(damage: f32, armor: u32, attacker_level: u8) -> f32 {
    if armor == 0 {
        return damage;
    }

    let reduction = armor as f32 / (armor as f32 + 400.0 + 85.0 * attacker_level as f32);
    let reduction = reduction.min(0.75); // Cap at 75%

    damage * (1.0 - reduction)
}

/// Calculate armor reduction percentage (for display)
pub fn calculate_armor_reduction_pct(armor: u32, attacker_level: u8) -> f32 {
    if armor == 0 {
        return 0.0;
    }

    let reduction = armor as f32 / (armor as f32 + 400.0 + 85.0 * attacker_level as f32);
    (reduction * 100.0).min(75.0)
}

/// Spell resistance check (binary resist system for vanilla)
/// Returns portion of damage resisted (0%, 25%, 50%, 75%, 100%)
pub fn calculate_spell_resistance(
    damage: f32,
    resistance: u32,
    attacker_level: u8,
    spell_penetration: u32,
) -> (f32, f32) {
    let effective_resist = (resistance as i32 - spell_penetration as i32).max(0) as f32;
    let level = attacker_level as f32;

    // Average resistance: resist / (5 * level)
    let avg_resist_pct = (effective_resist / (5.0 * level)).min(0.75);

    // Roll for partial resist (vanilla uses discrete outcomes)
    let roll = rand::random::<f32>();
    let resist_pct = if roll < avg_resist_pct * 0.5 {
        0.75 // 75% resist
    } else if roll < avg_resist_pct {
        0.50 // 50% resist
    } else if roll < avg_resist_pct * 1.5 {
        0.25 // 25% resist
    } else {
        0.0 // No resist
    };

    let resisted = damage * resist_pct;
    (damage - resisted, resisted)
}

/// Calculate damage after all modifiers
pub fn calculate_final_damage(base_damage: f32, modifiers: &[f32]) -> f32 {
    let mut final_dmg = base_damage;
    for modifier in modifiers {
        final_dmg *= modifier;
    }
    final_dmg.max(0.0)
}

/// Get damage reduction from armor as a display string
pub fn format_armor_reduction(armor: u32, attacker_level: u8) -> String {
    let pct = calculate_armor_reduction_pct(armor, attacker_level);
    format!("{:.1}%", pct)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_armor_reduction() {
        // Test with no armor
        let dmg = apply_armor_reduction(100.0, 0, 60);
        assert!((dmg - 100.0).abs() < 0.01);

        // Test with armor
        let dmg = apply_armor_reduction(100.0, 1000, 60);
        assert!(dmg < 100.0);
        assert!(dmg > 0.0);

        // Test 75% cap
        let dmg = apply_armor_reduction(100.0, 100000, 60);
        assert!((dmg - 25.0).abs() < 0.01); // 25% of original = 75% reduction
    }

    #[test]
    fn test_melee_damage_normal_hit() {
        let snapshot = CombatSnapshot::new(60, 60);
        let result = calculate_melee_damage(
            &snapshot,
            AttackOutcome::NormalHit,
            10.0,
            20.0, // weapon damage 10-20
            2000, // 2 second swing
        );

        assert!(result.damage > 0);
        assert_eq!(result.outcome, AttackOutcome::NormalHit);
    }

    #[test]
    fn test_melee_damage_crit() {
        let snapshot = CombatSnapshot::new(60, 60);
        let result = calculate_melee_damage(
            &snapshot,
            AttackOutcome::CriticalHit,
            10.0,
            10.0, // fixed 10 damage
            2000,
        );

        // Crit should be 200% damage (base + AP contribution)
        assert!(result.damage > 0);
        assert_eq!(result.outcome, AttackOutcome::CriticalHit);
    }

    #[test]
    fn test_offhand_penalty() {
        let mut snapshot = CombatSnapshot::new(60, 60);

        // Main hand
        snapshot.hand = AttackHand::MainHand;
        let main = calculate_melee_damage(&snapshot, AttackOutcome::NormalHit, 10.0, 10.0, 2000);

        // Off hand
        snapshot.hand = AttackHand::OffHand;
        let off = calculate_melee_damage(&snapshot, AttackOutcome::NormalHit, 10.0, 10.0, 2000);

        // Offhand should deal ~50% of main hand
        assert!(off.damage < main.damage);
    }

    #[test]
    fn test_glancing_damage() {
        let snapshot = CombatSnapshot::new(60, 63); // vs higher level
        let result = calculate_melee_damage(
            &snapshot,
            AttackOutcome::Glancing,
            100.0,
            100.0, // fixed 100 damage
            2000,
        );

        // Glancing should reduce damage
        assert!(result.damage < 100);
        assert_eq!(result.outcome, AttackOutcome::Glancing);
    }

    #[test]
    fn test_spell_resistance() {
        // Test with no resistance
        let (dealt, resisted) = calculate_spell_resistance(100.0, 0, 60, 0);
        assert_eq!(dealt, 100.0);
        assert_eq!(resisted, 0.0);

        // Test with high resistance (may still get no resist due to RNG)
        for _ in 0..10 {
            let (_dealt, _resisted) = calculate_spell_resistance(100.0, 200, 60, 0);
        }
    }
}
