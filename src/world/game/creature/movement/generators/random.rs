//! Random movement generator - creatures wander around spawn point
//!
//! MaNGOS-style random wander behavior:
//! - Pick random point within wander radius
//! - Walk there at walking speed
//! - Pause 4-10 seconds between movements

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};
use rand::Rng;

/// Random wander movement around home position
pub struct RandomMovementGenerator {
    /// Center position for wandering
    home_position: Position,
    /// Maximum wander distance
    wander_distance: f32,
    /// Current destination
    destination: Option<Position>,
    /// Time until next wander (ms)
    next_wander_time: u32,
    /// Walk speed in yards/sec
    walk_speed: f32,
}

impl RandomMovementGenerator {
    pub fn new(home_position: Position, wander_distance: f32, walk_speed: f32) -> Self {
        Self {
            home_position,
            wander_distance,
            destination: None,
            next_wander_time: 0,
            walk_speed,
        }
    }

    /// Pick a random point within wander radius
    fn pick_random_destination(&self) -> Position {
        let mut rng = rand::thread_rng();

        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let dist = rng.gen_range(0.0..self.wander_distance);

        Position {
            x: self.home_position.x + angle.cos() * dist,
            y: self.home_position.y + angle.sin() * dist,
            z: self.home_position.z, // Height adjusted by terrain/VMap later
            o: angle,
        }
    }

    /// Random pause time between wanders (500-10000ms per MaNGOS)
    fn pick_pause_time() -> u32 {
        rand::thread_rng().gen_range(500..10000)
    }

    /// Called when creature arrives at destination
    pub fn on_arrival(&mut self) {
        self.destination = None;
    }
}

impl MovementGenerator for RandomMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Random
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        self.next_wander_time = Self::pick_pause_time();
        tracing::debug!(
            "[MOVEMENT] Random generator initialized for {:?}, wander_dist={}",
            creature_guid,
            self.wander_distance
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        // Currently moving?
        if self.destination.is_some() {
            return MovementUpdate::Continue;
        }

        // Waiting to wander?
        if self.next_wander_time > 0 {
            self.next_wander_time = self.next_wander_time.saturating_sub(diff_ms);
            return MovementUpdate::Continue;
        }

        // Time to pick a new destination
        let dest = self.pick_random_destination();
        self.destination = Some(dest);
        self.next_wander_time = Self::pick_pause_time();

        MovementUpdate::NewDestination {
            destination: dest,
            speed: self.walk_speed,
            is_walking: true,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Random generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        false // Random movement continues indefinitely
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.destination = None;
        self.next_wander_time = Self::pick_pause_time();
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
