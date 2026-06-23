//! Follow movement generator - stays at an offset from a target.

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};

const CHECK_DISTANCE_INTERVAL: u32 = 100;

/// Follow a target while maintaining a fixed offset.
pub struct FollowMovementGenerator {
    target: ObjectGuid,
    target_position: Position,
    creature_position: Position,
    follow_distance: f32,
    follow_angle: f32,
    walk_speed: f32,
    is_moving: bool,
    check_distance_timer: u32,
    target_last_path_pos: Option<Position>,
}

impl FollowMovementGenerator {
    pub fn new(
        target: ObjectGuid,
        follow_distance: f32,
        follow_angle: f32,
        walk_speed: f32,
    ) -> Self {
        Self {
            target,
            target_position: Position::default(),
            creature_position: Position::default(),
            follow_distance,
            follow_angle,
            walk_speed,
            is_moving: false,
            check_distance_timer: 0,
            target_last_path_pos: None,
        }
    }

    pub fn update_target_position(&mut self, pos: Position) {
        self.target_position = pos;
    }

    pub fn set_creature_position(&mut self, pos: Position) {
        self.creature_position = pos;
    }

    pub fn on_arrival(&mut self) {
        self.is_moving = false;
    }

    fn follow_point(&self) -> Position {
        let angle = self.target_position.o + self.follow_angle;
        Position {
            x: self.target_position.x + angle.cos() * self.follow_distance,
            y: self.target_position.y + angle.sin() * self.follow_distance,
            z: self.target_position.z,
            o: self.target_position.o,
        }
    }

    fn target_moved_enough(&self) -> bool {
        let Some(last_pos) = self.target_last_path_pos else {
            return true;
        };

        let dx = self.target_position.x - last_pos.x;
        let dy = self.target_position.y - last_pos.y;
        dx * dx + dy * dy > 4.0
    }
}

impl MovementGenerator for FollowMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Follow
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.creature_position = current_pos;
        self.check_distance_timer = 0;
        self.is_moving = false;
        self.target_last_path_pos = None;
        tracing::debug!(
            "[MOVEMENT] Follow generator initialized for {:?}, target {:?}",
            creature_guid,
            self.target
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        self.check_distance_timer = self.check_distance_timer.saturating_sub(diff_ms);
        if self.check_distance_timer > 0 {
            return MovementUpdate::Continue;
        }

        self.check_distance_timer = CHECK_DISTANCE_INTERVAL;

        if self.is_moving && !self.target_moved_enough() {
            return MovementUpdate::Continue;
        }

        self.target_last_path_pos = Some(self.target_position);
        self.is_moving = true;

        MovementUpdate::NewDestination {
            destination: self.follow_point(),
            speed: self.walk_speed,
            is_walking: true,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Follow generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        false
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.target_last_path_pos = None;
        self.check_distance_timer = 0;
        self.is_moving = false;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
