//! Confused movement generator - random short-range wandering.

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::Rng;

/// Confused movement around the current origin.
pub struct ConfusedMovementGenerator {
    origin: Position,
    has_destination: bool,
}

impl ConfusedMovementGenerator {
    pub fn new() -> Self {
        Self {
            origin: Position::default(),
            has_destination: false,
        }
    }

    fn pick_random_point(&self) -> Position {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let dist = rng.gen_range(0.0..4.0);

        Position {
            x: self.origin.x + angle.cos() * dist,
            y: self.origin.y + angle.sin() * dist,
            z: self.origin.z,
            o: angle,
        }
    }

    pub fn on_arrival(&mut self) {
        self.has_destination = false;
    }
}

impl MovementGenerator for ConfusedMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Confused
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.origin = current_pos;
        self.has_destination = false;
        tracing::debug!(
            "[MOVEMENT] Confused generator initialized for {:?}",
            creature_guid
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, _diff_ms: u32) -> MovementUpdate {
        if self.has_destination {
            return MovementUpdate::Continue;
        }

        self.has_destination = true;
        MovementUpdate::NewDestination {
            destination: self.pick_random_point(),
            speed: 2.5,
            is_walking: true,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Confused generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        false
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.has_destination = false;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
