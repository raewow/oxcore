//! Idle movement generator - creature stands still

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};

/// Default idle generator - creature stands still
pub struct IdleMovementGenerator;

impl IdleMovementGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl MovementGenerator for IdleMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Idle
    }

    fn initialize(&mut self, _creature_guid: ObjectGuid, _current_pos: Position) {
        // Nothing to initialize
    }

    fn update(&mut self, _creature_guid: ObjectGuid, _diff_ms: u32) -> MovementUpdate {
        // Idle never finishes - it's the default state
        MovementUpdate::Continue
    }

    fn finalize(&mut self, _creature_guid: ObjectGuid) {
        // Nothing to clean up
    }

    fn is_finished(&self) -> bool {
        false // Idle never finishes
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        // Nothing to reset
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
