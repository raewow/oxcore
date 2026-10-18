use crate::file_loader::DbcRecord;
use crate::store::DbcEntry;
use anyhow::{Context, Result};
/// AreaTable DBC entry
/// Format: "niiixxxxxxxxxxxxxxxxxxxiiixx" (25 fields)
#[derive(Debug, Clone)]
pub struct AreaTableEntry {
    pub id: u32,
    pub map_id: u32,
    pub zone: u32,
    pub explore_flag: u32,
    pub flags: u32,
    pub area_level: i32,
    pub team: u32,
    pub liquid_type_override: u32,
}

impl DbcEntry for AreaTableEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read AreaTable ID")?;

        // Skip if ID is 0 (invalid entry)
        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            map_id: record
                .get_u32(1)
                .context("Failed to read AreaTable map_id")?,
            zone: record.get_u32(2).context("Failed to read AreaTable zone")?,
            explore_flag: record
                .get_u32(3)
                .context("Failed to read AreaTable explore_flag")?,
            flags: record
                .get_u32(4)
                .context("Failed to read AreaTable flags")?,
            area_level: record
                .get_u32(10)
                .context("Failed to read AreaTable area_level")? as i32,
            team: record
                .get_u32(20)
                .context("Failed to read AreaTable team")?,
            liquid_type_override: record
                .get_u32(24)
                .context("Failed to read AreaTable liquid_type_override")?,
        };

        Ok(Some((id, entry)))
    }
}

/// Map DBC entry.
///
/// Vanilla `Map.dbc` columns: 0 id, 1 internal name (string offset), 2 instance
/// type, 3 PvP flag, then localized names. The file carries no player limit,
/// reset delay, parent map or ghost entrance; those come from the world
/// database's `map_template` and are merged in with
/// [`DbcManager::apply_map_template`](crate::DbcManager::apply_map_template).
#[derive(Debug, Clone, Default)]
pub struct MapEntry {
    pub id: u32,
    pub map_type: u32, // 0=Common, 1=Instance, 2=Raid, 3=Battleground
    /// Player cap for one copy; 0 means unlimited.
    pub max_players: u32,
    /// Raid reset period in days; 0 for maps without a global reset.
    pub reset_delay: u32,
    /// Map whose entrance encloses this one (deep dungeons), or 0.
    pub parent: u32,
    /// Where a ghost lands when sent to this dungeon's entrance (-1: none).
    pub ghost_entrance_map: i32,
    pub ghost_entrance_x: f32,
    pub ghost_entrance_y: f32,
    /// English map name, used in player-facing refusals.
    pub name: String,
}

impl MapEntry {
    pub const COMMON: u32 = 0;
    pub const INSTANCE: u32 = 1;
    pub const RAID: u32 = 2;
    pub const BATTLEGROUND: u32 = 3;

    /// Dungeons and raids: maps that are split into bound copies.
    pub fn is_dungeon(&self) -> bool {
        self.map_type == Self::INSTANCE || self.map_type == Self::RAID
    }

    pub fn is_raid(&self) -> bool {
        self.map_type == Self::RAID
    }

    pub fn is_non_raid_dungeon(&self) -> bool {
        self.map_type == Self::INSTANCE
    }

    pub fn is_battleground(&self) -> bool {
        self.map_type == Self::BATTLEGROUND
    }

    /// Mounts work outdoors and in the few open-air dungeons: Zul'Farrak,
    /// Caverns of Time, Zul'Gurub and the Ruins of Ahn'Qiraj.
    pub fn is_mount_allowed(&self) -> bool {
        !self.is_dungeon() || matches!(self.id, 209 | 269 | 309 | 509)
    }
}

impl DbcEntry for MapEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Map ID")?;

        // Note: Map ID 0 is Eastern Kingdoms - it's valid! Don't skip it.
        let entry = Self {
            id,
            map_type: record.get_u32(2).context("Failed to read Map map_type")?,
            ghost_entrance_map: -1,
            ..Self::default()
        };

        Ok(Some((id, entry)))
    }
}

/// AreaTrigger DBC entry
#[derive(Debug, Clone)]
pub struct AreaTriggerEntry {
    pub id: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub box_x: f32,
    pub box_y: f32,
    pub box_z: f32,
    pub box_orientation: f32,
}

impl DbcEntry for AreaTriggerEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read AreaTrigger ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            map_id: record
                .get_u32(1)
                .context("Failed to read AreaTrigger map_id")?,
            x: record.get_f32(2).context("Failed to read AreaTrigger x")?,
            y: record.get_f32(3).context("Failed to read AreaTrigger y")?,
            z: record.get_f32(4).context("Failed to read AreaTrigger z")?,
            radius: record
                .get_f32(5)
                .context("Failed to read AreaTrigger radius")?,
            box_x: record
                .get_f32(6)
                .context("Failed to read AreaTrigger box_x")?,
            box_y: record
                .get_f32(7)
                .context("Failed to read AreaTrigger box_y")?,
            box_z: record
                .get_f32(8)
                .context("Failed to read AreaTrigger box_z")?,
            box_orientation: record
                .get_f32(9)
                .context("Failed to read AreaTrigger box_orientation")?,
        };

        Ok(Some((id, entry)))
    }
}

/// WorldSafeLocs DBC entry
/// Format: "nifffxxxxxxxx" (ID, MapID, X, Y, Z, Name[8])
/// In vanilla 1.12.1, WorldSafeLocs.dbc has:
/// - Field 0: ID (uint32) - safe location ID
/// - Field 1: MapID (uint32) - map ID
/// - Field 2: X (float) - X coordinate
/// - Field 3: Y (float) - Y coordinate
/// - Field 4: Z (float) - Z coordinate
/// - Fields 5-12: Name (8 locale strings - offsets)
///
/// Used for graveyard locations and other safe teleport destinations.
#[derive(Debug, Clone)]
pub struct WorldSafeLocsEntry {
    pub id: u32,
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl DbcEntry for WorldSafeLocsEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read WorldSafeLocs ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            map_id: record
                .get_u32(1)
                .context("Failed to read WorldSafeLocs map_id")?,
            x: record
                .get_f32(2)
                .context("Failed to read WorldSafeLocs x")?,
            y: record
                .get_f32(3)
                .context("Failed to read WorldSafeLocs y")?,
            z: record
                .get_f32(4)
                .context("Failed to read WorldSafeLocs z")?,
        };

        Ok(Some((id, entry)))
    }
}
