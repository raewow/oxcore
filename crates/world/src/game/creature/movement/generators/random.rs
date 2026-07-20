//! Random movement generator - creatures wander around spawn point
//!
//! MaNGOS-style random wander behavior:
//! - Pick random point within wander radius
//! - Walk there at walking speed
//! - Short 50ms hop between wander steps, then a 4-10 second break

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::Rng;

/// Delay before the first wander after (re)initialization.
const INITIAL_MOVE_DELAY_MS: u32 = 1000;

/// Pause between wander steps while steps remain in the current burst.
const STEP_PAUSE_MS: u32 = 50;

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
    /// Creature is rooted, stunned or distracted and may not wander.
    // ... models UNIT_STATE_CAN_NOT_MOVE | UNIT_STATE_DISTRACTED, which the generator
    // cannot read directly under the guid-only update signature.
    movement_blocked: bool,
}

impl RandomMovementGenerator {
    pub fn new(home_position: Position, wander_distance: f32, walk_speed: f32) -> Self {
        Self {
            home_position,
            wander_distance,
            destination: None,
            next_wander_time: INITIAL_MOVE_DELAY_MS,
            wander_steps: 0,
            expire_time_ms: 0,
            walk_speed,
            movement_blocked: false,
        }
    }

    pub fn with_expire_time(mut self, expire_time_ms: u32) -> Self {
        self.expire_time_ms = expire_time_ms;
        self
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

    /// Break between wander bursts: whole seconds in 4..=10, as retail uses rounded numbers.
    fn pick_pause_time() -> u32 {
        rand::thread_rng().gen_range(4..=10) * 1000
    }

    fn pick_step_count(&self) -> u8 {
        let upper = if self.wander_distance <= 1.0 { 2 } else { 8 };
        rand::thread_rng().gen_range(0..=upper)
    }

    /// Launch a wander leg and schedule the pause that follows it.
    ///
    /// The pause is scheduled at departure, matching the C++ generator: the timer only
    /// ticks down once the spline has finalized, so it is spent after arrival either way.
    fn start_random_movement(&mut self) -> Position {
        let destination = self.pick_random_destination();
        self.destination = Some(destination);

        if self.wander_steps > 0 {
            // Creature has yet to do steps before pausing
            self.wander_steps -= 1;
            self.next_wander_time = STEP_PAUSE_MS;
        } else {
            // Creature has made all its steps, time for a little break
            self.next_wander_time = Self::pick_pause_time();
            self.wander_steps = self.pick_step_count();
        }

        destination
    }

    /// Called when creature arrives at destination
    pub fn on_arrival(&mut self) {
        self.destination = None;
    }

    /// Position the creature should return to when its movement stack is rebuilt.
    ///
    /// Keeps the current position when still inside the wander radius, otherwise the
    /// wander origin.
    pub fn reset_position(&self, current_pos: Position) -> Position {
        let dx = current_pos.x - self.home_position.x;
        let dy = current_pos.y - self.home_position.y;

        if dx * dx + dy * dy <= self.wander_distance * self.wander_distance {
            current_pos
        } else {
            self.home_position
        }
    }

    /// Flag the creature as unable to wander (rooted, stunned, distracted).
    pub fn set_movement_blocked(&mut self, blocked: bool) {
        self.movement_blocked = blocked;
    }

    /// Extend the pause before the next wander, never shortening an existing one.
    pub fn add_pause_time(&mut self, pause_ms: u32) {
        if self.next_wander_time < pause_ms {
            self.next_wander_time = pause_ms;
        }
    }

    /// Shared body of initialize/reset: the C++ generator resets both through Initialize.
    // ... the aliveness guard (`if !creature.IsAlive() return`) lives with the caller,
    // which owns creature state.
    fn restart(&mut self) {
        self.destination = None;
        self.next_wander_time = INITIAL_MOVE_DELAY_MS;
    }

    /// Shared body of interrupt/finalize. Clearing UNIT_STATE_ROAMING* and restoring the
    /// walk flag is the motion master's job; here we only drop the pending leg.
    fn stop_roaming(&mut self) {
        self.destination = None;
    }
}

impl MovementGenerator for RandomMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Random
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        self.restart();
        tracing::debug!(
            "[MOVEMENT] Random generator initialized for {:?}, wander_dist={}",
            creature_guid,
            self.wander_distance
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        if self.expire_time_ms > 0 {
            if self.expire_time_ms <= diff_ms {
                return MovementUpdate::Finished;
            }
            self.expire_time_ms -= diff_ms;
        }

        // Blocked creatures expire the wander timer and drop the pending leg, so they
        // resume immediately once they can move again.
        if self.movement_blocked {
            self.next_wander_time = 0;
            self.destination = None;
            return MovementUpdate::Continue;
        }

        // Still travelling - the spline has not finalized yet.
        if self.destination.is_some() {
            return MovementUpdate::Continue;
        }

        if self.next_wander_time > 0 {
            self.next_wander_time = self.next_wander_time.saturating_sub(diff_ms);
            if self.next_wander_time > 0 {
                return MovementUpdate::Continue;
            }
        }

        // TODO: flying creatures follow a circular multi-point path in the C++ generator;
        // splines here carry a single destination, so they wander on the ground instead.
        let destination = self.start_random_movement();

        MovementUpdate::NewDestination {
            destination,
            speed: self.walk_speed,
            is_walking: true,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        self.stop_roaming();
        tracing::trace!(
            "[MOVEMENT] Random generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        false // Random movement continues indefinitely
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.restart();
    }

    fn interrupt(&mut self, _creature_guid: ObjectGuid) {
        self.stop_roaming();
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

    fn generator() -> RandomMovementGenerator {
        RandomMovementGenerator::new(pos(100.0, 100.0, 10.0), 5.0, 2.5)
    }

    #[test]
    fn first_wander_waits_the_initial_delay() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));

        assert!(matches!(
            gen.update(creature, 999),
            MovementUpdate::Continue
        ));
        assert!(matches!(
            gen.update(creature, 1),
            MovementUpdate::NewDestination { .. }
        ));
    }

    #[test]
    fn wander_destination_stays_inside_the_radius() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));

        match gen.update(creature, INITIAL_MOVE_DELAY_MS) {
            MovementUpdate::NewDestination {
                destination,
                speed,
                is_walking,
            } => {
                let dx = destination.x - 100.0;
                let dy = destination.y - 100.0;
                assert!((dx * dx + dy * dy).sqrt() <= 5.0);
                assert_eq!(destination.z, 10.0);
                assert_eq!(speed, 2.5);
                assert!(is_walking);
            }
            update => panic!("expected wander destination, got {update:?}"),
        }
    }

    #[test]
    fn travelling_creature_does_not_pick_a_new_destination() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));
        let _ = gen.update(creature, INITIAL_MOVE_DELAY_MS);

        for _ in 0..10 {
            assert!(matches!(
                gen.update(creature, 1_000),
                MovementUpdate::Continue
            ));
        }
    }

    #[test]
    fn first_leg_schedules_a_four_to_ten_second_break() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));
        let _ = gen.update(creature, INITIAL_MOVE_DELAY_MS);

        assert!((4_000..=10_000).contains(&gen.next_wander_time));
        assert_eq!(gen.next_wander_time % 1_000, 0);
        assert!(gen.wander_steps <= 8);
    }

    #[test]
    fn queued_steps_use_the_short_pause() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));
        gen.wander_steps = 3;

        let _ = gen.update(creature, INITIAL_MOVE_DELAY_MS);

        assert_eq!(gen.wander_steps, 2);
        assert_eq!(gen.next_wander_time, STEP_PAUSE_MS);
    }

    #[test]
    fn small_wander_radius_caps_the_step_burst() {
        let gen = RandomMovementGenerator::new(pos(0.0, 0.0, 0.0), 1.0, 2.5);
        for _ in 0..50 {
            assert!(gen.pick_step_count() <= 2);
        }
    }

    #[test]
    fn arrival_lets_the_scheduled_pause_run_down() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));
        let _ = gen.update(creature, INITIAL_MOVE_DELAY_MS);

        gen.on_arrival();
        let pause = gen.next_wander_time;

        assert!(matches!(
            gen.update(creature, pause - 1),
            MovementUpdate::Continue
        ));
        assert!(matches!(
            gen.update(creature, 1),
            MovementUpdate::NewDestination { .. }
        ));
    }

    #[test]
    fn expire_time_finishes_the_generator() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator().with_expire_time(2_000);
        gen.initialize(creature, pos(100.0, 100.0, 10.0));

        // The first tick still starts a wander leg - only the expiry ends the generator.
        assert!(!matches!(
            gen.update(creature, 1_000),
            MovementUpdate::Finished
        ));
        assert!(matches!(
            gen.update(creature, 1_000),
            MovementUpdate::Finished
        ));
    }

    #[test]
    fn blocked_creature_stops_wandering_and_resumes_immediately() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));
        let _ = gen.update(creature, INITIAL_MOVE_DELAY_MS);

        gen.set_movement_blocked(true);
        assert!(matches!(gen.update(creature, 100), MovementUpdate::Continue));
        assert!(gen.destination.is_none());
        assert_eq!(gen.next_wander_time, 0);

        gen.set_movement_blocked(false);
        assert!(matches!(
            gen.update(creature, 0),
            MovementUpdate::NewDestination { .. }
        ));
    }

    #[test]
    fn reset_and_interrupt_drop_the_pending_leg() {
        let creature = ObjectGuid::from_raw(1);
        let mut gen = generator();
        gen.initialize(creature, pos(100.0, 100.0, 10.0));
        let _ = gen.update(creature, INITIAL_MOVE_DELAY_MS);

        gen.interrupt(creature);
        assert!(gen.destination.is_none());

        let _ = gen.update(creature, 20_000);
        gen.reset(creature);
        assert!(gen.destination.is_none());
        assert_eq!(gen.next_wander_time, INITIAL_MOVE_DELAY_MS);
    }

    #[test]
    fn add_pause_time_only_extends_the_wait() {
        let mut gen = generator();
        gen.next_wander_time = 500;

        gen.add_pause_time(200);
        assert_eq!(gen.next_wander_time, 500);

        gen.add_pause_time(3_000);
        assert_eq!(gen.next_wander_time, 3_000);
    }

    #[test]
    fn reset_position_keeps_current_spot_inside_the_radius() {
        let gen = generator();

        assert_eq!(
            gen.reset_position(pos(103.0, 100.0, 11.0)),
            pos(103.0, 100.0, 11.0)
        );
        assert_eq!(
            gen.reset_position(pos(200.0, 100.0, 11.0)),
            pos(100.0, 100.0, 10.0)
        );
    }
}
