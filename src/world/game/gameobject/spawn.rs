//! GameObject spawn data from database

use crate::shared::protocol::Position;

/// Represents a gameobject spawn point from the database
#[derive(Debug, Clone)]
pub struct GameObjectSpawnData {
    /// Database spawn ID (guid in gameobject table)
    pub spawn_id: u32,
    /// Gameobject entry (links to gameobject_template)
    pub entry: u32,
    /// Map ID where gameobject spawns
    pub map_id: u32,
    /// Spawn position
    pub position: Position,
    /// Quaternion rotation components
    pub rotation0: f32,
    pub rotation1: f32,
    pub rotation2: f32,
    pub rotation3: f32,
    /// Respawn time in seconds (0 = never despawns)
    pub spawntimesecs: u32,
    /// Animation progress (0-255)
    pub animprogress: u8,
    /// Initial GOState (0=Active, 1=Ready)
    pub state: u8,
}
