//! Corpse DB model.
//!
//! Maps to the `corpse` table in the characters database. Schema matches
//! vmangos: minimal body record keyed by corpse GUID + owner player GUID.
//! Appearance/equipment are reconstructed at load time from the character row.

use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct CorpseRow {
    /// Corpse counter (low 32 bits of HighGuid::Corpse GUID).
    pub guid: u32,
    /// Owner player's character guid.
    pub player_guid: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub map: u32,
    /// Unix-seconds timestamp of corpse creation.
    pub time: u64,
    /// 0 = Bones, 1 = ResurrectablePve, 2 = ResurrectablePvp.
    pub corpse_type: u8,
    pub instance: u32,
}
