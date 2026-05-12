//! Home movement generator - return to spawn position

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};

/// Return home movement - move back to spawn position
pub struct HomeMovementGenerator {
    home_position: Position,
    finished: bool,
    is_moving: bool,
    /// Run speed in yards/sec
    run_speed: f32,
}

impl HomeMovementGenerator {
    pub fn new(home_position: Position, run_speed: f32) -> Self {
        Self {
            home_position,
            finished: false,
            is_moving: false,
            run_speed,
        }
    }
}

impl MovementGenerator for HomeMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Home
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        tracing::trace!(
            "[MOVEMENT] Home generator initialized for {:?}, returning to ({:.1}, {:.1})",
            creature_guid,
            self.home_position.x,
            self.home_position.y
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, _diff_ms: u32) -> MovementUpdate {
        if self.finished {
            return MovementUpdate::Finished;
        }

        // Already moving to home, wait for arrival
        if self.is_moving {
            return MovementUpdate::Continue;
        }

        // Start moving home
        self.is_moving = true;
        MovementUpdate::NewDestination {
            destination: self.home_position,
            speed: self.run_speed,
            is_walking: false,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Home generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.finished = false;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
