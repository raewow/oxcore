//! Movement generator trait

use super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};

/// Trait for movement generators
pub trait MovementGenerator: Send + Sync {
    /// Generator type for priority ordering
    fn generator_type(&self) -> MovementGeneratorType;

    /// Initialize the generator
    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position);

    /// Update the generator, returns true if movement continues
    fn update(&mut self, creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate;

    /// Finalize/cleanup when generator is removed
    fn finalize(&mut self, creature_guid: ObjectGuid);

    /// Check if generator is finished
    fn is_finished(&self) -> bool;

    /// Reset the generator
    fn reset(&mut self, creature_guid: ObjectGuid);

    /// Get as Any for downcasting (needed for ChaseMovementGenerator updates)
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Result of movement update
#[derive(Debug, Clone)]
pub enum MovementUpdate {
    /// Continue current movement
    Continue,
    /// Movement finished, remove generator
    Finished,
    /// New destination set
    NewDestination {
        destination: Position,
        speed: f32,
        is_walking: bool,
    },
    /// Arrived at destination
    Arrived,
}
