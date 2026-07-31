//! Movement state - position, speeds, and movement flags

use crate::game::creature::movement::packet_sender::{MovementChangeType, MovementFlagChange};
use oxcore_shared::protocol::ObjectGuid;
use oxcore_shared::protocol::Position;

/// Server-forced knockback awaiting the controller's acknowledgement.
#[derive(Debug, Clone, Copy)]
pub struct PendingKnockback {
    pub counter: u32,
    pub cos_angle: f32,
    pub sin_angle: f32,
    pub horizontal_speed: f32,
    pub vertical_speed: f32,
}

/// A scripted player spline which must be acknowledged with its exact ID.
#[derive(Debug, Clone, Copy)]
pub struct PendingSpline {
    pub id: u32,
    pub destination: Position,
}

/// What a server-forced movement change altered, used to match the client's ack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingChangeKind {
    Speed(MovementChangeType),
    Flag(MovementFlagChange),
}

/// A server-forced movement change awaiting the controller's acknowledgement.
#[derive(Debug, Clone, Copy)]
pub struct PendingMovementChange {
    pub counter: u32,
    pub kind: PendingChangeKind,
    /// Absolute speed for speed changes, unused for flag toggles.
    pub new_value: f32,
    /// Whether the flag is being applied or removed.
    pub apply: bool,
}

/// Per-player movement state
#[derive(Debug, Clone)]
pub struct MovementState {
    pub position: Position,
    pub flags: u32,
    pub timestamp: u32,
    pub fall_start_z: f32,
    pub fall_time: u32,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub swim_speed: f32,
    pub turn_rate: f32,
    /// Transport GUID if the player is on a transport
    pub transport_guid: Option<ObjectGuid>,
    /// Position relative to the current transport
    pub transport_position: Option<Position>,
    /// Transport timer from the movement packet
    pub transport_time: Option<u32>,
    /// Movement flags (from movement packets)
    pub movement_flags: u32,
    /// Last movement packet timestamp (for anti-cheat)
    pub last_movement_time: u32,
    /// Water walking enabled (ghost form, Path of Frost, etc.)
    pub water_walking: bool,
    /// Hover enabled (Levitate-like auras)
    pub hover: bool,
    /// Feather fall enabled (Slow Fall, Levitate)
    pub feather_fall: bool,
    /// Counter assigned to controller-bound forced movement packets.
    pub movement_counter: u32,
    pub pending_knockback: Option<PendingKnockback>,
    pub pending_spline: Option<PendingSpline>,
    /// Server-forced speed and flag changes the client has not acknowledged yet.
    pub pending_changes: Vec<PendingMovementChange>,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            position: Position::default(),
            flags: 0,
            timestamp: 0,
            fall_start_z: 0.0,
            fall_time: 0,
            walk_speed: 2.5,
            run_speed: 7.0,
            swim_speed: 4.7222,
            turn_rate: 3.14159,
            transport_guid: None,
            transport_position: None,
            transport_time: None,
            movement_flags: 0,
            last_movement_time: 0,
            water_walking: false,
            hover: false,
            feather_fall: false,
            movement_counter: 0,
            pending_knockback: None,
            pending_spline: None,
            pending_changes: Vec::new(),
        }
    }
}

impl MovementState {
    /// Allocate the counter for the next server-forced movement change.
    ///
    /// Counter 0 is never handed out: it is the sentinel for packets sent outside the
    /// controller queue (the logout root, for one).
    pub fn next_movement_counter(&mut self) -> u32 {
        self.movement_counter = self.movement_counter.wrapping_add(1);
        if self.movement_counter == 0 {
            self.movement_counter = 1;
        }
        self.movement_counter
    }

    /// Queue a change the client must acknowledge.
    pub fn push_pending_change(&mut self, change: PendingMovementChange) {
        self.pending_changes.push(change);
    }

    /// Whether any forced change is still awaiting an ack.
    pub fn has_pending_movement_change(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    /// Consume the queued speed change matching this ack, if any.
    ///
    /// Speeds are compared with a 0.01 tolerance.
    pub fn find_pending_speed_change(
        &mut self,
        speed_received: f32,
        counter: u32,
        change_type: MovementChangeType,
    ) -> bool {
        let found = self.pending_changes.iter().position(|change| {
            change.counter == counter
                && change.kind == PendingChangeKind::Speed(change_type)
                && (change.new_value - speed_received).abs() <= 0.01
        });

        match found {
            Some(index) => {
                self.pending_changes.remove(index);
                true
            }
            None => false,
        }
    }

    /// Consume the queued flag change matching this ack, returning whether it applied it.
    pub fn find_pending_flag_change(
        &mut self,
        counter: u32,
        flag: MovementFlagChange,
    ) -> Option<bool> {
        let index = self.pending_changes.iter().position(|change| {
            change.counter == counter && change.kind == PendingChangeKind::Flag(flag)
        })?;

        Some(self.pending_changes.remove(index).apply)
    }
}
