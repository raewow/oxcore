//! Random movement generator - creatures wander around spawn point
//!
//! MaNGOS-style random wander behavior:
//! - Pick random point within wander radius
//! - Walk there at walking speed
//! - Pause 4-10 seconds between movements

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};
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
    /// Remaining wander steps before a longer pause
    wander_steps: u8,
    /// Optional expiry timer for temporary random movement (ms)
    expire_time_ms: u32,
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
            wander_steps: 0,
            expire_time_ms: 0,
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

    fn pick_step_pause() -> u32 {
        50
    }

    fn pick_step_count(&self) -> u8 {
        let upper = if self.wander_distance <= 1.0 { 2 } else { 8 };
        rand::thread_rng().gen_range(0..=upper)
    }

    /// Called when creature arrives at destination
    pub fn on_arrival(&mut self) {
        self.destination = None;

        if self.wander_steps > 0 {
            self.wander_steps -= 1;
            self.next_wander_time = Self::pick_step_pause();
        } else {
            self.next_wander_time = Self::pick_pause_time();
            self.wander_steps = self.pick_step_count();
        }
    }
}

impl MovementGenerator for RandomMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Random
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        self.next_wander_time = 1000;
        self.wander_steps = 0;
        tracing::debug!(
            "[MOVEMENT] Random generator initialized for {:?}, wander_dist={}",
            creature_guid,
            self.wander_distance
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        if self.expire_time_ms > 0 {
            self.expire_time_ms = self.expire_time_ms.saturating_sub(diff_ms);
            if self.expire_time_ms == 0 {
                return MovementUpdate::Finished;
            }
        }

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
        self.next_wander_time = 1000;
        self.wander_steps = 0;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
