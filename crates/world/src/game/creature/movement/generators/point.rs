//! Point movement generator - moves to a single destination.

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// One-shot movement to a specific point.
pub struct PointMovementGenerator {
    id: u32,
    destination: Position,
    speed: f32,
    is_walking: bool,
    has_destination: bool,
    assistance_delay_ms: Option<u32>,
}

impl PointMovementGenerator {
    pub fn new(
        id: u32,
        destination: Position,
        speed: f32,
        is_walking: bool,
        final_orientation: f32,
    ) -> Self {
        Self {
            id,
            destination: Position {
                o: final_orientation,
                ..destination
            },
            speed,
            is_walking,
            has_destination: false,
            assistance_delay_ms: None,
        }
    }

    pub fn with_assistance(mut self, delay_ms: u32) -> Self {
        self.assistance_delay_ms = Some(delay_ms);
        self
    }

    pub fn assistance_delay_ms(&self) -> Option<u32> {
        self.assistance_delay_ms
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn on_arrival(&mut self) {
        self.has_destination = true;
    }
}

impl MovementGenerator for PointMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Point
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        tracing::debug!(
            "[MOVEMENT] Point generator initialized for {:?}, id={}",
            creature_guid,
            self.id
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, _diff_ms: u32) -> MovementUpdate {
        if self.has_destination {
            return MovementUpdate::Continue;
        }

        self.has_destination = true;
        MovementUpdate::NewDestination {
            destination: self.destination,
            speed: self.speed,
            is_walking: self.is_walking,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Point generator finalized for {:?}, id={}",
            creature_guid,
            self.id
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

/// Assistance movement shares the point destination but triggers a follow-up delay.
pub struct AssistanceMovementGenerator {
    inner: PointMovementGenerator,
}

impl AssistanceMovementGenerator {
    pub fn new(id: u32, destination: Position, speed: f32, delay_ms: u32) -> Self {
        Self {
            inner: PointMovementGenerator::new(id, destination, speed, true, -8.0)
                .with_assistance(delay_ms),
        }
    }

    pub fn assistance_delay_ms(&self) -> Option<u32> {
        self.inner.assistance_delay_ms()
    }
}

impl MovementGenerator for AssistanceMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Point
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.inner.initialize(creature_guid, current_pos);
    }

    fn update(&mut self, creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        self.inner.update(creature_guid, diff_ms)
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        self.inner.finalize(creature_guid);
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
