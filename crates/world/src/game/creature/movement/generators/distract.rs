//! Distract movement generator - short-lived motion pause

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Temporary distraction movement.
pub struct DistractMovementGenerator {
    timer_ms: u32,
}

impl DistractMovementGenerator {
    pub fn new(timer_ms: u32) -> Self {
        Self { timer_ms }
    }

    pub fn interrupt(&mut self) {}
}

impl MovementGenerator for DistractMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Distract
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        tracing::trace!(
            "[MOVEMENT] Distract generator initialized for {:?} ({} ms)",
            creature_guid,
            self.timer_ms
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        if diff_ms >= self.timer_ms {
            self.timer_ms = 0;
            MovementUpdate::Finished
        } else {
            self.timer_ms -= diff_ms;
            MovementUpdate::Continue
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Distract generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        self.timer_ms == 0
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Assistance distract currently shares the same timing behavior in Rust.
pub struct AssistanceDistractMovementGenerator {
    inner: DistractMovementGenerator,
}

impl AssistanceDistractMovementGenerator {
    pub fn new(timer_ms: u32) -> Self {
        Self {
            inner: DistractMovementGenerator::new(timer_ms),
        }
    }
}

impl MovementGenerator for AssistanceDistractMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Distract
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.inner.initialize(creature_guid, current_pos);
    }

    fn update(&mut self, creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        self.inner.update(creature_guid, diff_ms)
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Assistance distract finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    fn reset(&mut self, creature_guid: ObjectGuid) {
        self.inner.reset(creature_guid);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distract_initialize_and_reset_preserve_remaining_timer() {
        let creature = ObjectGuid::from_raw(1);
        let mut generator = DistractMovementGenerator::new(2_000);

        generator.initialize(creature, Position::default());
        assert_eq!(generator.generator_type(), MovementGeneratorType::Distract);
        assert_eq!(generator.timer_ms, 2_000);

        generator.reset(creature);
        assert_eq!(generator.timer_ms, 2_000);
        assert!(!generator.is_finished());
    }

    #[test]
    fn distract_update_counts_down_and_finishes_at_zero() {
        let creature = ObjectGuid::from_raw(1);
        let mut generator = DistractMovementGenerator::new(2_000);

        assert!(matches!(generator.update(creature, 750), MovementUpdate::Continue));
        assert_eq!(generator.timer_ms, 1_250);
        assert!(!generator.is_finished());

        assert!(matches!(generator.update(creature, 1_250), MovementUpdate::Finished));
        assert_eq!(generator.timer_ms, 0);
        assert!(generator.is_finished());
    }

    #[test]
    fn distract_interrupt_and_finalize_are_noops_for_timer() {
        let creature = ObjectGuid::from_raw(1);
        let mut generator = DistractMovementGenerator::new(2_000);

        generator.interrupt();
        assert_eq!(generator.timer_ms, 2_000);

        generator.finalize(creature);
        assert_eq!(generator.timer_ms, 2_000);
        assert!(!generator.is_finished());
    }

    #[test]
    fn assistance_distract_delegates_countdown_and_finished_state() {
        let creature = ObjectGuid::from_raw(1);
        let mut generator = AssistanceDistractMovementGenerator::new(500);

        generator.initialize(creature, Position::default());
        assert_eq!(generator.generator_type(), MovementGeneratorType::Distract);
        assert!(matches!(generator.update(creature, 499), MovementUpdate::Continue));
        assert!(!generator.is_finished());

        assert!(matches!(generator.update(creature, 1), MovementUpdate::Finished));
        assert!(generator.is_finished());

        generator.finalize(creature);
    }
}
