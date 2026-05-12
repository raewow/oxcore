/// Calculate the chance (0-100) that a weapon skill increases after a successful hit.
///
/// The formula is ported from MaNGOS Player::UpdateCombatSkills() and has two regimes:
///
/// **Progress 1-90% (skill < 90% of max):**
///   chance = (max * 0.9 * 50) / current
///   This gives ~100% at very low skill, decaying to ~50% at 90% of max.
///
/// **Progress 90-100% (skill >= 90% of max):**
///   ratio = 300 / max
///   chance = (0.5 - 0.0168966 * current * ratio + 0.0152069 * max * ratio) * 100
///   This produces a steep drop-off in the final 10%, making the last few points slow.
///
/// **Intellect bonus:** Up to +10% chance from intellect stat (0.02 * intellect, capped at 10).
///   This means high-intellect classes (mages, priests) skill up slightly faster.
///
/// **Small skill-diff penalty:** If skill_diff (max - current) <= 3, the chance is reduced
///   by a factor of 0.5 / (4 - skill_diff), making the final 3 points especially slow.
///
/// # Arguments
/// * `current` - Current weapon skill value
/// * `max` - Maximum weapon skill value (level * 5)
/// * `intellect` - Player's intellect stat (as f32)
/// * `skill_diff` - max - current (passed separately since caller already has it)
///
/// # Returns
/// Chance percentage (0.0 - 100.0)
pub fn calculate_weapon_skill_up_chance(
    current: u16,
    max: u16,
    intellect: f32,
    skill_diff: u32,
) -> f32 {
    if current == 0 || max == 0 || current >= max {
        return 0.0;
    }

    let current_f = current as f32;
    let max_f = max as f32;

    // Base chance calculation (two-regime formula)
    let chance = if max_f * 0.9 > current_f {
        // Progress 1-90%: chance decreases from ~100% to ~50%
        ((max_f * 0.9 * 50.0) / current_f).min(100.0)
    } else {
        // Progress 90-100%: complex decay formula
        let ratio = 300.0 / max_f;
        (0.5 - 0.0168966 * current_f * ratio + 0.0152069 * max_f * ratio) * 100.0
    };

    // Add intellect bonus (capped at +10%)
    let intel_bonus = (0.02 * intellect).min(10.0);
    let mut final_chance = chance + intel_bonus;

    // Reduce chance for very small skill differences (last 3 points are slow)
    if skill_diff <= 3 {
        final_chance *= 0.5 / (4.0 - skill_diff as f32);
    }

    final_chance.clamp(0.0, 100.0)
}

/// Calculate the chance (0-100) that defense skill increases after being hit.
///
/// The formula is ported from MaNGOS Player::UpdateCombatSkills() with is_defense=true:
///
///   gray_level = max(1, player_level - 5)   (simplified gray level formula)
///   mob_level  = min(creature_level, player_level + 5)
///   lvl_diff   = max(3, mob_level - gray_level)
///   chance     = (3.0 * lvl_diff * skill_diff) / player_level
///
/// Key behaviors:
/// - No defense skill gain from gray-level creatures (too low level)
/// - Creatures more than 5 levels above the player are capped (no extra benefit)
/// - Higher skill_diff (further from max) = higher chance
/// - Higher player level = lower chance per point (harder to gain at high levels)
///
/// # Arguments
/// * `player_level` - Player's current level
/// * `creature_level` - Level of the creature that hit the player
/// * `current` - Current defense skill value
/// * `max` - Maximum defense skill value (level * 5)
///
/// # Returns
/// Chance percentage (0.0 - 100.0)
pub fn calculate_defense_skill_up_chance(
    player_level: u8,
    creature_level: u8,
    current: u16,
    max: u16,
) -> f32 {
    if current >= max || player_level == 0 {
        return 0.0;
    }

    let player_lvl = player_level as u32;
    let gray_level = get_gray_level(player_lvl);
    let mob_level = (creature_level as u32).min(player_lvl + 5);
    let skill_diff = (max - current) as f32;

    // Minimum effective level difference of 3
    let lvl_diff = ((mob_level as i32) - (gray_level as i32)).max(3) as f32;

    let chance = (3.0 * lvl_diff * skill_diff) / player_lvl as f32;
    chance.clamp(0.0, 100.0)
}

/// Get the gray level threshold for a given player level.
///
/// Creatures at or below gray level give no experience and no skill gains.
/// Simplified formula: max(1, level - 5)
///
/// The full MaNGOS formula uses a lookup table:
/// - Level 1-5:   gray = 0
/// - Level 6-39:  gray = level - 5 - floor(level/10)
/// - Level 40-59: gray = level - 1 - floor(level/5)
/// - Level 60:    gray = level - 9
fn get_gray_level(level: u32) -> u32 {
    level.saturating_sub(5).max(1)
}

/// No skill gain from PvP combat.
/// This is an important rule: weapon and defense skills only increase
/// from PvE combat (hitting or being hit by creatures).
pub fn can_gain_skill_from_target(target_is_player: bool) -> bool {
    !target_is_player
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_skill_high_chance() {
        // Skill 10/300 should have very high chance
        let chance = calculate_weapon_skill_up_chance(10, 300, 0.0, 290);
        assert!(
            chance > 90.0,
            "Low skill should have high chance, got {}",
            chance
        );
    }

    #[test]
    fn test_near_max_low_chance() {
        // Skill 298/300 should have very low chance
        let chance = calculate_weapon_skill_up_chance(298, 300, 0.0, 2);
        assert!(
            chance < 5.0,
            "Near-max skill should have low chance, got {}",
            chance
        );
    }

    #[test]
    fn test_already_maxed_zero_chance() {
        let chance = calculate_weapon_skill_up_chance(300, 300, 100.0, 0);
        assert_eq!(chance, 0.0);
    }

    #[test]
    fn test_intellect_bonus_capped() {
        let without_int = calculate_weapon_skill_up_chance(150, 300, 0.0, 150);
        let with_int = calculate_weapon_skill_up_chance(150, 300, 1000.0, 150);
        // Intellect bonus capped at +10%
        assert!((with_int - without_int) <= 10.01);
    }

    #[test]
    fn test_defense_skill_gain_from_same_level() {
        // Level 60, skill 290/300, creature level 60
        let chance = calculate_defense_skill_up_chance(60, 60, 290, 300);
        assert!(chance > 0.0, "Should have some chance, got {}", chance);
    }

    #[test]
    fn test_defense_maxed_no_gain() {
        let chance = calculate_defense_skill_up_chance(60, 60, 300, 300);
        assert_eq!(chance, 0.0);
    }

    #[test]
    fn test_defense_higher_creature_more_chance() {
        let chance_same = calculate_defense_skill_up_chance(60, 60, 280, 300);
        let chance_higher = calculate_defense_skill_up_chance(60, 63, 280, 300);
        assert!(chance_higher >= chance_same);
    }

    #[test]
    fn test_no_skill_gain_pvp() {
        assert!(!can_gain_skill_from_target(true));
        assert!(can_gain_skill_from_target(false));
    }
}
