//! Idle movement generator - creature stands still

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_reset_is_noop_and_idle_never_finishes() {
        let creature = ObjectGuid::from_raw(1);
        let mut generator = IdleMovementGenerator::new();

        generator.initialize(creature, Position::default());
        assert_eq!(generator.generator_type(), MovementGeneratorType::Idle);
        assert!(matches!(generator.update(creature, 1_000), MovementUpdate::Continue));
        assert!(!generator.is_finished());

        generator.reset(creature);
        assert!(matches!(generator.update(creature, 1_000), MovementUpdate::Continue));
        assert!(!generator.is_finished());

        generator.finalize(creature);
        assert!(matches!(generator.update(creature, 1_000), MovementUpdate::Continue));
    }
}
