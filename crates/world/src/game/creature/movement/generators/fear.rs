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

    pub fn with_timing(
        mut self,
        initial_flee_time: u32,
        force_walking: bool,
        custom_speed: f32,
    ) -> Self {
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
        self.next_check_time =
            rand::thread_rng().gen_range(NEXT_CHECK_TIME_LOWER_BOUND..=NEXT_CHECK_TIME_UPPER_BOUND);
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
        let target_angle = source_angle
            + rng.gen_range(
                -std::f32::consts::PI * INIT_FLEE_ANGLE_MULT
                    ..std::f32::consts::PI * INIT_FLEE_ANGLE_MULT,
            );

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
                    speed: if self.custom_speed > 0.0 {
                        self.custom_speed
                    } else {
                        7.0
                    },
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
                speed: if self.custom_speed > 0.0 {
                    self.custom_speed
                } else {
                    7.0
                },
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

    fn interrupt(&mut self, _creature_guid: ObjectGuid) {
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
            inner: FearMovementGenerator::new(fright_guid, flee_distance).with_timing(
                initial_flee_time,
                true,
                run_speed,
            ),
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

    fn interrupt(&mut self, creature_guid: ObjectGuid) {
        self.inner.interrupt(creature_guid);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32, z: f32) -> Position {
        Position { x, y, z, o: 0.0 }
    }

    #[test]
    fn constructor_configures_timed_walking_fear() {
        let target = ObjectGuid::from_raw(2);
        let generator = TimedFearMovementGenerator::new(target, 4_000, 30.0, 6.5);

        assert_eq!(generator.total_flee_time, 4_000);
        assert_eq!(generator.time_remaining, 4_000);
        assert_eq!(generator.inner.fright_guid, target);
        assert_eq!(generator.inner.flee_distance, 30.0);
        assert!(generator.inner.force_walking);
        assert_eq!(generator.inner.custom_speed, 6.5);
        assert!(generator.inner.initial_flee_time >= DEFAULT_INIT_FLEE_TIME);
        assert!(generator.inner.initial_flee_time <= DEFAULT_INIT_FLEE_TIME + 1_000);
    }

    #[test]
    fn initialize_resets_timed_and_inner_fear_state() {
        let creature = ObjectGuid::from_raw(1);
        let target = ObjectGuid::from_raw(2);
        let mut generator = TimedFearMovementGenerator::new(target, 4_000, 30.0, 6.5);

        let _ = generator.update(creature, 1_000);
        generator.on_arrival();
        generator.initialize(creature, pos(10.0, 20.0, 3.0));

        assert_eq!(generator.time_remaining, 4_000);
        assert_eq!(
            generator.inner.initial_flee_time_remaining,
            generator.inner.initial_flee_time
        );
        assert_eq!(generator.inner.creature_position, pos(10.0, 20.0, 3.0));
        assert!(!generator.inner.point_init_done);
        assert!(!generator.inner.force_update);
        assert_eq!(generator.inner.next_check_time, 0);
    }

    #[test]
    fn update_delegates_initial_panic_destination_before_timeout() {
        let creature = ObjectGuid::from_raw(1);
        let target = ObjectGuid::from_raw(2);
        let mut generator = TimedFearMovementGenerator::new(target, 4_000, 30.0, 6.5);
        generator.update_target_position(pos(0.0, 0.0, 0.0));
        generator.initialize(creature, pos(10.0, 0.0, 1.0));

        match generator.update(creature, 100) {
            MovementUpdate::NewDestination {
                destination,
                speed,
                is_walking,
            } => {
                let dx = destination.x - 10.0;
                let dy = destination.y;
                let distance = (dx * dx + dy * dy).sqrt();
                assert!((24.0..=39.0).contains(&distance));
                assert_eq!(destination.z, 1.0);
                assert_eq!(speed, 6.5);
                assert!(is_walking);
            }
            update => panic!("expected initial fear destination, got {update:?}"),
        }

        assert!(matches!(
            generator.update(creature, 100),
            MovementUpdate::Continue
        ));
    }

    #[test]
    fn arrival_requests_post_initial_destination_after_initial_flee() {
        let creature = ObjectGuid::from_raw(1);
        let target = ObjectGuid::from_raw(2);
        let mut generator = TimedFearMovementGenerator::new(target, 5_000, 30.0, 6.5);
        generator.initialize(creature, pos(10.0, 0.0, 1.0));
        generator.inner.initial_flee_time_remaining = 0;
        generator.inner.next_check_time = 0;
        generator.inner.point_init_done = true;
        generator.set_creature_position(pos(30.0, 0.0, 1.0));

        generator.on_arrival();
        generator.inner.next_check_time = 0;

        match generator.update(creature, 100) {
            MovementUpdate::NewDestination { destination, .. } => {
                let dx = destination.x - 30.0;
                let dy = destination.y;
                let distance = (dx * dx + dy * dy).sqrt();
                assert!((12.0..=24.0).contains(&distance));
                assert_eq!(destination.z, 1.0);
            }
            update => panic!("expected post-initial fear destination, got {update:?}"),
        }
    }

    #[test]
    fn update_finishes_when_total_duration_expires() {
        let creature = ObjectGuid::from_raw(1);
        let target = ObjectGuid::from_raw(2);
        let mut generator = TimedFearMovementGenerator::new(target, 1_000, 30.0, 6.5);
        generator.initialize(creature, pos(10.0, 0.0, 1.0));

        assert!(matches!(
            generator.update(creature, 1_000),
            MovementUpdate::Finished
        ));
        assert!(generator.is_finished());
    }
}
