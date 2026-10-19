//! Honour formulas: ranks from rank points, kill values, the weekly standing
//! curve and rank-point decay.

/// Negative ranks (Pariah..Outlaw).
const NEGATIVE_RANKS: u8 = 4;
/// Positive ranks (Private..Grand Marshal) plus "none".
const POSITIVE_RANKS: u8 = 15;
/// All ranks, 0 = none.
pub const RANK_COUNT: u8 = 19;

/// A rank as stored (`rank`, 0..18) and as the client's rank bar shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HonorRankInfo {
    pub rank: u8,
    /// -4..14; positive ranks are 1 (Private) to 14 (Grand Marshal).
    pub visual_rank: i8,
    pub max_rp: f32,
    pub min_rp: f32,
    pub positive: bool,
}

impl Default for HonorRankInfo {
    fn default() -> Self {
        Self {
            rank: 0,
            visual_rank: 0,
            max_rp: 2000.0,
            min_rp: 0.0,
            positive: true,
        }
    }
}

/// Fill in the visual rank and rank-point bounds of a stored rank.
pub fn rank_info(rank: u8, positive: bool) -> HonorRankInfo {
    if rank == 0 {
        return HonorRankInfo::default();
    }
    let relative = if positive {
        rank as i32 - NEGATIVE_RANKS as i32 - 1
    } else {
        rank as i32 - NEGATIVE_RANKS as i32
    };
    let mut max_rp = (relative as f32 * 5000.0).abs();
    let mut min_rp = if max_rp > 5000.0 { max_rp - 5000.0 } else { 2000.0 };
    if rank == 5 {
        max_rp = 2000.0;
        min_rp = 0.0;
    }
    HonorRankInfo {
        rank,
        visual_rank: if rank > NEGATIVE_RANKS {
            (rank - NEGATIVE_RANKS) as i8
        } else {
            -(rank as i8)
        },
        max_rp,
        min_rp,
        positive,
    }
}

/// The rank that rank points earn.
pub fn calculate_rank(rank_points: f32) -> HonorRankInfo {
    if rank_points == 0.0 {
        return HonorRankInfo::default();
    }
    let positive = rank_points > 0.0;
    let points = rank_points.abs();
    let rank_count = if positive { POSITIVE_RANKS - 2 } else { NEGATIVE_RANKS };
    let first_rank = if positive { NEGATIVE_RANKS + 1 } else { 1 };
    let rank = if points < 2000.0 {
        if positive {
            first_rank
        } else {
            NEGATIVE_RANKS
        }
    } else if points > (rank_count as f32 - 1.0) * 5000.0 {
        if positive {
            RANK_COUNT - 1
        } else {
            first_rank
        }
    } else {
        let step = (points / 5000.0) as u8 + first_rank;
        if positive {
            step + 1
        } else {
            NEGATIVE_RANKS.saturating_sub(step)
        }
    };
    rank_info(rank, positive)
}

/// The XP level factor a kill's value is scaled by.
pub fn level_factor(killer_level: u32, victim_level: u32) -> f32 {
    if victim_level >= killer_level {
        let diff = (victim_level - killer_level).min(4);
        return 1.0 + 0.05 * diff as f32;
    }
    let gray = if killer_level <= 5 {
        0
    } else if killer_level <= 39 {
        killer_level - 5 - killer_level / 10
    } else {
        killer_level - 1 - killer_level / 5
    };
    if victim_level > gray {
        let zero_difference = match killer_level {
            0..=7 => 5,
            8..=9 => 6,
            10..=11 => 7,
            12..=15 => 8,
            16..=19 => 9,
            20..=29 => 11,
            30..=39 => 12,
            40..=44 => 13,
            45..=49 => 14,
            50..=54 => 15,
            55..=59 => 16,
            _ => 17,
        };
        return (zero_difference + victim_level - killer_level) as f32 / zero_difference as f32;
    }
    0.0
}

/// Contribution points for an honourable kill.
///
/// `victim_rank` is the victim's visual rank; `kills_today` how often the
/// killer already killed this victim today (each repeat is worth 10% less).
pub fn honor_gain(killer_level: u32, victim_level: u32, victim_rank: i32, kills_today: u32, group_size: u32) -> f32 {
    const SAME_VICTIM_PENALTY: f32 = 10.0;
    let same_victim = if kills_today as f32 >= SAME_VICTIM_PENALTY {
        0.0
    } else {
        1.0 - kills_today as f32 / SAME_VICTIM_PENALTY
    };
    let level_coeff = match killer_level {
        60.. => 1.0,
        50..=59 => 0.9545,
        40..=49 => 0.5707,
        30..=39 => 0.3434,
        20..=29 => 0.2070,
        _ => 0.1212,
    };
    let rank_value = 188.3 * (0.05331 * victim_rank as f64).exp();
    (level_coeff * same_victim as f64 * rank_value * level_factor(killer_level, victim_level) as f64
        / group_size.max(1) as f64) as f32
}

/// Rank points lost for a dishonourable kill.
pub fn dishonorable_kill_points(level: u32) -> f32 {
    let mut result = 10.0;
    if (30..=35).contains(&level) {
        result += 1.5 * (level - 29) as f32;
    }
    if (36..=41).contains(&level) {
        result += 9.0 + 2.0 * (level - 35) as f32;
    }
    if (42..=50).contains(&level) {
        result += 21.0 + 3.2 * (level - 41) as f32;
    }
    if level >= 51 {
        result += 50.0 + 4.0 * (level - 50) as f32;
    }
    result.min(100.0)
}

/// The weekly curve mapping contribution points to rank points earned,
/// built from the standings of one side.
#[derive(Debug, Clone)]
pub struct HonorScores {
    pub fx: [f32; 15],
    pub fy: [f32; 15],
    pub brk: [f32; 14],
}

/// Build the curve from contribution points sorted best first.
pub fn generate_scores(standing_cp: &[f32], pool_size_override: u32) -> HonorScores {
    let mut brk = [
        1.000, 0.845, 0.697, 0.566, 0.436, 0.327, 0.228, 0.159, 0.100, 0.060, 0.035, 0.020, 0.008, 0.003,
    ];
    let pool = if pool_size_override == 0 {
        standing_cp.len() as f32
    } else {
        pool_size_override as f32
    };
    for group in brk.iter_mut() {
        *group = (*group * pool + 0.5).floor();
    }
    let mut fy = [0.0f32; 15];
    fy[1] = 400.0;
    for (i, value) in fy.iter_mut().enumerate().take(14).skip(2) {
        *value = (i as f32 - 1.0) * 1000.0;
    }
    fy[14] = 13000.0;
    let cp_at = |position: f32| -> f32 {
        let position = position as usize;
        if position == 0 {
            return 0.0;
        }
        standing_cp.get(position - 1).copied().unwrap_or(0.0)
    };
    let top_cp = standing_cp.first().copied().unwrap_or(0.0);
    let mut fx = [0.0f32; 15];
    let mut top = false;
    for i in 1..=13 {
        let mut honor = 0.0;
        let first = cp_at(brk[i]);
        if first != 0.0 {
            honor += first;
            let second = cp_at(brk[i] + 1.0);
            if second != 0.0 {
                honor += second;
            }
        }
        fx[i] = if honor != 0.0 { honor / 2.0 } else { 0.0 };
        if !top && honor == 0.0 {
            fx[i] = if fx[i - 1] != 0.0 { top_cp } else { 0.0 };
            top = true;
        }
    }
    fx[14] = if !top { top_cp } else { 0.0 };
    HonorScores { fx, fy, brk }
}

/// Rank points earned this week for `cp` contribution points.
pub fn rp_earning(cp: f32, sc: &HonorScores) -> f32 {
    let mut i = 0;
    while i < 14 && sc.brk[i] > 0.0 && sc.fx[i] <= cp {
        i += 1;
    }
    if i > 0 && sc.fx[i] > cp && cp >= sc.fx[i - 1] {
        return (sc.fy[i] - sc.fy[i - 1]) * (cp - sc.fx[i - 1]) / (sc.fx[i] - sc.fx[i - 1]) + sc.fy[i - 1];
    }
    sc.fy[i]
}

/// Rank points after a week: earnings against `decay` of what was held.
pub fn rp_after_decay(earning: f32, rp: f32, decay: f32) -> f32 {
    let lost = (decay * rp + 0.5).floor();
    let mut delta = earning - lost;
    if delta < 0.0 {
        delta /= 2.0;
    }
    rp + delta.max(-2500.0)
}

/// The most rank points a character of `level` may hold.
pub fn max_rp_at_level(level: u32) -> f32 {
    match level {
        0..=29 => 6500.0,
        30..=35 => 7150.0 + 975.0 * (level - 30) as f32,
        36..=39 => 12025.0 + 1300.0 * (level - 35) as f32,
        40..=43 => 17225.0 + 1625.0 * (level - 39) as f32,
        44..=52 => 23725.0 + 2275.0 * (level - 43) as f32,
        53..=60 => 44200.0 + 2600.0 * (level - 52) as f32,
        _ => 65000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_points_map_to_ranks() {
        assert_eq!(calculate_rank(0.0).rank, 0);
        // Under 2000 is Private (rank 5, visual 1).
        let private = calculate_rank(100.0);
        assert_eq!((private.rank, private.visual_rank), (5, 1));
        // 5000 per rank after that.
        assert_eq!(calculate_rank(2000.0).rank, 6);
        assert_eq!(calculate_rank(5000.0).rank, 7);
        assert_eq!(calculate_rank(60_000.0).rank, 18);
        assert_eq!(calculate_rank(60_000.0).visual_rank, 14);
    }

    #[test]
    fn kills_are_worth_less_each_time_and_nothing_when_gray() {
        let first = honor_gain(60, 60, 0, 0, 1);
        assert!((first - 188.3).abs() < 0.1, "{first}");
        let fifth = honor_gain(60, 60, 0, 4, 1);
        assert!((fifth - first * 0.6).abs() < 0.1);
        assert_eq!(honor_gain(60, 60, 0, 10, 1), 0.0);
        assert_eq!(honor_gain(60, 40, 0, 0, 1), 0.0);
        assert!((honor_gain(60, 60, 0, 0, 2) - first / 2.0).abs() < 0.1);
    }

    #[test]
    fn dishonourable_kills_cost_more_at_higher_levels() {
        assert_eq!(dishonorable_kill_points(10), 10.0);
        assert_eq!(dishonorable_kill_points(60), 100.0);
        assert!(dishonorable_kill_points(40) > dishonorable_kill_points(30));
    }

    #[test]
    fn the_weekly_curve_interpolates_between_breakpoints() {
        let standings: Vec<f32> = (0..100).map(|i| 10_000.0 - i as f32 * 100.0).collect();
        let scores = generate_scores(&standings, 0);
        assert_eq!(rp_earning(0.0, &scores), 0.0);
        let top = rp_earning(10_000.0, &scores);
        assert!(top >= 12_000.0, "{top}");
        let mid = rp_earning(5_000.0, &scores);
        assert!(mid > 0.0 && mid < top);
    }

    #[test]
    fn decay_halves_losses_and_caps_them() {
        assert_eq!(rp_after_decay(0.0, 10_000.0, 0.2), 9_000.0);
        assert_eq!(rp_after_decay(5_000.0, 0.0, 0.2), 5_000.0);
        assert_eq!(rp_after_decay(0.0, 60_000.0, 0.2), 57_500.0);
        assert_eq!(max_rp_at_level(60), 65000.0);
    }
}
