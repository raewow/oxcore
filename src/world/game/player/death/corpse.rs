//! Corpse world object
//!
//! The corpse is a world object placed at the player's death location.
//! It stores the player's visual appearance so other players see a body
//! on the ground.

use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::Position;

/// Corpse types determine loot and reclaim behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpseType {
    /// Bones: non-interactive remnant after corpse expires or is reclaimed.
    Bones = 0,
    /// PvE death corpse: reclaimable, standard reclaim delay (30s).
    ResurrectablePve = 1,
    /// PvP death corpse: reclaimable, extended reclaim delay (2min).
    ResurrectablePvp = 2,
}

impl From<u32> for CorpseType {
    fn from(value: u32) -> Self {
        match value {
            1 => CorpseType::ResurrectablePve,
            2 => CorpseType::ResurrectablePvp,
            _ => CorpseType::Bones,
        }
    }
}

/// Corpse flags control visual appearance and lootability.
pub mod corpse_flags {
    pub const NONE: u32 = 0x00;
    pub const BONES: u32 = 0x01;
    pub const UNK1: u32 = 0x02;
    pub const UNK2: u32 = 0x04;
    pub const HIDE_HELM: u32 = 0x08;
    pub const HIDE_CLOAK: u32 = 0x10;
    pub const LOOTABLE: u32 = 0x20;
}

/// Value equal to the client's resurrection dialog show radius.
/// The client will show the "Resurrect Now" popup when the ghost is
/// within this distance of their corpse.
pub const CORPSE_RECLAIM_RADIUS: f32 = 39.0;

/// Corpse repop time: 6 minutes before the corpse converts to bones.
pub const CORPSE_REPOP_TIME_MS: u32 = 360_000;

/// Corpse world object placed at the player's death location.
///
/// Stores appearance data so other players see a body. The corpse
/// persists in the world until:
/// 1. The player reclaims it (CMSG_RECLAIM_CORPSE) -> converted to bones
/// 2. The death timer expires (6 minutes) -> converted to bones
/// 3. The player logs out while dead -> saved to database, restored on login
#[derive(Debug, Clone)]
pub struct Corpse {
    /// Unique corpse GUID (HighGuid::Corpse).
    pub guid: ObjectGuid,
    /// GUID of the player who died.
    pub owner_guid: ObjectGuid,
    /// World position of the corpse.
    pub position: Position,
    /// Map the corpse is on.
    pub map_id: u32,
    /// Instance ID (for dungeon/raid corpses).
    pub instance_id: u32,
    /// PvE or PvP corpse type.
    pub corpse_type: CorpseType,
    /// Unix timestamp of when the corpse was created.
    pub created_time: u64,
    /// Visual appearance data for the corpse model.
    pub skin: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_style: u8,
    pub gender: u8,
    pub race: u8,
    /// Equipment display IDs for the 19 equipment slots.
    /// Used to render equipped items on the corpse model.
    pub equipment: [u32; 19],
}

/// Create a corpse from a dead player's data.
///
/// Called during handle_player_death after the death state is set.
/// The corpse is added to the map's object list and saved to the database.
pub fn create_corpse_from_player(
    corpse_guid: ObjectGuid,
    owner_guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    is_pvp_death: bool,
    skin: u8,
    face: u8,
    hair_style: u8,
    hair_color: u8,
    facial_style: u8,
    gender: u8,
    race: u8,
    equipment_display_ids: [u32; 19],
) -> Corpse {
    Corpse {
        guid: corpse_guid,
        owner_guid,
        position,
        map_id,
        instance_id,
        corpse_type: if is_pvp_death {
            CorpseType::ResurrectablePvp
        } else {
            CorpseType::ResurrectablePve
        },
        created_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        skin,
        face,
        hair_style,
        hair_color,
        facial_style,
        gender,
        race,
        equipment: equipment_display_ids,
    }
}
