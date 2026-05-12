//! Flee movement generator - creatures flee from threats
//!
//! Used by critters and low-health creatures to run away
//! in the opposite direction from the threat source.

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};

/// Flee from a target
pub struct FleeMovementGenerator {
    /// Target to flee from
    flee_from: ObjectGuid,
    /// Flee duration (ms)
    flee_time: u32,
    /// Time remaining
    time_remaining: u32,
    /// Target's last known position
    target_position: Position,
    /// Flee distance
    flee_distance: f32,
    /// Whether we've sent the initial flee destination
    has_destination: bool,
    /// Creature's position when flee started
    start_position: Position,
    /// Run speed in yards/sec
    run_speed: f32,
}

impl FleeMovementGenerator {
    pub fn new(flee_from: ObjectGuid, flee_time: u32, run_speed: f32) -> Self {
        Self {
            flee_from,
            flee_time,
            time_remaining: flee_time,
            target_position: Position::default(),
            flee_distance: 20.0,
            has_destination: false,
            start_position: Position::default(),
            run_speed,
        }
    }

    /// Update the target position
    pub fn update_target_position(&mut self, pos: Position) {
        self.target_position = pos;
    }

    /// Calculate flee destination (opposite direction from target)
    fn calculate_flee_point(&self, current_pos: Position) -> Position {
        let dx = current_pos.x - self.target_position.x;
        let dy = current_pos.y - self.target_position.y;
        let dist = (dx * dx + dy * dy).sqrt().max(0.1);

        // Normalize and extend in opposite direction
        let flee_x = current_pos.x + (dx / dist) * self.flee_distance;
        let flee_y = current_pos.y + (dy / dist) * self.flee_distance;

        Position {
            x: flee_x,
            y: flee_y,
            z: current_pos.z,
            o: dy.atan2(dx), // Face away from target
        }
    }

    /// Get the GUID of the entity being fled from
    pub fn flee_from(&self) -> ObjectGuid {
        self.flee_from
    }
}

impl MovementGenerator for FleeMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Fleeing
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.time_remaining = self.flee_time;
        self.start_position = current_pos;
        self.has_destination = false;
        tracing::debug!(
            "[MOVEMENT] Flee generator initialized for {:?}, fleeing from {:?}",
            creature_guid,
            self.flee_from
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        self.time_remaining = self.time_remaining.saturating_sub(diff_ms);

        if self.time_remaining == 0 {
            return MovementUpdate::Finished;
        }

        // Send flee destination on first update
        if !self.has_destination {
            self.has_destination = true;
            let flee_point = self.calculate_flee_point(self.start_position);
            return MovementUpdate::NewDestination {
                destination: flee_point,
                speed: self.run_speed,
                is_walking: false,
            };
        }

        MovementUpdate::Continue
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!("[MOVEMENT] Flee generator finalized for {:?}", creature_guid);
    }

    fn is_finished(&self) -> bool {
        self.time_remaining == 0
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.time_remaining = self.flee_time;
        self.has_destination = false;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
