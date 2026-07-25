//! Slim GameObject object - only runtime data
//!
//! Template data lives in GameObjectManager.

use super::types::{GOState, GameObjectType, LootState};
use oxcore_shared::protocol::{ObjectGuid, Position};

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
            scale: if template.size > 0.0 {
                template.size
            } else {
                1.0
            },
            rotation,
            go_type: GameObjectType::from(template.go_type),
            flags: template.flags,
            faction: template.faction,
            level: match GameObjectType::from(template.go_type) {
                GameObjectType::Chest => template.data[9].max(0) as u32,
                GameObjectType::Trap => template.data[1].max(0) as u32,
                _ => 0,
            },
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

    /// Whether this object is explicitly marked as not blocking line of sight.
    ///
    /// Set on buttons and goobers that should be shootable/targetable through.
    /// Ported from the `losOK` fields of `GameObjectInfo`.
    pub fn is_los_ok(&self) -> bool {
        match GameObjectType::from(self.go_type) {
            GameObjectType::Button => self.data[8] != 0,
            GameObjectType::Goober => self.data[16] != 0,
            _ => false,
        }
    }

    /// Whether this object exists only server-side and is never sent to clients.
    ///
    /// Invisible triggers and spell focus markers must not block anything.
    /// Ported from `GameObjectInfo::IsServerOnly`.
    pub fn is_server_only(&self) -> bool {
        match GameObjectType::from(self.go_type) {
            GameObjectType::Generic => self.data[2] != 0,
            GameObjectType::Trap => self.data[8] != 0,
            GameObjectType::SpellFocus => self.data[3] != 0,
            GameObjectType::AuraGenerator => self.data[6] != 0,
            _ => false,
        }
    }

    /// Whether this object blocks line of sight even as an M2 doodad.
    ///
    /// Doors and generic objects are solid regardless of model kind; other M2
    /// models are skipped by LoS checks that ignore doodads.
    /// Ported from `GameObjectInfo::CanAlwaysBreakLoS`.
    pub fn can_always_break_los(&self) -> bool {
        matches!(
            GameObjectType::from(self.go_type),
            GameObjectType::Door | GameObjectType::Generic
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(go_type: GameObjectType, data: [i32; 24]) -> GameObjectTemplate {
        GameObjectTemplate {
            entry: 1,
            go_type: go_type as u32,
            display_id: 1,
            name: String::new(),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data,
        }
    }

    fn with_data(go_type: GameObjectType, index: usize, value: i32) -> GameObjectTemplate {
        let mut data = [0; 24];
        data[index] = value;
        template(go_type, data)
    }

    #[test]
    fn button_los_ok_comes_from_data_8() {
        assert!(with_data(GameObjectType::Button, 8, 1).is_los_ok());
        assert!(!with_data(GameObjectType::Button, 8, 0).is_los_ok());
        // Same index means something else on another type.
        assert!(!with_data(GameObjectType::Chest, 8, 1).is_los_ok());
    }

    #[test]
    fn goober_los_ok_comes_from_data_16() {
        assert!(with_data(GameObjectType::Goober, 16, 1).is_los_ok());
        assert!(!with_data(GameObjectType::Goober, 16, 0).is_los_ok());
    }

    #[test]
    fn server_only_flag_is_read_per_type() {
        assert!(with_data(GameObjectType::Generic, 2, 1).is_server_only());
        assert!(with_data(GameObjectType::Trap, 8, 1).is_server_only());
        assert!(with_data(GameObjectType::SpellFocus, 3, 1).is_server_only());
        assert!(with_data(GameObjectType::AuraGenerator, 6, 1).is_server_only());

        // Wrong index for the type must not be misread as server-only.
        assert!(!with_data(GameObjectType::Generic, 8, 1).is_server_only());
        assert!(!with_data(GameObjectType::Door, 2, 1).is_server_only());
    }

    #[test]
    fn doors_and_generics_always_break_los() {
        assert!(template(GameObjectType::Door, [0; 24]).can_always_break_los());
        assert!(template(GameObjectType::Generic, [0; 24]).can_always_break_los());
        assert!(!template(GameObjectType::Chest, [0; 24]).can_always_break_los());
        assert!(!template(GameObjectType::Button, [0; 24]).can_always_break_los());
    }

    #[test]
    fn auto_close_time_only_applies_to_doors_and_buttons() {
        assert_eq!(
            with_data(GameObjectType::Door, 1, 5000).auto_close_time(),
            5000
        );
        assert_eq!(
            with_data(GameObjectType::Button, 1, 5000).auto_close_time(),
            5000
        );
        assert_eq!(
            with_data(GameObjectType::Chest, 1, 5000).auto_close_time(),
            0
        );
        assert_eq!(with_data(GameObjectType::Door, 1, -1).auto_close_time(), 0);
    }
}
