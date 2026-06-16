/// One gold expressed in copper (the base currency unit).
const GOLD: u32 = 10000;

/// Seconds in one month (30 days), used for reset cost decay.
const MONTH_SECONDS: u64 = 30 * 24 * 3600; // 2,592,000

/// Calculate the gold cost for a talent reset.
///
/// The cost escalates with each paid reset and decays over time:
///
/// | Reset # | Cost    |
/// |---------|---------|
/// | 1st     | 1 gold  |
/// | 2nd     | 5 gold  |
/// | 3rd     | 10 gold |
/// | 4th     | 15 gold |
/// | ...     | ...     |
/// | 10th+   | 50 gold |
///
/// After each reset, the multiplier increases by 1.
/// Each month (30 days) without a reset, the multiplier decays by 1.
/// The multiplier never drops below the configured minimum (default 2).
///
/// Ported from MaNGOS Player::GetResetTalentsCost() (Player.cpp:3050-3100).
///
/// # Arguments
/// * `multiplier` - Current reset cost multiplier (0 = first reset)
/// * `base_cost` - Gold cost for first reset (default 1, from config)
/// * `multi_cost` - Gold per multiplier step (default 5, from config)
/// * `max_multi` - Maximum multiplier cap (default 10, from config)
///
/// # Returns
/// Cost in copper (multiply by 10000 for gold)
pub fn calculate_reset_cost(
    multiplier: u32,
    base_cost: u32,
    multi_cost: u32,
    max_multi: u32,
) -> u32 {
    if multiplier == 0 {
        // First reset: base cost (default 1 gold)
        base_cost * GOLD
    } else {
        // Subsequent resets: multiplier * multi_cost gold, capped
        let effective = multiplier.min(max_multi);
        effective * multi_cost * GOLD
    }
}

/// Decay the reset cost multiplier based on elapsed time.
///
/// For each full month (30 days) since the last reset, the multiplier
/// decreases by 1. A minimum floor is enforced if the multiplier was
/// already at or above that floor before decay.
///
/// Ported from MaNGOS Player::UpdateResetTalentsMultiplier()
/// (Player.cpp:3110-3140).
///
/// # Arguments
/// * `multiplier` - Current multiplier value
/// * `last_reset` - Unix timestamp of last reset
/// * `now` - Current Unix timestamp
/// * `min_multiplier` - Floor value (default 2, from config)
///
/// # Returns
/// New multiplier value after decay
pub fn decay_multiplier(multiplier: u32, last_reset: u64, now: u64, min_multiplier: u32) -> u32 {
    if now <= last_reset {
        return multiplier;
    }

    let months_elapsed = (now - last_reset) / MONTH_SECONDS;
    if months_elapsed == 0 {
        return multiplier;
    }

    // Track if we should clamp to minimum
    let was_above_min = multiplier >= min_multiplier;

    let decayed = multiplier.saturating_sub(months_elapsed as u32);

    // If multiplier was at/above minimum, don't let it drop below
    if was_above_min && decayed < min_multiplier {
        min_multiplier
    } else {
        decayed
    }
}

/// Configuration for talent reset costs.
///
/// Loaded from worldserver.conf settings:
///   - RespecBaseCost = 1 (gold for first reset)
///   - RespecMultiplicativeCost = 5 (gold per multiplier step)
///   - RespecMinMultiplier = 2 (minimum multiplier after decay)
///   - RespecMaxMultiplier = 10 (cap on multiplier, 10 * 5 = 50g max)
///   - NoRespecPriceDecay = false (disable decay for testing)
#[derive(Debug, Clone)]
pub struct ResetCostConfig {
    pub base_cost: u32,
    pub multi_cost: u32,
    pub min_multiplier: u32,
    pub max_multiplier: u32,
    pub no_decay: bool,
}

impl Default for ResetCostConfig {
    fn default() -> Self {
        Self {
            base_cost: 1,
            multi_cost: 5,
            min_multiplier: 2,
            max_multiplier: 10,
            no_decay: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_reset_cost() {
        assert_eq!(calculate_reset_cost(0, 1, 5, 10), 1 * GOLD);
    }

    #[test]
    fn test_second_reset_cost() {
        assert_eq!(calculate_reset_cost(1, 1, 5, 10), 5 * GOLD);
    }

    #[test]
    fn test_escalating_cost() {
        assert_eq!(calculate_reset_cost(2, 1, 5, 10), 10 * GOLD);
        assert_eq!(calculate_reset_cost(5, 1, 5, 10), 25 * GOLD);
        assert_eq!(calculate_reset_cost(10, 1, 5, 10), 50 * GOLD);
    }

    #[test]
    fn test_cost_cap() {
        // multiplier 15 capped at max_multi 10
        assert_eq!(calculate_reset_cost(15, 1, 5, 10), 50 * GOLD);
    }

    #[test]
    fn test_decay_one_month() {
        let last_reset = 0u64;
        let now = MONTH_SECONDS;
        assert_eq!(decay_multiplier(5, last_reset, now, 2), 4);
    }

    #[test]
    fn test_decay_multiple_months() {
        let last_reset = 0u64;
        let now = 3 * MONTH_SECONDS;
        assert_eq!(decay_multiplier(5, last_reset, now, 2), 2);
    }

    #[test]
    fn test_decay_floor() {
        let last_reset = 0u64;
        let now = 10 * MONTH_SECONDS; // 10 months
                                      // Was at 5, above min 2 => clamp to 2
        assert_eq!(decay_multiplier(5, last_reset, now, 2), 2);
    }

    #[test]
    fn test_decay_below_min_if_never_above() {
        let last_reset = 0u64;
        let now = 2 * MONTH_SECONDS;
        // Was at 1 (below min 2), decays to 0 (no clamping)
        assert_eq!(decay_multiplier(1, last_reset, now, 2), 0);
    }

    #[test]
    fn test_no_decay_without_reset() {
        assert_eq!(decay_multiplier(5, 0, 0, 2), 5);
    }
}
