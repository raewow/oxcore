//! Fear movement generator - creatures flee in panic

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::Rng;

const DEFAULT_INIT_FLEE_TIME: u32 = 2000;
const DEFAULT_INIT_FLEE_DIST: f32 = 28.0;
const POST_INIT_RADIUS: f32 = 20.0;
const INIT_FLEE_ANGLE_MULT: f32 = 0.95;
const NEXT_CHECK_TIME_LOWER_BOUND: u32 = 200;
const NEXT_CHECK_TIME_UPPER_BOUND: u32 = 500;

/// Panic flee generator used for fear effects.
pub struct FearMovementGenerator {
    fright_guid: ObjectGuid,
    initial_flee_time: u32,
    initial_flee_time_remaining: u32,
    point_init_done: bool,
    force_walking: bool,
    custom_speed: f32,
    target_position: Position,
    creature_position: Position,
    next_check_time: u32,
    force_update: bool,
    flee_distance: f32,
}

impl FearMovementGenerator {
    pub fn new(fright_guid: ObjectGuid, flee_distance: f32) -> Self {
        Self {
            fright_guid,
            initial_flee_time: DEFAULT_INIT_FLEE_TIME,
            initial_flee_time_remaining: DEFAULT_INIT_FLEE_TIME,
            point_init_done: false,
            force_walking: false,
            custom_speed: 0.0,
            target_position: Position::default(),
            creature_position: Position::default(),
            next_check_time: 0,
            force_update: false,
            flee_distance: if flee_distance > 0.0 {
                flee_distance
            } else {
                DEFAULT_INIT_FLEE_DIST
            },
        }
    }

    pub fn with_timing(mut self, initial_flee_time: u32, force_walking: bool, custom_speed: f32) -> Self {
        self.initial_flee_time = initial_flee_time;
        self.initial_flee_time_remaining = initial_flee_time;
        self.force_walking = force_walking;
        self.custom_speed = custom_speed;
        self
    }

    pub fn update_target_position(&mut self, pos: Position) {
        self.target_position = pos;
    }

    pub fn set_creature_position(&mut self, pos: Position) {
        self.creature_position = pos;
    }

    pub fn on_arrival(&mut self) {
        self.force_update = true;
        self.next_check_time = rand::thread_rng().gen_range(NEXT_CHECK_TIME_LOWER_BOUND..=NEXT_CHECK_TIME_UPPER_BOUND);
    }

    fn calculate_initial_point(&self) -> Position {
        let current_pos = self.creature_position;
        let dx = current_pos.x - self.target_position.x;
        let dy = current_pos.y - self.target_position.y;
        let source_dist = (dx * dx + dy * dy).sqrt();
        let source_angle = if source_dist > 0.5 {
            dy.atan2(dx)
        } else {
            rand::thread_rng().gen_range(0.0..std::f32::consts::TAU)
        };

        let mut rng = rand::thread_rng();
        let target_dist = rng.gen_range(0.8..1.3) * self.flee_distance;
        let target_angle = source_angle + rng.gen_range(-std::f32::consts::PI * INIT_FLEE_ANGLE_MULT..std::f32::consts::PI * INIT_FLEE_ANGLE_MULT);

        Position {
            x: current_pos.x + target_dist * target_angle.cos(),
            y: current_pos.y + target_dist * target_angle.sin(),
            z: current_pos.z,
            o: target_angle,
        }
    }

    fn calculate_post_init_point(&self) -> Position {
        let current_pos = self.creature_position;
        let mut rng = rand::thread_rng();
        let target_dist = rng.gen_range(0.6..1.2) * POST_INIT_RADIUS;
        let target_angle = rng.gen_range(0.0..std::f32::consts::TAU);

        Position {
            x: current_pos.x + target_dist * target_angle.cos(),
            y: current_pos.y + target_dist * target_angle.sin(),
            z: current_pos.z,
            o: target_angle,
        }
    }
}

impl MovementGenerator for FearMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Fleeing
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.initial_flee_time_remaining = self.initial_flee_time;
        self.point_init_done = false;
        self.force_update = false;
        self.next_check_time = 0;
        self.creature_position = current_pos;
        tracing::debug!(
            "[MOVEMENT] Fear generator initialized for {:?}, fright {:?}",
            creature_guid,
            self.fright_guid
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        self.initial_flee_time_remaining = self.initial_flee_time_remaining.saturating_sub(diff_ms);
        self.next_check_time = self.next_check_time.saturating_sub(diff_ms);

        if self.next_check_time > 0 {
            return MovementUpdate::Continue;
        }

        if self.initial_flee_time_remaining > 0 {
            if !self.point_init_done {
                self.point_init_done = true;
                self.force_update = false;

                return MovementUpdate::NewDestination {
                    destination: self.calculate_initial_point(),
                    speed: if self.custom_speed > 0.0 { self.custom_speed } else { 7.0 },
                    is_walking: self.force_walking,
                };
            }

            return MovementUpdate::Continue;
        }

        if !self.point_init_done || self.force_update {
            self.point_init_done = true;
            self.force_update = false;

            return MovementUpdate::NewDestination {
                destination: self.calculate_post_init_point(),
                speed: if self.custom_speed > 0.0 { self.custom_speed } else { 7.0 },
                is_walking: self.force_walking,
            };
        }

        MovementUpdate::Continue
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Fear generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        false
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.initial_flee_time_remaining = self.initial_flee_time;
        self.point_init_done = false;
        self.force_update = false;
        self.next_check_time = 0;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Timed fear generator used for creature fear effects.
pub struct TimedFearMovementGenerator {
    inner: FearMovementGenerator,
    total_flee_time: u32,
    time_remaining: u32,
}

impl TimedFearMovementGenerator {
    pub fn new(fright_guid: ObjectGuid, time: u32, flee_distance: f32, run_speed: f32) -> Self {
        let mut rng = rand::thread_rng();
        let extra = (time as f32 * 0.25) as u32;
        let initial_flee_time = DEFAULT_INIT_FLEE_TIME + rng.gen_range(0..=extra);

        Self {
            inner: FearMovementGenerator::new(fright_guid, flee_distance)
                .with_timing(initial_flee_time, true, run_speed),
            total_flee_time: time,
            time_remaining: time,
        }
    }

    pub fn update_target_position(&mut self, pos: Position) {
        self.inner.update_target_position(pos);
    }

    pub fn set_creature_position(&mut self, pos: Position) {
        self.inner.set_creature_position(pos);
    }

    pub fn on_arrival(&mut self) {
        self.inner.on_arrival();
    }
}

impl MovementGenerator for TimedFearMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Fleeing
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.time_remaining = self.total_flee_time;
        self.inner.initialize(creature_guid, current_pos);
    }

    fn update(&mut self, creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        self.time_remaining = self.time_remaining.saturating_sub(diff_ms);
        if self.time_remaining == 0 {
            return MovementUpdate::Finished;
        }

        self.inner.update(creature_guid, diff_ms)
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        self.inner.finalize(creature_guid);
    }

    fn is_finished(&self) -> bool {
        self.time_remaining == 0
    }

    fn reset(&mut self, creature_guid: ObjectGuid) {
        self.time_remaining = self.total_flee_time;
        self.inner.reset(creature_guid);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
