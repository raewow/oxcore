//! Slim GameObject object - only runtime data
//!
//! Template data lives in GameObjectManager.

use super::types::{GOState, GameObjectType, LootState};
use crate::shared::protocol::{ObjectGuid, Position};

/// Slim gameobject entity
#[derive(Debug, Clone)]
pub struct GameObject {
    // ========== Identity ==========
    /// Unique spawn GUID
    pub guid: ObjectGuid,
    /// Entry ID (links to gameobject_template)
    pub entry: u32,
    /// Spawn ID (database reference)
    pub spawn_id: u32,

    // ========== Location (static — gameobjects don't move) ==========
    /// Current position
    pub position: Position,
    /// Current map ID
    pub map_id: u32,

    // ========== Display (cached from template) ==========
    /// Display model ID
    pub display_id: u32,
    /// Scale factor
    pub scale: f32,
    /// Quaternion rotation
    pub rotation: [f32; 4],

    // ========== Type & behavior (cached from template) ==========
    /// GameObject type (Door, Chest, etc.)
    pub go_type: GameObjectType,
    /// Flags (locked, no interact, etc.)
    pub flags: u32,
    /// Faction template ID
    pub faction: u32,
    /// Level (for mining nodes, traps, etc.)
    pub level: u32,

    // ========== State machine ==========
    /// Current GO state (Active/Ready)
    pub go_state: GOState,
    /// Loot state
    pub loot_state: LootState,
    /// Art kit override
    pub art_kit: u32,
    /// Animation progress (0-255)
    pub anim_progress: u32,

    // ========== World state ==========
    /// Phase mask for visibility
    pub phase_mask: u32,
    /// Whether placed in world
    pub in_world: bool,

    // ========== Respawn ==========
    /// When gameobject should respawn (unix timestamp ms, 0 = no respawn pending)
    pub respawn_time: u64,
    /// Base respawn delay from spawn data (seconds)
    pub spawntimesecs: u32,

    // ========== Runtime ==========
    /// Player who summoned this (for campfires, fishing bobbers, etc.)
    pub created_by: ObjectGuid,
    /// Name (cached from template)
    pub name: String,
}

impl GameObject {
    /// Create a new gameobject from spawn data and template
    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        spawn_id: u32,
        position: Position,
        map_id: u32,
        template: &GameObjectTemplate,
        rotation: [f32; 4],
        state: u8,
        anim_progress: u8,
    ) -> Self {
        Self {
            guid,
            entry,
            spawn_id,
            position,
            map_id,
            display_id: template.display_id,
            scale: if template.size > 0.0 { template.size } else { 1.0 },
            rotation,
            go_type: GameObjectType::from(template.go_type),
            flags: template.flags,
            faction: template.faction,
            level: 0,
            go_state: GOState::from(state),
            loot_state: LootState::Ready,
            art_kit: 0,
            anim_progress: anim_progress as u32,
            phase_mask: 1,
            in_world: false,
            respawn_time: 0,
            spawntimesecs: 0,
            created_by: ObjectGuid::empty(),
            name: template.name.clone(),
        }
    }
}

/// Template data for gameobjects (loaded from gameobject_template)
#[derive(Debug, Clone)]
pub struct GameObjectTemplate {
    pub entry: u32,
    pub go_type: u32,
    pub display_id: u32,
    pub name: String,
    pub icon_name: String,
    pub cast_bar_caption: String,
    pub faction: u32,
    pub flags: u32,
    pub size: f32,
    pub data: [i32; 24],
}

impl GameObjectTemplate {
    /// Get the auto-close time for doors/buttons (data[1] in milliseconds)
    pub fn auto_close_time(&self) -> u32 {
        match GameObjectType::from(self.go_type) {
            GameObjectType::Door | GameObjectType::Button => {
                if self.data[1] > 0 {
                    self.data[1] as u32
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}
