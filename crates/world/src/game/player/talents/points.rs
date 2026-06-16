use super::state::TalentState;

/// Calculate total talent points available for a given level.
///
/// Players earn talent points starting at level 10.
/// Formula: (level - 9) * rate
///   - Level 10 = 1 point
///   - Level 30 = 21 points
///   - Level 60 = 51 points (max in vanilla)
///
/// Ported from MaNGOS Player::CalculateTalentsPoints() (Player.cpp:3180-3198).
///
/// # Arguments
/// * `level` - Player's current level (1-60)
/// * `rate` - Config multiplier (default 1.0, from CONFIG_FLOAT_RATE_TALENT)
///
/// # Returns
/// Total talent points available at this level (before subtracting spent points)
pub fn calculate_total_talent_points(level: u8, rate: f32) -> u32 {
    if level < 10 {
        0
    } else {
        let base_points = (level as u32) - 9;
        (base_points as f32 * rate) as u32
    }
}

/// Update free talent points after spending, resetting, or leveling.
///
/// Recalculates free = total - used, handling the edge case where
/// a config change or level reduction leaves used > total.
///
/// Ported from MaNGOS Player::UpdateFreeTalentPoints() (Player.cpp:3200-3240).
///
/// # Arguments
/// * `state` - Mutable reference to player's talent state
/// * `level` - Player's current level
/// * `rate` - Talent rate multiplier from config
/// * `reset_if_needed` - If true and used > total, reset all talents (no cost)
///
/// # Returns
/// `true` if a forced reset was triggered
pub fn update_free_talent_points(
    state: &mut TalentState,
    level: u8,
    rate: f32,
    reset_if_needed: bool,
) -> bool {
    if level < 10 {
        // Below level 10: no talent points at all
        if state.used_talent_count > 0 && reset_if_needed {
            // Force reset all talents (admin/automatic)
            state.talents.clear();
            state.used_talent_count = 0;
            state.free_talent_points = 0;
            return true;
        }
        state.free_talent_points = 0;
        return false;
    }

    let total = calculate_total_talent_points(level, rate);

    if state.used_talent_count > total {
        // Player has used more points than available (config change, etc.)
        if reset_if_needed {
            state.talents.clear();
            state.used_talent_count = 0;
            state.free_talent_points = total;
            return true;
        } else {
            state.free_talent_points = 0;
            return false;
        }
    }

    state.free_talent_points = total - state.used_talent_count;
    false
}

/// Grant a talent point on level up.
///
/// Called from the level-up handler when a player reaches a new level.
/// If the player is level 10 or above, they gain a free talent point.
///
/// Ported from MaNGOS Player::InitTalentForLevel() (Player.cpp:3170-3178).
///
/// # Arguments
/// * `state` - Mutable reference to player's talent state
/// * `new_level` - The level the player just reached
/// * `rate` - Talent rate multiplier from config
pub fn on_level_up(state: &mut TalentState, new_level: u8, rate: f32) {
    update_free_talent_points(state, new_level, rate, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_talent_points_below_10() {
        assert_eq!(calculate_total_talent_points(1, 1.0), 0);
        assert_eq!(calculate_total_talent_points(9, 1.0), 0);
    }

    #[test]
    fn test_talent_points_at_level_boundaries() {
        assert_eq!(calculate_total_talent_points(10, 1.0), 1);
        assert_eq!(calculate_total_talent_points(20, 1.0), 11);
        assert_eq!(calculate_total_talent_points(60, 1.0), 51);
    }

    #[test]
    fn test_talent_points_with_rate() {
        // Double rate: level 10 gives 2 points, level 60 gives 102
        assert_eq!(calculate_total_talent_points(10, 2.0), 2);
        assert_eq!(calculate_total_talent_points(60, 2.0), 102);
    }

    #[test]
    fn test_talent_points_zero_rate() {
        assert_eq!(calculate_total_talent_points(60, 0.0), 0);
    }

    #[test]
    fn test_update_free_points_normal() {
        let mut state = TalentState::default();
        state.used_talent_count = 10;
        update_free_talent_points(&mut state, 30, 1.0, false);
        // Level 30 = 21 total, 10 used = 11 free
        assert_eq!(state.free_talent_points, 11);
    }

    #[test]
    fn test_update_free_points_overspent_reset() {
        let mut state = TalentState::default();
        state.used_talent_count = 60; // More than possible
        state.talents.insert(100, 5);
        let reset = update_free_talent_points(&mut state, 20, 1.0, true);
        assert!(reset);
        assert_eq!(state.used_talent_count, 0);
        assert!(state.talents.is_empty());
        assert_eq!(state.free_talent_points, 11); // Level 20 = 11 total
    }
}
