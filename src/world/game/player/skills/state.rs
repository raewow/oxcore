use std::collections::HashMap;

/// Save state for database synchronization.
/// Tracks whether a skill entry needs to be inserted, updated, or deleted
/// on the next character save cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSaveState {
    /// Newly learned skill, not yet persisted to database
    New,
    /// Skill value changed since last save
    Changed,
    /// Skill value unchanged since last save
    Unchanged,
    /// Skill marked for deletion from database
    Deleted,
}

/// Individual skill entry for a player.
///
/// Each skill occupies one of 256 possible positions in the player
/// update fields array. The position determines which PLAYER_SKILL_INFO_*
/// update field index is used when sending UPDATE_OBJECT to the client.
#[derive(Debug, Clone)]
pub struct SkillData {
    /// Skill ID from SkillLine.dbc (e.g. 43 = Swords, 95 = Defense)
    pub skill_id: u16,
    /// Current skill value (0-300 at level 60)
    pub current_value: u16,
    /// Maximum skill value (level * 5 for weapon/defense skills)
    pub max_value: u16,
    /// Step value for tiered skills (professions use 1/2/3/4 for apprentice/journeyman/expert/artisan)
    pub step: u16,
    /// Position in the update fields array (0-255).
    /// Each position maps to three consecutive update field indices:
    /// - PLAYER_SKILL_INDEX + position*3+0: skill_id | step
    /// - PLAYER_SKILL_INDEX + position*3+1: current_value | max_value
    /// - PLAYER_SKILL_INDEX + position*3+2: bonus_value (temporary + permanent)
    pub position: usize,
    /// Database synchronization state
    pub state: SkillSaveState,
}

/// Per-player skill state, embedded in the Player struct.
///
/// Contains ALL skill data for a single player character:
/// weapon skills, armor proficiencies, defense, languages, trade skills, etc.
#[derive(Debug, Clone)]
pub struct SkillState {
    /// Map of skill_id -> SkillData for all known skills
    pub skills: HashMap<u16, SkillData>,
}

/// Maximum number of skill slots available per player.
/// Matches the client-side PLAYER_SKILL_INFO field count (256 skill slots,
/// each occupying 3 update field entries = 768 total update fields).
pub const PLAYER_MAX_SKILLS: usize = 256;

impl Default for SkillState {
    fn default() -> Self {
        Self {
            skills: HashMap::with_capacity(64), // Most characters have 30-60 skills
        }
    }
}

impl SkillData {
    /// Create a new skill entry
    pub fn new(
        skill_id: u16,
        current_value: u16,
        max_value: u16,
        step: u16,
        position: usize,
    ) -> Self {
        Self {
            skill_id,
            current_value,
            max_value,
            step,
            position,
            state: SkillSaveState::New,
        }
    }
}
