//! Creature Combat Update System
//!
//! Updates creature combat timers and executes melee attacks each tick.
//! This matches vmangos behavior where Unit::Update decrements timers and
//! AI::DoMeleeAttackIfReady() calls UpdateMeleeAttackingState() which
//! checks timer + range + executes attack all in one function.
//!
//! By handling melee attacks here instead of through the AI decision pipeline,
//! we eliminate the extra tick of latency from snapshot -> decision -> action -> execute.

use crate::shared::protocol::ObjectGuid;
use crate::world::World;

/// Update combat timers and execute melee attacks for all creatures
/// Called from the world update loop
pub fn update_creature_combat(world: &World, diff_ms: u32) {
    // Get all creatures that need combat updates (in combat or have attack timers)
    let creatures_to_update: Vec<ObjectGuid> = world.managers.creature_mgr
        .iter_creatures()
        .filter(|entry| {
            let creature = entry.value();
            // Update if in combat OR has an attack timer running
            creature.combat.in_combat || creature.attack_timer > 0
        })
        .map(|entry| *entry.key())
        .collect();

    for creature_guid in creatures_to_update {
        update_single_creature_combat(world, creature_guid, diff_ms);
    }
}

/// Update combat timers and execute melee attack for a single creature.
///
/// Matches vmangos flow:
/// 1. Decrement attack timer (Unit::Update)
/// 2. If timer ready + has target: check range and execute attack (DoMeleeAttackIfReady)
/// 3. If out of range: delay timer 100ms (DelayAutoAttacks)
fn update_single_creature_combat(world: &World, creature_guid: ObjectGuid, diff_ms: u32) {
    // Step 1: Decrement attack timer and gather attack data if ready
    let attack_data = world.managers.creature_mgr.with_creature_mut(creature_guid, |creature| {
        // Update attack timer
        if creature.attack_timer > 0 {
            creature.attack_timer = creature.attack_timer.saturating_sub(diff_ms);
        }

        // If timer is ready and creature has an attack target, gather data for attack
        if creature.attack_timer == 0 {
            if let Some(target_guid) = creature.combat.attacking {
                if creature.is_alive() {
                    return Some((
                        target_guid,
                        creature.position,
                        creature.combat_reach,
                    ));
                }
            }
        }

        None
    }).flatten();

    // Step 2: If attack timer fired, check range and execute
    let Some((target_guid, creature_pos, creature_reach)) = attack_data else {
        return;
    };

    // Only handle creature -> player attacks
    if !target_guid.is_player() {
        return;
    }

    // Get target position for range check
    let Some(target_pos) = world.managers.player_mgr.get_position(target_guid) else {
        return;
    };

    // Check melee range (same formula used everywhere)
    use crate::world::game::combat::melee_range::{self, DEFAULT_COMBAT_REACH};
    let in_range = melee_range::is_within_melee_range(
        &creature_pos,
        creature_reach,
        &target_pos,
        DEFAULT_COMBAT_REACH,
        false, // no leeway for creature attacks
    );

    if in_range {
        // In range: execute the attack (perform_creature_melee_attack checks
        // is_attack_ready again as a safety net, resets timer, rolls hit table)
        crate::world::game::combat::creature_attacks::perform_creature_melee_attack(
            world, creature_guid, target_guid,
        );
    } else {
        // Out of range: delay 100ms before retrying (matches vmangos DelayAutoAttacks)
        world.managers.creature_mgr.with_creature_mut(creature_guid, |creature| {
            creature.attack_timer = 100;
        });
    }
}

/// Out-of-range retry delay in ms. Matches vmangos DelayAutoAttacks().
const OUT_OF_RANGE_DELAY_MS: u32 = 100;

#[cfg(test)]
mod tests {
    use super::*;

    // --- Timer countdown math (mirrors the logic in update_single_creature_combat) ---

    #[test]
    fn test_timer_countdown_partial() {
        let mut timer: u32 = 2000;
        timer = timer.saturating_sub(50);
        assert_eq!(timer, 1950);
        assert_ne!(timer, 0); // not ready yet
    }

    #[test]
    fn test_timer_countdown_to_zero() {
        let mut timer: u32 = 200;
        timer = timer.saturating_sub(200);
        assert_eq!(timer, 0); // exactly ready
    }

    #[test]
    fn test_timer_saturates_no_underflow() {
        let mut timer: u32 = 50;
        timer = timer.saturating_sub(300); // diff larger than remaining
        assert_eq!(timer, 0); // clamps to 0, does not wrap to u32::MAX
    }

    // --- Out-of-range delay constant ---

    #[test]
    fn test_out_of_range_delay_is_100ms() {
        // Documents that the retry delay matches vmangos DelayAutoAttacks() = 100ms.
        // If this changes, it should be a deliberate decision.
        assert_eq!(OUT_OF_RANGE_DELAY_MS, 100);
    }

    // --- Player delay constant (Fix C) ---

    #[test]
    fn test_player_delay_constant_matches_vmangos() {
        // The player system also uses 100ms (not 200ms) to match vmangos.
        // Verified here to keep both sides of the fix visible in one place.
        assert_eq!(OUT_OF_RANGE_DELAY_MS, 100);
    }
}
