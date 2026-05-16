//! Auto-Attack Loop - Swing timer management and attack execution
//!
//! Handles main-hand, off-hand, and ranged swing timers.

use super::state::{AttackHand, CombatState};
use crate::shared::protocol::ObjectGuid;

/// A pending attack ready to be executed
#[derive(Debug, Clone)]
pub struct PendingAttack {
    pub attacker: ObjectGuid,
    pub target: ObjectGuid,
    pub hand: AttackHand,
}

/// Update auto-attack timers and return any attacks that should execute
/// Called every world tick (typically 50ms)
pub fn update_auto_attack(
    player_guid: ObjectGuid,
    diff_ms: u32,
    combat: &mut CombatState,
) -> Vec<PendingAttack> {
    let mut attacks = Vec::new();

    if !combat.is_auto_attacking {
        return attacks;
    }

    let target = match combat.attack_target {
        Some(t) => t,
        None => return attacks,
    };

    // Main hand timer
    if combat.main_hand_timer > 0 {
        combat.main_hand_timer = combat.main_hand_timer.saturating_sub(diff_ms);
    }

    if combat.main_hand_timer == 0 {
        attacks.push(PendingAttack {
            attacker: player_guid,
            target,
            hand: AttackHand::MainHand,
        });
        // Reset timer
        combat.main_hand_timer = combat.main_hand_speed;
    }

    // Off hand timer (if dual wielding)
    if combat.can_dual_wield {
        if combat.off_hand_timer > 0 {
            combat.off_hand_timer = combat.off_hand_timer.saturating_sub(diff_ms);
        }
        if combat.off_hand_timer == 0 {
            attacks.push(PendingAttack {
                attacker: player_guid,
                target,
                hand: AttackHand::OffHand,
            });
            combat.off_hand_timer = combat.off_hand_speed;
        }
    }

    attacks
}

/// Update auto-shoot (ranged) timer
pub fn update_auto_shoot(
    player_guid: ObjectGuid,
    diff_ms: u32,
    combat: &mut CombatState,
) -> Option<PendingAttack> {
    if !combat.is_auto_shooting {
        return None;
    }

    let target = match combat.attack_target {
        Some(t) => t,
        None => return None,
    };

    // Ranged timer
    if combat.ranged_timer > 0 {
        combat.ranged_timer = combat.ranged_timer.saturating_sub(diff_ms);
    }

    if combat.ranged_timer == 0 {
        // Reset timer
        combat.ranged_timer = combat.ranged_speed;

        return Some(PendingAttack {
            attacker: player_guid,
            target,
            hand: AttackHand::Ranged,
        });
    }

    None
}

/// Reset swing timer for a specific hand (e.g., after spell cast)
pub fn reset_swing_timer(combat: &mut CombatState, hand: AttackHand) {
    match hand {
        AttackHand::MainHand => {
            combat.main_hand_timer = combat.main_hand_speed;
        }
        AttackHand::OffHand => {
            combat.off_hand_timer = combat.off_hand_speed;
        }
        AttackHand::Ranged => {
            combat.ranged_timer = combat.ranged_speed;
        }
    }
}

/// Delay swing timers (e.g., when stunned or casting)
pub fn pause_swing_timers(combat: &mut CombatState) {
    // Timers are already paused by not updating them
    // This function is a placeholder for future pause logic
}

/// Resume swing timers after pause
pub fn resume_swing_timers(combat: &mut CombatState) {
    // Placeholder for resume logic
}

/// Calculate haste-affected swing speed
pub fn apply_haste(base_speed_ms: u32, haste_pct: f32) -> u32 {
    if haste_pct <= 0.0 {
        return base_speed_ms;
    }

    // Haste reduces swing time: new_time = base / (1 + haste/100)
    let multiplier = 1.0 + (haste_pct / 100.0);
    ((base_speed_ms as f32) / multiplier) as u32
}

/// Set new attack speed and adjust current timer proportionally
pub fn adjust_attack_speed(combat: &mut CombatState, hand: AttackHand, new_speed_ms: u32) {
    match hand {
        AttackHand::MainHand => {
            let old_speed = combat.main_hand_speed;
            combat.main_hand_speed = new_speed_ms;

            // Adjust current timer proportionally
            if old_speed > 0 {
                let progress = 1.0 - (combat.main_hand_timer as f32 / old_speed as f32);
                combat.main_hand_timer = (new_speed_ms as f32 * (1.0 - progress)) as u32;
            }
        }
        AttackHand::OffHand => {
            let old_speed = combat.off_hand_speed;
            combat.off_hand_speed = new_speed_ms;

            if old_speed > 0 {
                let progress = 1.0 - (combat.off_hand_timer as f32 / old_speed as f32);
                combat.off_hand_timer = (new_speed_ms as f32 * (1.0 - progress)) as u32;
            }
        }
        AttackHand::Ranged => {
            let old_speed = combat.ranged_speed;
            combat.ranged_speed = new_speed_ms;

            if old_speed > 0 {
                let progress = 1.0 - (combat.ranged_timer as f32 / old_speed as f32);
                combat.ranged_timer = (new_speed_ms as f32 * (1.0 - progress)) as u32;
            }
        }
    }
}

/// Check if player can attack (has target, in range, etc.)
pub fn can_attack(combat: &CombatState) -> bool {
    if !combat.is_auto_attacking {
        return false;
    }
    combat.attack_target.is_some()
}

/// Check if player can shoot (has ranged weapon, ammo, etc.)
pub fn can_shoot(combat: &CombatState) -> bool {
    if !combat.is_auto_shooting {
        return false;
    }
    combat.has_ranged_weapon && combat.attack_target.is_some()
}

/// Get time until next swing in milliseconds
pub fn time_until_next_swing(combat: &CombatState, hand: AttackHand) -> u32 {
    match hand {
        AttackHand::MainHand => combat.main_hand_timer,
        AttackHand::OffHand => combat.off_hand_timer,
        AttackHand::Ranged => combat.ranged_timer,
    }
}

/// Initialize swing timers when starting combat
pub fn initialize_swing_timers(combat: &mut CombatState) {
    // Main hand starts immediately
    if combat.main_hand_timer == 0 {
        combat.main_hand_timer = combat.main_hand_speed;
    }

    // Off hand starts at half speed offset (for visual variety)
    if combat.can_dual_wield && combat.off_hand_timer == 0 {
        combat.off_hand_timer = combat.off_hand_speed / 2;
    }

    // Ranged starts immediately
    if combat.ranged_timer == 0 {
        combat.ranged_timer = combat.ranged_speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_auto_attack() {
        let guid = ObjectGuid::from_raw(0x00000000_00000001);
        let target = ObjectGuid::from_raw(0x00000000_00000002);

        let mut combat = CombatState::default();
        combat.is_auto_attacking = true;
        combat.attack_target = Some(target);
        combat.main_hand_speed = 2000;
        combat.main_hand_timer = 1000;

        // Update with 500ms - not ready yet
        let attacks = update_auto_attack(guid, 500, &mut combat);
        assert!(attacks.is_empty());
        assert_eq!(combat.main_hand_timer, 500);

        // Update with 600ms - ready to swing
        let attacks = update_auto_attack(guid, 600, &mut combat);
        assert_eq!(attacks.len(), 1);
        assert_eq!(attacks[0].hand, AttackHand::MainHand);
        assert_eq!(combat.main_hand_timer, combat.main_hand_speed);
    }

    #[test]
    fn test_dual_wield_attack() {
        let guid = ObjectGuid::from_raw(0x00000000_00000001);
        let target = ObjectGuid::from_raw(0x00000000_00000002);

        let mut combat = CombatState::default();
        combat.is_auto_attacking = true;
        combat.attack_target = Some(target);
        combat.can_dual_wield = true;
        combat.main_hand_speed = 2000;
        combat.off_hand_speed = 2000;
        combat.main_hand_timer = 0; // Ready immediately
        combat.off_hand_timer = 0; // Ready immediately

        let attacks = update_auto_attack(guid, 0, &mut combat);

        // Should have both main and off hand attacks
        assert_eq!(attacks.len(), 2);
        assert!(attacks.iter().any(|a| a.hand == AttackHand::MainHand));
        assert!(attacks.iter().any(|a| a.hand == AttackHand::OffHand));
    }

    #[test]
    fn test_apply_haste() {
        let base = 2000u32;

        // No haste
        let speed = apply_haste(base, 0.0);
        assert_eq!(speed, 2000);

        // 50% haste
        let speed = apply_haste(base, 50.0);
        assert_eq!(speed, 1333); // 2000 / 1.5

        // 100% haste
        let speed = apply_haste(base, 100.0);
        assert_eq!(speed, 1000); // 2000 / 2.0
    }

    #[test]
    fn test_adjust_attack_speed() {
        let mut combat = CombatState::default();
        combat.main_hand_speed = 2000;
        combat.main_hand_timer = 1000; // Halfway through

        // Double the speed (haste buff)
        adjust_attack_speed(&mut combat, AttackHand::MainHand, 1000);

        assert_eq!(combat.main_hand_speed, 1000);
        // Timer should be adjusted proportionally (was 50% through, so should be ~500)
        assert_eq!(combat.main_hand_timer, 500);
    }

    #[test]
    fn test_auto_shoot() {
        let guid = ObjectGuid::from_raw(0x00000000_00000001);
        let target = ObjectGuid::from_raw(0x00000000_00000002);

        let mut combat = CombatState::default();
        combat.is_auto_shooting = true;
        combat.attack_target = Some(target);
        combat.ranged_speed = 2800;
        combat.ranged_timer = 100;

        // Not ready yet
        let attack = update_auto_shoot(guid, 50, &mut combat);
        assert!(attack.is_none());

        // Ready
        let attack = update_auto_shoot(guid, 100, &mut combat);
        assert!(attack.is_some());
        assert_eq!(attack.unwrap().hand, AttackHand::Ranged);
    }

    // --- Timer reset accuracy ---

    #[test]
    fn test_timer_resets_to_speed_exactly_after_swing() {
        // Verifies no drift: timer must be reset to exactly main_hand_speed
        let guid = ObjectGuid::from_raw(0x1);
        let target = ObjectGuid::from_raw(0x2);
        let mut combat = CombatState::default();
        combat.is_auto_attacking = true;
        combat.attack_target = Some(target);
        combat.main_hand_speed = 2000;
        combat.main_hand_timer = 0;

        let attacks = update_auto_attack(guid, 1, &mut combat);
        assert_eq!(attacks.len(), 1);
        assert_eq!(combat.main_hand_timer, 2000);
    }

    #[test]
    fn test_timer_saturates_at_zero_no_underflow() {
        // diff > remaining timer: should hit 0, not wrap/underflow
        let guid = ObjectGuid::from_raw(0x1);
        let target = ObjectGuid::from_raw(0x2);
        let mut combat = CombatState::default();
        combat.is_auto_attacking = true;
        combat.attack_target = Some(target);
        combat.main_hand_speed = 2000;
        combat.main_hand_timer = 50;

        // 100ms diff > 50ms remaining
        let attacks = update_auto_attack(guid, 100, &mut combat);
        assert_eq!(attacks.len(), 1, "should fire when timer saturates to 0");
    }

    // --- Guard conditions ---

    #[test]
    fn test_no_attack_without_target() {
        let guid = ObjectGuid::from_raw(0x1);
        let mut combat = CombatState::default();
        combat.is_auto_attacking = true;
        combat.attack_target = None; // no target
        combat.main_hand_timer = 0;

        let attacks = update_auto_attack(guid, 50, &mut combat);
        assert!(attacks.is_empty());
    }

    #[test]
    fn test_no_attack_when_not_auto_attacking() {
        let guid = ObjectGuid::from_raw(0x1);
        let target = ObjectGuid::from_raw(0x2);
        let mut combat = CombatState::default();
        combat.is_auto_attacking = false; // flag off
        combat.attack_target = Some(target);
        combat.main_hand_timer = 0;

        let attacks = update_auto_attack(guid, 50, &mut combat);
        assert!(attacks.is_empty());
    }

    // --- First swing fires immediately on start ---

    #[test]
    fn test_first_swing_fires_on_next_tick_after_start() {
        // start_attack() leaves main_hand_timer=0 so first swing fires on the
        // very next update call (even with diff=1ms). This is the intended
        // behavior: no artificial delay before the first hit.
        let guid = ObjectGuid::from_raw(0x1);
        let target = ObjectGuid::from_raw(0x2);
        let mut combat = CombatState::default();
        combat.main_hand_speed = 2000;
        // Simulate start_attack()
        combat.start_attack(target);
        assert_eq!(
            combat.main_hand_timer, 0,
            "timer should be 0 after start_attack"
        );

        let attacks = update_auto_attack(guid, 1, &mut combat);
        assert_eq!(
            attacks.len(),
            1,
            "first swing must fire on the very next tick"
        );
    }
}
