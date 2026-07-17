//! Movement state - position, speeds, and movement flags

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
    /// Hover enabled (Levitate-like auras): SPELL_AURA_HOVER
    pub hover: bool,
    /// Feather fall enabled (Slow Fall, Levitate): SPELL_AURA_FEATHER_FALL
    pub feather_fall: bool,
    /// Counter assigned to controller-bound forced movement packets.
    pub movement_counter: u32,
    pub pending_knockback: Option<PendingKnockback>,
    pub pending_spline: Option<PendingSpline>,
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
        }
    }
}
