//! Melee Attack Range Calculations
//!
//! Implements the MaNGOS melee reach formula:
//!   reach = attacker_reach + target_reach + BASE_MELEERANGE_OFFSET
//!   reach = max(reach, ATTACK_DISTANCE)
//!   if both_moving: reach += LEEWAY_BONUS_RANGE
//!   check: distance_2d(attacker, target) <= reach
//!
//! Reference: mangos/src/game/Objects/Unit.cpp GetCombatReachToTarget()

use crate::shared::protocol::Position;

/// Base melee range offset added to combined combat reaches (4/3 yards)
/// Matches MaNGOS BASE_MELEERANGE_OFFSET
pub const BASE_MELEERANGE_OFFSET: f32 = 1.333333373069763;

/// Minimum melee attack distance in yards
/// Matches MaNGOS ATTACK_DISTANCE
pub const ATTACK_DISTANCE: f32 = 5.0;

/// Bonus range when both attacker and target are moving (yards)
/// Matches MaNGOS LEEWAY_BONUS_RANGE
pub const LEEWAY_BONUS_RANGE: f32 = 2.66;

/// Default combat reach for players and creatures without model data
pub const DEFAULT_COMBAT_REACH: f32 = 1.5;

/// Maximum vertical (Z-axis) distance for melee attacks.
/// Matches vmangos CanReachWithMeleeAutoAttackAtPosition Z check.
pub const MELEE_Z_LIMIT: f32 = 5.0;

/// Calculate the effective melee reach between two units.
///
/// Formula: max(attacker_reach + target_reach + 1.33, 5.0) + leeway
pub fn get_melee_reach(attacker_reach: f32, target_reach: f32, both_moving: bool) -> f32 {
    let mut reach = attacker_reach + target_reach + BASE_MELEERANGE_OFFSET;
    if reach < ATTACK_DISTANCE {
        reach = ATTACK_DISTANCE;
    }
    if both_moving {
        reach += LEEWAY_BONUS_RANGE;
    }
    reach
}

/// Check if attacker can reach target with melee.
///
/// Uses the MaNGOS combat reach formula with 2D (XY) distance for horizontal
/// range and a separate Z-axis check for vertical distance.
/// Matches vmangos CanReachWithMeleeAutoAttackAtPosition:
///   (dx*dx + dy*dy < reach*reach) && ((dz*dz) < zReach)
pub fn is_within_melee_range(
    attacker_pos: &Position,
    attacker_reach: f32,
    target_pos: &Position,
    target_reach: f32,
    both_moving: bool,
) -> bool {
    let reach = get_melee_reach(attacker_reach, target_reach, both_moving);
    let dist_2d = attacker_pos.distance_2d(target_pos);
    let dz = (attacker_pos.z - target_pos.z).abs();
    dist_2d <= reach && dz <= MELEE_Z_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Position;

    fn pos(x: f32, y: f32, z: f32) -> Position {
        Position { x, y, z, o: 0.0 }
    }

    // --- get_melee_reach ---

    #[test]
    fn test_get_melee_reach_defaults_clamped_to_minimum() {
        // 1.5 + 1.5 + 1.333 = 4.333 < ATTACK_DISTANCE(5.0) → clamped
        let reach = get_melee_reach(DEFAULT_COMBAT_REACH, DEFAULT_COMBAT_REACH, false);
        assert_eq!(reach, ATTACK_DISTANCE);
    }

    #[test]
    fn test_get_melee_reach_leeway_adds_bonus() {
        let reach_static = get_melee_reach(DEFAULT_COMBAT_REACH, DEFAULT_COMBAT_REACH, false);
        let reach_moving = get_melee_reach(DEFAULT_COMBAT_REACH, DEFAULT_COMBAT_REACH, true);
        assert!((reach_moving - reach_static - LEEWAY_BONUS_RANGE).abs() < 0.001);
    }

    #[test]
    fn test_get_melee_reach_large_units_not_clamped() {
        // 2.0 + 2.0 + 1.333 = 5.333 > 5.0, no clamp
        let reach = get_melee_reach(2.0, 2.0, false);
        assert!(reach > ATTACK_DISTANCE);
        assert!((reach - (2.0 + 2.0 + BASE_MELEERANGE_OFFSET)).abs() < 0.001);
    }

    // --- is_within_melee_range: XY ---

    #[test]
    fn test_within_range_xy() {
        // 4 yards away, flat ground — well within default 5yd reach
        assert!(is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(4.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, false));
    }

    #[test]
    fn test_within_range_xy_boundary() {
        // Exactly at the reach boundary (5.0)
        assert!(is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(5.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, false));
    }

    #[test]
    fn test_outside_range_xy() {
        // 6 yards away — beyond the 5.0 minimum reach
        assert!(!is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(6.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, false));
    }

    // --- is_within_melee_range: Z-axis (Fix D) ---

    #[test]
    fn test_within_range_z_ok() {
        // XY = 0, Z diff = 4.9 < MELEE_Z_LIMIT(5.0) → in range
        assert!(is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(0.0, 0.0, 4.9), DEFAULT_COMBAT_REACH, false));
    }

    #[test]
    fn test_outside_range_z_too_high() {
        // XY = 0 (would be in XY range), but Z diff = 5.1 > MELEE_Z_LIMIT → out of range
        assert!(!is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(0.0, 0.0, 5.1), DEFAULT_COMBAT_REACH, false));
    }

    #[test]
    fn test_z_check_uses_absolute_diff() {
        // Attacker above target — same limit applies
        assert!(!is_within_melee_range(&pos(0.0, 0.0, 5.1), DEFAULT_COMBAT_REACH, &pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, false));
    }

    // --- is_within_melee_range: leeway ---

    #[test]
    fn test_leeway_extends_reach_when_both_moving() {
        // 7 yards: beyond static reach (5.0) but within leeway reach (7.66)
        assert!(!is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(7.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, false));
        assert!(is_within_melee_range(&pos(0.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, &pos(7.0, 0.0, 0.0), DEFAULT_COMBAT_REACH, true));
    }

    // --- distance_2d ignores Z ---

    #[test]
    fn test_distance_2d_ignores_z() {
        // 3-4-5 triangle in XY, Z=100 — distance_2d should still be 5.0
        let a = pos(0.0, 0.0, 0.0);
        let b = pos(3.0, 4.0, 100.0);
        assert!((a.distance_2d(&b) - 5.0).abs() < 0.001);
    }
}
