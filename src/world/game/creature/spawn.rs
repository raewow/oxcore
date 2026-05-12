//! Creature spawn data from database

use crate::shared::protocol::Position;

/// Spawn flags for creature behavior
pub mod spawn_flags {
    /// No special behavior
    pub const NONE: u32 = 0x00000000;
    /// Apply random respawn time variance (90-110%)
    pub const RANDOM_RESPAWN_TIME: u32 = 0x00000001;
    /// Apply dynamic respawn time based on nearby player population
    pub const DYNAMIC_RESPAWN_TIME: u32 = 0x00000002;
}

/// Represents a creature spawn point from the database
#[derive(Debug, Clone)]
pub struct CreatureSpawnData {
    /// Database spawn ID (guid in creature table)
    pub spawn_id: u32,
    /// Creature entry (links to creature_template)
    pub entry: u32,
    /// Map ID where creature spawns
    pub map_id: u32,
    /// Spawn position
    pub position: Position,
    /// Respawn time in seconds
    pub spawntimesecs: u32,
    /// Random wander distance
    pub wander_distance: f32,
    /// Movement type (0=idle, 1=random, 2=waypoint)
    pub movement_type: u8,
    /// Phase mask for visibility (bitfield)
    pub phase_mask: u32,
    /// Spawn flags controlling respawn behavior
    pub spawn_flags: u32,
}

impl CreatureSpawnData {
    /// Create spawn data with default flags
    pub fn new(
        spawn_id: u32,
        entry: u32,
        map_id: u32,
        position: Position,
        spawntimesecs: u32,
    ) -> Self {
        Self {
            spawn_id,
            entry,
            map_id,
            position,
            spawntimesecs,
            wander_distance: 0.0,
            movement_type: 0,
            phase_mask: 1,
            spawn_flags: spawn_flags::RANDOM_RESPAWN_TIME, // Default to random variance
        }
    }

    /// Create spawn data with all fields
    pub fn with_flags(
        spawn_id: u32,
        entry: u32,
        map_id: u32,
        position: Position,
        spawntimesecs: u32,
        wander_distance: f32,
        movement_type: u8,
        phase_mask: u32,
        spawn_flags: u32,
    ) -> Self {
        Self {
            spawn_id,
            entry,
            map_id,
            position,
            spawntimesecs,
            wander_distance,
            movement_type,
            phase_mask,
            spawn_flags,
        }
    }
}
