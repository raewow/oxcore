use super::state::EnvironmentalDamageType;

/// Safe fall distance in yards - falls under this threshold deal no damage.
/// This value corresponds to approximately 14.57 yards of vertical distance.
pub const SAFE_FALL_DISTANCE: f32 = 14.57;

/// Calculate fall damage based on vertical fall distance.
///
/// The vanilla WoW fall damage formula is a quadratic function of excess
/// distance beyond the safe threshold, expressed as a percentage of max health.
///
/// Formula:
///   excess = fall_distance - SAFE_FALL_DISTANCE
///   pct = min(excess^2 / 600, 1.0)
///   damage = max_health * pct
///
/// The Safe Fall aura (Rogue talent, enchant) reduces effective fall distance,
/// effectively raising the safe threshold.
///
/// # Arguments
/// * `fall_distance` - Total vertical distance fallen in yards
/// * `max_health` - Player's maximum health
/// * `safe_fall_bonus` - Extra safe distance from Safe Fall aura (yards)
///
/// # Returns
/// Damage to apply (0 if within safe distance)
pub fn calculate_fall_damage(fall_distance: f32, max_health: u32, safe_fall_bonus: f32) -> u32 {
    let effective_safe = SAFE_FALL_DISTANCE + safe_fall_bonus;

    if fall_distance <= effective_safe {
        return 0;
    }

    let excess = fall_distance - effective_safe;
    let pct = (excess * excess / 600.0).min(1.0);
    (max_health as f32 * pct) as u32
}

/// Handle a player landing after a fall.
///
/// Called from the movement handler when a fall-end movement info is received.
/// Checks for immunity conditions and applies damage.
///
/// # Conditions that prevent fall damage:
/// - Player is dead
/// - Player is in flight (taxi, flying mount)
/// - Player has SPELL_AURA_FLY aura
/// - Player is a game master (GM mode)
/// - Fall distance is within safe threshold
///
/// # Returns
/// The damage dealt (0 if no damage)
pub fn handle_fall_landing(
    is_alive: bool,
    is_taxi_flying: bool,
    is_game_master: bool,
    has_fly_aura: bool,
    fall_distance: f32,
    max_health: u32,
    safe_fall_bonus: f32,
) -> u32 {
    // Immunity checks
    if !is_alive {
        return 0;
    }
    if is_taxi_flying {
        return 0;
    }
    if is_game_master {
        return 0;
    }
    if has_fly_aura {
        return 0;
    }

    calculate_fall_damage(fall_distance, max_health, safe_fall_bonus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_damage_under_safe_distance() {
        assert_eq!(calculate_fall_damage(10.0, 1000, 0.0), 0);
        assert_eq!(calculate_fall_damage(14.57, 1000, 0.0), 0);
    }

    #[test]
    fn test_damage_increases_with_distance() {
        let d1 = calculate_fall_damage(20.0, 1000, 0.0);
        let d2 = calculate_fall_damage(30.0, 1000, 0.0);
        let d3 = calculate_fall_damage(40.0, 1000, 0.0);

        assert!(d1 > 0);
        assert!(d2 > d1);
        assert!(d3 > d2);
    }

    #[test]
    fn test_damage_capped_at_max_health() {
        // Very large fall should not exceed max_health
        let damage = calculate_fall_damage(200.0, 1000, 0.0);
        assert!(damage <= 1000);
    }

    #[test]
    fn test_safe_fall_bonus_extends_threshold() {
        // With 10 yards of Safe Fall bonus, 20 yards should be near-zero
        let without = calculate_fall_damage(20.0, 1000, 0.0);
        let with = calculate_fall_damage(20.0, 1000, 10.0);
        assert!(with < without);
    }

    #[test]
    fn test_exact_damage_values() {
        // fall_distance = 24.57, excess = 10.0
        // pct = 100 / 600 = 0.1667
        // damage = 1000 * 0.1667 = 166
        let damage = calculate_fall_damage(24.57, 1000, 0.0);
        assert_eq!(damage, 166);
    }
}
