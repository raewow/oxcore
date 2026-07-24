//! Charge movement generator - rapid movement toward a target used by SPELL_EFFECT_CHARGE.
//!
//! vmangos/MaNGOS implement charge inside `Spell::OnSpellLaunch` by calling
//! `MotionMaster::MoveCharge`. The generator moves the creature straight to the
//! target's last-known position and, on arrival, optionally flags the creature to
//! begin auto-attacking that target.

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Distance at which the charge is considered to have reached the target.
const CHARGE_ARRIVAL_THRESHOLD: f32 = 0.5;

/// Rapid one-shot movement toward a unit target.
pub struct ChargeMovementGenerator {
    pub target: ObjectGuid,
    /// Whether the creature should start attacking the target on arrival.
    pub trigger_auto_attack: bool,
    /// Target's last-known position, updated each tick by the movement system.
    target_position: Position,
    /// Creature's current position, updated each tick by the movement system.
    creature_position: Position,
    /// Run speed in yards/sec (from the creature's `run_speed()`).
    run_speed: f32,
    /// Set to true once the destination has been emitted so we don't spam new
    /// destinations while the spline plays.
    has_destination: bool,
}

impl ChargeMovementGenerator {
    pub fn new(target: ObjectGuid, trigger_auto_attack: bool, run_speed: f32) -> Self {
        Self {
            target,
            trigger_auto_attack,
            target_position: Position::default(),
            creature_position: Position::default(),
            run_speed,
            has_destination: false,
        }
    }

    pub fn update_target_position(&mut self, pos: Position) {
        self.target_position = pos;
    }

    pub fn set_creature_position(&mut self, pos: Position) {
        self.creature_position = pos;
    }
}

impl MovementGenerator for ChargeMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Charge
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        tracing::debug!(
            "[MOVEMENT] Charge generator initialized for {:?} toward {:?}",
            creature_guid,
            self.target
        );
    }

    fn update(&mut self, creature_guid: ObjectGuid, _diff_ms: u32) -> MovementUpdate {
        let distance = self.creature_position.distance_to(&self.target_position);
        if distance <= CHARGE_ARRIVAL_THRESHOLD {
            tracing::debug!(
                "[MOVEMENT] Charge arrived for {:?} at distance {}",
                creature_guid,
                distance
            );
            return MovementUpdate::Arrived;
        }

        if self.has_destination {
            return MovementUpdate::Continue;
        }

        self.has_destination = true;
        MovementUpdate::NewDestination {
            destination: self.target_position,
            speed: self.run_speed,
            is_walking: false,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Charge generator finalized for {:?}",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_generator_reports_arrived_when_close() {
        let target = ObjectGuid::new_player(1);
        let mut gen = ChargeMovementGenerator::new(target, false, 7.0);
        gen.set_creature_position(Position::xyz(0.0, 0.0, 0.0));
        gen.update_target_position(Position::xyz(0.3, 0.0, 0.0));

        assert!(matches!(gen.update(target, 0), MovementUpdate::Arrived));
    }

    #[test]
    fn charge_generator_emits_destination_when_far() {
        let target = ObjectGuid::new_player(1);
        let mut gen = ChargeMovementGenerator::new(target, false, 7.0);
        gen.set_creature_position(Position::xyz(0.0, 0.0, 0.0));
        gen.update_target_position(Position::xyz(10.0, 0.0, 0.0));

        assert!(
            matches!(gen.update(target, 0), MovementUpdate::NewDestination { destination, speed, .. } if destination.x == 10.0 && speed == 7.0)
        );
    }
}
