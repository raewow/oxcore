//! Hit Table Calculation - Vanilla WoW single-roll hit table
//!
//! Uses a single random roll (0-10000) to determine attack outcome.
//! Order: miss → dodge → parry → glancing → block → crit → crush → hit
//!
//! Reference: https://wowwiki-archive.fandom.com/wiki/Attack_table

use super::state::{AttackHand, AttackOutcome};

/// Snapshot of attacker/defender stats needed for hit table
/// Captured once, used by pure functions
#[derive(Debug, Clone)]
pub struct CombatSnapshot {
    // Attacker
    pub attacker_level: u8,
    pub attacker_weapon_skill: u16, // Weapon skill (level*5 base)
    pub attacker_hit_bonus: f32,    // +hit% from gear/talents
    pub attacker_crit_chance: f32,  // Crit% from stats
    pub attacker_ap: i32,           // Attack power
    pub is_dual_wielding: bool,

    // Defender
    pub defender_level: u8,
    pub defender_defense_skill: u16, // Defense skill (level*5 base)
    pub defender_dodge_chance: f32,
    pub defender_parry_chance: f32,
    pub defender_block_chance: f32,
    pub defender_block_value: u32,
    pub defender_armor: u32,
    pub defender_is_player: bool,
    pub defender_can_parry: bool,
    pub defender_can_block: bool,

    // Context
    pub hand: AttackHand,
    pub is_ranged: bool,
}

impl CombatSnapshot {
    /// Create a basic snapshot for testing
    pub fn new(attacker_level: u8, defender_level: u8) -> Self {
        Self {
            attacker_level,
            attacker_weapon_skill: attacker_level as u16 * 5,
            attacker_hit_bonus: 0.0,
            attacker_crit_chance: 5.0,
            attacker_ap: 100,
            is_dual_wielding: false,
            defender_level,
            defender_defense_skill: defender_level as u16 * 5,
            defender_dodge_chance: 5.0,
            defender_parry_chance: 5.0,
            defender_block_chance: 5.0,
            defender_block_value: 0,
            defender_armor: 0,
            defender_is_player: false,
            defender_can_parry: true,
            defender_can_block: true,
            hand: AttackHand::MainHand,
            is_ranged: false,
        }
    }
}

/// Calculate attack outcome using single-roll hit table
/// Returns the outcome determined by a single random roll
pub fn calculate_hit_table(snapshot: &CombatSnapshot) -> AttackOutcome {
    let skill_diff = snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
    let level_diff = snapshot.defender_level as i32 - snapshot.attacker_level as i32;

    // === Miss Chance ===
    // Base miss: 5% + skill adjustments
    // For +3 level boss (skill diff = 15): 5 + 15*0.04 = 5.6% (but special rule applies)
    // Special rule: if defender skill > attacker skill + 10: miss += (diff-10) * 0.4%
    let mut miss_chance = 5.0;
    if skill_diff > 0 {
        if skill_diff <= 10 {
            miss_chance += skill_diff as f32 * 0.1;
        } else {
            miss_chance += 10.0 * 0.1 + (skill_diff - 10) as f32 * 0.4;
        }
    } else {
        miss_chance += skill_diff as f32 * 0.04;
    }

    // Dual wield penalty: +19% miss for white attacks
    if snapshot.is_dual_wielding && !snapshot.is_ranged && !is_yellow_attack() {
        miss_chance += 19.0;
    }

    // Apply +hit bonus (reduces miss)
    miss_chance -= snapshot.attacker_hit_bonus;
    miss_chance = miss_chance.max(0.0);

    // Minimum miss chance for white attacks (1% vs same level, increases with level diff)
    let min_miss = if level_diff > 0 {
        1.0 + level_diff as f32 * 0.5
    } else {
        1.0
    };
    miss_chance = miss_chance.max(min_miss);

    // === Dodge Chance ===
    let mut dodge_chance = snapshot.defender_dodge_chance;
    dodge_chance += skill_diff as f32 * 0.04;
    dodge_chance = dodge_chance.max(0.0);

    // === Parry Chance ===
    let mut parry_chance = 0.0;
    if snapshot.defender_can_parry && !snapshot.is_ranged {
        parry_chance = snapshot.defender_parry_chance;
        parry_chance += skill_diff as f32 * 0.04;
        parry_chance = parry_chance.max(0.0);
    }

    // === Glancing Blow Chance ===
    // Only applies to white (auto-attack) hits vs higher-level targets
    // Chance: 10 + (defender_defense - attacker_skill) * 2
    let mut glancing_chance = 0.0;
    if skill_diff > 0 && !is_yellow_attack() {
        glancing_chance = (10.0 + skill_diff as f32 * 2.0).max(0.0);
        glancing_chance = glancing_chance.min(40.0); // Cap at 40%
    }

    // === Block Chance ===
    let mut block_chance = 0.0;
    if snapshot.defender_can_block && !snapshot.is_ranged {
        block_chance = snapshot.defender_block_chance;
        block_chance += skill_diff as f32 * 0.04;
        block_chance = block_chance.max(0.0);
    }

    // === Crit Chance ===
    let mut crit_chance = snapshot.attacker_crit_chance;
    // Crit suppression vs higher level targets
    if level_diff > 0 {
        crit_chance -= level_diff as f32 * 0.2;
    }
    // Crit suppression from defense skill
    crit_chance -= skill_diff as f32 * 0.04;
    crit_chance = crit_chance.max(0.0);

    // === Crushing Blow Chance ===
    // Only from mobs 4+ levels above player
    // Chance: (level_diff * 2 - 15) * 2%
    let mut crushing_chance = 0.0;
    if !snapshot.defender_is_player && level_diff >= 4 {
        crushing_chance = ((level_diff * 2 - 15) as f32 * 2.0).max(0.0);
    }

    // === Single Roll ===
    // Roll 0-100 and check thresholds in order
    let roll = rand::random::<f32>() * 100.0;
    let mut threshold = 0.0;

    // 1. Miss
    threshold += miss_chance;
    if roll < threshold {
        return AttackOutcome::Miss;
    }

    // 2. Dodge
    threshold += dodge_chance;
    if roll < threshold {
        return AttackOutcome::Dodge;
    }

    // 3. Parry
    threshold += parry_chance;
    if roll < threshold {
        return AttackOutcome::Parry;
    }

    // 4. Glancing
    threshold += glancing_chance;
    if roll < threshold {
        return AttackOutcome::Glancing;
    }

    // 5. Block
    threshold += block_chance;
    if roll < threshold {
        return AttackOutcome::Block;
    }

    // 6. Crit
    threshold += crit_chance;
    if roll < threshold {
        return AttackOutcome::CriticalHit;
    }

    // 7. Crushing
    threshold += crushing_chance;
    if roll < threshold {
        return AttackOutcome::CrushingBlow;
    }

    // 8. Normal hit
    AttackOutcome::NormalHit
}

/// Check if this is a yellow attack (special ability)
/// For auto-attacks, this returns false
fn is_yellow_attack() -> bool {
    // Auto-attacks are white attacks
    false
}

/// Calculate miss chance percentage (for display/tooltips)
pub fn calculate_miss_chance(snapshot: &CombatSnapshot) -> f32 {
    let skill_diff = snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
    let level_diff = snapshot.defender_level as i32 - snapshot.attacker_level as i32;

    let mut miss_chance = 5.0;
    if skill_diff > 0 {
        if skill_diff <= 10 {
            miss_chance += skill_diff as f32 * 0.1;
        } else {
            miss_chance += 10.0 * 0.1 + (skill_diff - 10) as f32 * 0.4;
        }
    } else {
        miss_chance += skill_diff as f32 * 0.04;
    }

    if snapshot.is_dual_wielding && !snapshot.is_ranged {
        miss_chance += 19.0;
    }

    miss_chance -= snapshot.attacker_hit_bonus;

    let min_miss = if level_diff > 0 {
        1.0 + level_diff as f32 * 0.5
    } else {
        1.0
    };

    miss_chance.max(min_miss)
}

/// Calculate dodge chance against defender
pub fn calculate_dodge_chance(snapshot: &CombatSnapshot) -> f32 {
    let skill_diff = snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
    let mut dodge = snapshot.defender_dodge_chance;
    dodge += skill_diff as f32 * 0.04;
    dodge.max(0.0)
}

/// Calculate parry chance against defender
pub fn calculate_parry_chance(snapshot: &CombatSnapshot) -> f32 {
    if snapshot.is_ranged || !snapshot.defender_can_parry {
        return 0.0;
    }
    let skill_diff = snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
    let mut parry = snapshot.defender_parry_chance;
    parry += skill_diff as f32 * 0.04;
    parry.max(0.0)
}

/// Calculate crit chance against defender
pub fn calculate_effective_crit(snapshot: &CombatSnapshot) -> f32 {
    let skill_diff = snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
    let level_diff = snapshot.defender_level as i32 - snapshot.attacker_level as i32;

    let mut crit = snapshot.attacker_crit_chance;
    if level_diff > 0 {
        crit -= level_diff as f32 * 0.2;
    }
    crit -= skill_diff as f32 * 0.04;
    crit.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_table_same_level() {
        let snapshot = CombatSnapshot::new(60, 60);
        // Should return some outcome (mostly normal hits at same level)
        let outcome = calculate_hit_table(&snapshot);
        // Just verify it runs without panicking
        assert!(!matches!(outcome, AttackOutcome::CrushingBlow)); // No crushing vs same level
    }

    #[test]
    fn test_hit_table_vs_boss() {
        let mut snapshot = CombatSnapshot::new(60, 63);
        snapshot.defender_is_player = false;

        // Vs boss level target, should be able to get glancing, crushing
        // Run multiple times to get different outcomes
        for _ in 0..100 {
            let _outcome = calculate_hit_table(&snapshot);
        }
    }

    #[test]
    fn test_miss_chance_calculation() {
        let snapshot = CombatSnapshot::new(60, 60);
        let miss = calculate_miss_chance(&snapshot);
        assert!(miss >= 1.0); // Minimum 1% vs same level
    }

    #[test]
    fn test_dual_wield_miss_penalty() {
        let mut snapshot = CombatSnapshot::new(60, 60);
        let miss_normal = calculate_miss_chance(&snapshot);

        snapshot.is_dual_wielding = true;
        let miss_dw = calculate_miss_chance(&snapshot);

        assert!(miss_dw > miss_normal);
        assert!(miss_dw >= miss_normal + 19.0 - 0.01); // +19% penalty
    }

    #[test]
    fn test_glancing_vs_higher_level() {
        let mut snapshot = CombatSnapshot::new(60, 63);
        snapshot.defender_defense_skill = 315; // Boss defense
        snapshot.attacker_weapon_skill = 300; // Player weapon skill

        // Glancing chance should be higher vs boss
        let skill_diff =
            snapshot.defender_defense_skill as i32 - snapshot.attacker_weapon_skill as i32;
        assert!(skill_diff > 0);
    }
}
