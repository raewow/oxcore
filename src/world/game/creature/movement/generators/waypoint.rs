//! Waypoint movement generator - follows predefined patrol paths
//!
//! Supports both per-GUID waypoints (creature_movement) and
//! per-entry waypoints (creature_movement_template).

use super::super::generator::{MovementGenerator, MovementUpdate};
use super::super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};

/// Waypoint data from database
#[derive(Debug, Clone)]
pub struct Waypoint {
    pub point_id: u32,
    pub position: Position,
    /// Milliseconds to wait at this point
    pub wait_time: u32,
    /// Script to run on arrival
    pub script_id: u32,
    /// Override orientation at this waypoint
    pub orientation: Option<f32>,
}

/// Waypoint movement - follows predefined path
pub struct WaypointMovementGenerator {
    /// Waypoints in order
    waypoints: Vec<Waypoint>,
    /// Current waypoint index
    current_index: usize,
    /// Is path repeating (loops)?
    repeating: bool,
    /// Time waiting at current waypoint (ms)
    wait_timer: u32,
    /// Currently waiting at a waypoint
    is_waiting: bool,
    /// Currently moving to a waypoint
    is_moving: bool,
    /// Walk speed in yards/sec
    walk_speed: f32,
}

impl WaypointMovementGenerator {
    pub fn new(waypoints: Vec<Waypoint>, repeating: bool, walk_speed: f32) -> Self {
        Self {
            waypoints,
            current_index: 0,
            repeating,
            wait_timer: 0,
            is_waiting: false,
            is_moving: false,
            walk_speed,
        }
    }

    fn advance_waypoint(&mut self) -> bool {
        if self.waypoints.is_empty() {
            return false;
        }

        let next = self.current_index + 1;

        if next >= self.waypoints.len() {
            if self.repeating {
                // Loop back to start
                self.current_index = 0;
                return true;
            }
            return false;
        }

        self.current_index = next;
        true
    }

    /// Called when creature arrives at waypoint
    pub fn on_arrival(&mut self) {
        self.is_moving = false;

        // Start waiting at current waypoint
        if self.current_index < self.waypoints.len() {
            let wait_time = self.waypoints[self.current_index].wait_time;
            if wait_time > 0 {
                self.wait_timer = wait_time;
                self.is_waiting = true;
            } else {
                // No wait time, advance immediately
                self.is_waiting = false;
                self.advance_waypoint();
            }
        }
    }
}

impl MovementGenerator for WaypointMovementGenerator {
    fn generator_type(&self) -> MovementGeneratorType {
        MovementGeneratorType::Waypoint
    }

    fn initialize(&mut self, creature_guid: ObjectGuid, _current_pos: Position) {
        tracing::debug!(
            "[MOVEMENT] Waypoint generator initialized for {:?} with {} waypoints",
            creature_guid,
            self.waypoints.len()
        );
    }

    fn update(&mut self, _creature_guid: ObjectGuid, diff_ms: u32) -> MovementUpdate {
        if self.waypoints.is_empty() {
            return MovementUpdate::Finished;
        }

        // Currently moving to a waypoint?
        if self.is_moving {
            return MovementUpdate::Continue;
        }

        // Waiting at waypoint?
        if self.is_waiting {
            self.wait_timer = self.wait_timer.saturating_sub(diff_ms);
            if self.wait_timer > 0 {
                return MovementUpdate::Continue;
            }

            // Done waiting, advance to next
            self.is_waiting = false;
            if !self.advance_waypoint() {
                return MovementUpdate::Finished;
            }
        }

        // Move to current waypoint
        let wp = &self.waypoints[self.current_index];
        self.is_moving = true;

        MovementUpdate::NewDestination {
            destination: wp.position,
            speed: self.walk_speed,
            is_walking: true,
        }
    }

    fn finalize(&mut self, creature_guid: ObjectGuid) {
        tracing::trace!(
            "[MOVEMENT] Waypoint generator finalized for {:?}",
            creature_guid
        );
    }

    fn is_finished(&self) -> bool {
        !self.repeating
            && self.current_index >= self.waypoints.len().saturating_sub(1)
            && !self.is_moving
            && !self.is_waiting
    }

    fn reset(&mut self, _creature_guid: ObjectGuid) {
        self.current_index = 0;
        self.wait_timer = 0;
        self.is_waiting = false;
        self.is_moving = false;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
