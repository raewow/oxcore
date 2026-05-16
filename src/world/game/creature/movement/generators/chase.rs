//! Chase movement generator - follows a target with distance-based repathing
//!
//! Matches vmangos TargetedMovementGenerator behavior:
//! - 100ms recheck interval (m_checkDistanceTimer)
//! - Compares target's last-known position vs current position to decide repath
//! - Only repaths when target has actually moved, not when contact point angle changes
//! - 500ms minimum between full path recalculations to avoid jitter

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};

/// Chase movement - follows a target with melee-range awareness
pub struct ChaseMovementGenerator {
    pub target: ObjectGuid,
    target_position: Position,
    /// Creature's current position, updated each tick by system.rs
    creature_position: Position,
    /// Creature's combat reach (from creature data)
    creature_combat_reach: f32,
    /// Whether we currently have an active spline moving toward the target
    is_moving: bool,
    /// Has reached target at least once
    pub reached_target: bool,

    /// Distance check timer - how often we CHECK if target moved (counts down, 100ms interval)
    check_distance_timer: u32,
    /// Target position at the time of the last path calculation
    /// (vmangos: m_fTargetLastX/Y/Z - set in _setTargetLocation)
    target_last_path_pos: Option<Position>,
    /// Run speed in yards/sec (from creature's speed_run rate * 7.0)
    run_speed: f32,
}

/// Default combat reach for players (used when target reach is unknown)
const DEFAULT_TARGET_COMBAT_REACH: f32 = 1.5;

/// How often to check if target has moved (vmangos: m_checkDistanceTimer reset to 100ms)
const CHECK_DISTANCE_INTERVAL: u32 = 100;

impl ChaseMovementGenerator {
    pub fn new(target: ObjectGuid, creature_combat_reach: f32, run_speed: f32) -> Self {
        Self {
            target,
            target_position: Position::default(),
            creature_position: Position::default(),
            creature_combat_reach,
            is_moving: false,
            reached_target: false,
            check_distance_timer: 0, // Check immediately on first update
            target_last_path_pos: None,
            run_speed,
        }
    }

    /// Update target position from world state
    pub fn update_target_position(&mut self, pos: Position) {
        self.target_position = pos;
    }

    /// Update creature's current position from world state
    pub fn set_creature_position(&mut self, pos: Position) {
        self.creature_position = pos;
    }

    /// Calculate melee reach using the same formula as AI decision
    fn get_melee_reach(&self) -> f32 {
        use crate::world::game::combat::melee_range;
        melee_range::get_melee_reach(
            self.creature_combat_reach,
            melee_range::DEFAULT_COMBAT_REACH, // target (player) default reach
            false,
        )
    }

    /// Calculate contact point - the point on the target's boundary facing the chaser.
    /// Matches MaNGOS GetContactPoint behavior.
    fn get_contact_point(&self) -> Position {
        let dx = self.creature_position.x - self.target_position.x;
        let dy = self.creature_position.y - self.target_position.y;
        let angle = dy.atan2(dx);

        // Distance from target center to the contact point
        // MaNGOS: combatReach + targetCombatReach - targetBounding - ownerBounding - 1.0
        // Simplified: use combat reach sum minus some offset, min 0.5
        let contact_dist =
            (self.creature_combat_reach + DEFAULT_TARGET_COMBAT_REACH - 1.0).max(0.5);

        Position {
            x: self.target_position.x + angle.cos() * contact_dist,
            y: self.target_position.y + angle.sin() * contact_dist,
            z: self.target_position.z,
            o: 0.0,
        }
    }

    /// Check if the target has moved enough from its position at last path calculation
    /// to warrant a repath. vmangos: compares m_fTargetLastX/Y/Z vs current target pos,
    /// then checks allowed_dist = GetMaxChaseDistance.
    fn target_moved_enough(&self) -> bool {
        let Some(last_pos) = self.target_last_path_pos else {
            return true; // Never pathed, need initial path
        };

        let dx = self.target_position.x - last_pos.x;
        let dy = self.target_position.y - last_pos.y;
        let dist_sq = dx * dx + dy * dy;

        // vmangos: allowed_dist = GetMaxChaseDistance which is combatReach + targetCombatReach + RECALCULATION_RANGE
        // We use a simpler threshold: repath when target moved > 2.0 yards from where it was
        // when we last calculated a path. This prevents constant repathing when the player
        // is strafing in small circles.
        dist_sq > 4.0 // 2.0^2
    }
}

impl MovementGenerator for ChaseMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Chase
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        tracing::trace!(
            "[MOVEMENT] Chase generator initialized for {:?}, target {:?}",
            creature_guid,
            self.target
        );
        self.creature_position = current_pos;
        self.check_distance_timer = 0; // Check immediately
        self.is_moving = false;
        self.target_last_path_pos = None;
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        // Count down distance check timer
        self.check_distance_timer = self.check_distance_timer.saturating_sub(diff_ms);

        if self.check_distance_timer > 0 {
            return MovementUpdate::Continue;
        }

        // Reset timer (vmangos: m_checkDistanceTimer.Reset(100))
        self.check_distance_timer = CHECK_DISTANCE_INTERVAL;

        let dx = self.creature_position.x - self.target_position.x;
        let dy = self.creature_position.y - self.target_position.y;
        let distance = (dx * dx + dy * dy).sqrt();
        let melee_reach = self.get_melee_reach();

        if distance <= melee_reach {
            // In melee range - let spline finish naturally, don't send stop packet
            self.reached_target = true;
            if self.is_moving {
                self.is_moving = false;
                self.target_last_path_pos = None;
            }
            return MovementUpdate::Continue;
        }

        // Out of melee range — need to chase
        self.reached_target = false;

        // Only repath if target has actually moved from where it was when we last pathed
        // (vmangos: compares m_fTargetLastX/Y/Z vs i_target->GetPosition())
        if self.is_moving && !self.target_moved_enough() {
            return MovementUpdate::Continue;
        }

        // Record where the target is NOW so we can compare next time
        self.target_last_path_pos = Some(self.target_position);

        // Calculate contact point (offset from target center)
        let contact = self.get_contact_point();
        self.is_moving = true;

        MovementUpdate::NewDestination {
            destination: contact,
            speed: self.run_speed,
            is_walking: false,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Chase generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        false // Chase continues until explicitly stopped or leashed
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.target_last_path_pos = None;
        self.reached_target = false;
        self.is_moving = false;
        self.check_distance_timer = 0;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
