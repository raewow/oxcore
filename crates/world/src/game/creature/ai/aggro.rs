//! Proximity aggro rules: how far a creature notices a target, whether it sees
//! through stealth, and whether it may start the fight at all.

/// "The maximum aggro radius has a cap of 25 levels under."
pub const MAX_LEVEL_DIFF_FOR_AGGRO_RANGE: i32 = 25;
/// Creatures that cannot fly ignore targets further away than this vertically.
pub const CREATURE_Z_ATTACK_RANGE: f32 = 3.0;
/// Beyond this a creature never sees a stealthed unit.
pub const MAX_CREATURE_STEALTH_DETECT_RANGE: f32 = 30.0;
/// Minimum threat radius; the attack distance is used instead when larger.
pub const THREAT_RADIUS: f32 = 50.0;

/// NPC flags
pub const NPC_FLAG_GOSSIP: u32 = 0x00000001;
pub const NPC_FLAG_QUEST_GIVER: u32 = 0x00000002;
pub const NPC_FLAG_VENDOR: u32 = 0x00000080;
pub const NPC_FLAG_TRAINER: u32 = 0x00000010;

/// Whether the creature offers any NPC service.
pub fn is_npc(npc_flags: u32) -> bool {
    npc_flags & (NPC_FLAG_GOSSIP | NPC_FLAG_QUEST_GIVER | NPC_FLAG_VENDOR | NPC_FLAG_TRAINER) != 0
}

/// Distance at which a creature attacks a target that comes close.
///
/// Starts from the creature's detection range (about 18 yards against a target
/// of the same level), grows by one yard per level the target is below the
/// creature (capped at 25 levels) and shrinks by one yard per level above it,
/// never below min(detection range, 5). Detect-range auras on the creature and
/// detected-range auras on the target adjust it.
pub fn attack_distance(
    detection_range: f32,
    creature_level: u8,
    target_level: u8,
    creature_detect_range_mod: i32,
    target_detected_range_mod: i32,
) -> f32 {
    if detection_range < 1.0 {
        return 0.0;
    }
    let level_diff =
        (target_level as i32 - creature_level as i32).max(-MAX_LEVEL_DIFF_FOR_AGGRO_RANGE);
    let mut distance = detection_range - level_diff as f32;
    distance += creature_detect_range_mod as f32;
    distance += target_detected_range_mod as f32;
    distance.max(detection_range.min(5.0))
}

/// Whether a creature notices a stealthed unit `distance` yards away.
///
/// A unit closer than 1.5 yards is always noticed. Otherwise the creature's
/// detection skill (5 per level plus stealth-detection auras) is weighed
/// against the stealth skill at 5/6 yard per skill level difference (doubled
/// when the creature is more than three levels higher), capped at 30 yards and
/// reduced by 9 yards when the unit is behind the creature.
pub fn detects_stealth(
    creature_level: u8,
    creature_stealth_detect: i32,
    target_level: u8,
    target_stealth: i32,
    distance: f32,
    target_in_front: bool,
) -> bool {
    if distance < 1.5 {
        return true;
    }
    if distance > MAX_CREATURE_STEALTH_DETECT_RANGE {
        return false;
    }
    let mut yards_per_level = 5.0 / 6.0;
    let detect_skill = creature_level as i32 * 5 + creature_stealth_detect;
    if creature_level as i32 - target_level as i32 > 3 {
        yards_per_level *= 2.0;
    }
    let mut visible = 5.0 / 6.0 + (detect_skill - target_stealth) as f32 * yards_per_level / 5.0;
    visible = visible.clamp(0.0, 30.0);
    if !target_in_front {
        visible -= 9.0;
    }
    distance <= visible
}

/// Whether `target` lies within the creature's 180° front arc.
pub fn is_in_front(orientation: f32, from: (f32, f32), to: (f32, f32)) -> bool {
    let angle = (to.1 - from.1).atan2(to.0 - from.0);
    let mut diff = angle - orientation;
    while diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    while diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }
    diff.abs() <= std::f32::consts::FRAC_PI_2
}

/// Radius around the combat start the fight must stay within.
pub fn threat_area_radius(attack_distance: f32) -> f32 {
    (attack_distance * 1.5).max(THREAT_RADIUS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_level_uses_the_detection_range() {
        assert_eq!(attack_distance(18.0, 10, 10, 0, 0), 18.0);
    }

    #[test]
    fn lower_level_targets_are_noticed_from_further_away() {
        assert_eq!(attack_distance(18.0, 15, 10, 0, 0), 23.0);
        assert_eq!(attack_distance(18.0, 60, 1, 0, 0), 43.0, "capped at 25 levels");
    }

    #[test]
    fn higher_level_targets_shrink_to_five_yards() {
        assert_eq!(attack_distance(18.0, 5, 10, 0, 0), 13.0);
        assert_eq!(attack_distance(18.0, 1, 60, 0, 0), 5.0);
        assert_eq!(attack_distance(3.0, 1, 60, 0, 0), 3.0);
    }

    #[test]
    fn no_detection_range_means_no_proximity_aggro() {
        assert_eq!(attack_distance(0.0, 10, 10, 0, 0), 0.0);
    }

    #[test]
    fn stealth_is_seen_only_up_close() {
        // Same-level rogue with base stealth: about 0.8 yards plus the collision radius.
        assert!(detects_stealth(4, 0, 4, 20, 1.4, true));
        assert!(!detects_stealth(4, 0, 4, 20, 5.0, true));
        // A level 4 creature against a level 1 rogue notices it at about 3.2 yards.
        assert!(detects_stealth(4, 0, 1, 5, 3.0, true));
        assert!(!detects_stealth(4, 0, 1, 5, 4.0, true));
    }

    #[test]
    fn front_arc() {
        assert!(is_in_front(0.0, (0.0, 0.0), (5.0, 1.0)));
        assert!(!is_in_front(0.0, (0.0, 0.0), (-5.0, 0.0)));
    }

    #[test]
    fn threat_area_never_shrinks_below_fifty_yards() {
        assert_eq!(threat_area_radius(18.0), 50.0);
        assert_eq!(threat_area_radius(40.0), 60.0);
    }
}
