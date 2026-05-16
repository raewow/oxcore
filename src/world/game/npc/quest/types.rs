//! Quest data structures
//!
//! Defines the core data types for quests including templates, status, and progress.
//! Adapted from the legacy world::game::quest module for world.

use bitflags::bitflags;

/// Maximum number of quests a player can have in their quest log
pub const MAX_QUEST_LOG_SIZE: usize = 20;

/// Maximum number of objectives per quest
pub const QUEST_OBJECTIVES_COUNT: usize = 4;

/// Maximum number of item objectives per quest
pub const QUEST_ITEM_OBJECTIVES_COUNT: usize = 4;

/// Maximum number of reward choices per quest
pub const QUEST_REWARD_CHOICES_COUNT: usize = 6;

/// Maximum number of fixed rewards per quest
pub const QUEST_REWARDS_COUNT: usize = 4;

/// Maximum number of reputation rewards per quest
pub const QUEST_REPUTATIONS_COUNT: usize = 5;

/// Maximum number of emotes per quest
pub const QUEST_EMOTE_COUNT: usize = 4;

/// Quest status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QuestStatus {
    #[default]
    None = 0,
    Complete = 1,
    Unavailable = 2,
    Incomplete = 3,
    Available = 4,
    Failed = 5,
}

impl From<u8> for QuestStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => QuestStatus::None,
            1 => QuestStatus::Complete,
            2 => QuestStatus::Unavailable,
            3 => QuestStatus::Incomplete,
            4 => QuestStatus::Available,
            5 => QuestStatus::Failed,
            _ => QuestStatus::None,
        }
    }
}

/// Quest giver dialog status (determines icon above NPC)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum DialogStatus {
    #[default]
    None = 0,
    Unavailable = 1, // Gray !
    Chat = 2,        // No icon
    Incomplete = 3,  // Gray ?
    RewardRep = 4,   // Yellow ? (repeatable)
    Available = 5,   // Yellow !
    RewardOld = 6,   // Not used
    Reward2 = 7,     // Yellow ? (complete)
}

impl From<u8> for DialogStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => DialogStatus::None,
            1 => DialogStatus::Unavailable,
            2 => DialogStatus::Chat,
            3 => DialogStatus::Incomplete,
            4 => DialogStatus::RewardRep,
            5 => DialogStatus::Available,
            6 => DialogStatus::RewardOld,
            7 => DialogStatus::Reward2,
            _ => DialogStatus::None,
        }
    }
}

/// Quest method (completion type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QuestMethod {
    AutoComplete = 0,
    Disabled = 1,
    #[default]
    Deliver = 2,
}

impl From<u8> for QuestMethod {
    fn from(value: u8) -> Self {
        match value {
            0 => QuestMethod::AutoComplete,
            1 => QuestMethod::Disabled,
            _ => QuestMethod::Deliver,
        }
    }
}

/// Quest type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QuestType {
    #[default]
    Normal = 0,
    Elite = 1,
    Life = 21,
    PvP = 41,
    Raid = 62,
    Dungeon = 81,
    WorldEvent = 82,
    Legendary = 83,
    Escort = 84,
}

impl From<u8> for QuestType {
    fn from(value: u8) -> Self {
        match value {
            0 => QuestType::Normal,
            1 => QuestType::Elite,
            21 => QuestType::Life,
            41 => QuestType::PvP,
            62 => QuestType::Raid,
            81 => QuestType::Dungeon,
            82 => QuestType::WorldEvent,
            83 => QuestType::Legendary,
            84 => QuestType::Escort,
            _ => QuestType::Normal,
        }
    }
}

bitflags! {
    /// Quest flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct QuestFlags: u32 {
        const NONE = 0x00000000;
        const STAY_ALIVE = 0x00000001;
        const PARTY_ACCEPT = 0x00000002;
        const EXPLORATION = 0x00000004;
        const SHARABLE = 0x00000008;
        const EPIC = 0x00000020;
        const RAID = 0x00000040;
        const HIDDEN_REWARDS = 0x00000200;
        const AUTO_REWARDED = 0x00000400;
    }
}

impl QuestFlags {
    pub fn has_flag(&self, flag: QuestFlags) -> bool {
        self.contains(flag)
    }
}

bitflags! {
    /// Quest special flags (internal server flags)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct QuestSpecialFlags: u16 {
        const NONE = 0x0000;
        const REPEATABLE = 0x0001;
        const EXPLORATION_OR_EVENT = 0x0002;
        const DELIVER = 0x0008;
        const SPEAKTO = 0x0010;
        const KILL_OR_CAST = 0x0020;
        const TIMED = 0x0040;
    }
}

impl QuestSpecialFlags {
    pub fn has_flag(&self, flag: QuestSpecialFlags) -> bool {
        self.contains(flag)
    }
}

/// Complete quest template definition
#[derive(Debug, Clone)]
pub struct QuestTemplate {
    pub id: u32,
    pub method: QuestMethod,
    pub zone_or_sort: i32,
    pub min_level: u32,
    pub max_level: u32,
    pub quest_level: u32,
    pub quest_type: QuestType,

    // Requirements
    pub required_classes: u32,
    pub required_races: u32,
    pub required_skill: u32,
    pub required_skill_value: u32,
    pub required_condition: u32,

    // Reputation requirements
    pub rep_objective_faction: u32,
    pub rep_objective_value: i32,
    pub required_min_rep_faction: u32,
    pub required_min_rep_value: i32,
    pub required_max_rep_faction: u32,
    pub required_max_rep_value: i32,

    // Quest relationships
    pub prev_quest_id: i32,
    pub next_quest_id: i32,
    pub exclusive_group: i32,
    pub breadcrumb_for_quest_id: i32,
    pub next_quest_in_chain: u32,

    // Source items
    pub src_item_id: u32,
    pub src_item_count: u32,
    pub src_spell: u32,

    // Objectives (4 slots each)
    pub req_item_id: [u32; QUEST_ITEM_OBJECTIVES_COUNT],
    pub req_item_count: [u32; QUEST_ITEM_OBJECTIVES_COUNT],
    pub req_source_id: [u32; 4],
    pub req_source_count: [u32; 4],
    pub req_creature_or_go_id: [i32; QUEST_OBJECTIVES_COUNT],
    pub req_creature_or_go_count: [u32; QUEST_OBJECTIVES_COUNT],
    pub req_spell: [u32; QUEST_OBJECTIVES_COUNT],

    // Reward choices (6 options)
    pub rew_choice_item_id: [u32; QUEST_REWARD_CHOICES_COUNT],
    pub rew_choice_item_count: [u32; QUEST_REWARD_CHOICES_COUNT],

    // Fixed rewards (4 items)
    pub rew_item_id: [u32; QUEST_REWARDS_COUNT],
    pub rew_item_count: [u32; QUEST_REWARDS_COUNT],

    // Reputation rewards (5 factions)
    pub rew_rep_faction: [u32; QUEST_REPUTATIONS_COUNT],
    pub rew_rep_value: [i32; QUEST_REPUTATIONS_COUNT],
    pub rew_rep_spillover_mask: u8,

    // Other rewards
    pub rew_xp: u32,
    pub rew_or_req_money: i32,
    pub rew_money_max_level: u32,
    pub rew_spell: u32,
    pub rew_spell_cast: u32,
    pub rew_mail_template_id: i32,
    pub rew_mail_delay_secs: u32,
    pub rew_mail_money: u32,

    // Point of interest
    pub point_map_id: u32,
    pub point_x: f32,
    pub point_y: f32,
    pub point_opt: u32,

    // Flags
    pub quest_flags: QuestFlags,
    pub special_flags: QuestSpecialFlags,
    pub suggested_players: u32,
    pub limit_time: u32,

    // Text strings
    pub title: String,
    pub details: String,
    pub objectives: String,
    pub offer_reward_text: String,
    pub request_items_text: String,
    pub end_text: String,
    pub objective_text: [String; QUEST_OBJECTIVES_COUNT],

    // Emotes
    pub details_emote: [u32; QUEST_EMOTE_COUNT],
    pub details_emote_delay: [u32; QUEST_EMOTE_COUNT],
    pub incomplete_emote: u32,
    pub complete_emote: u32,
    pub offer_reward_emote: [u32; QUEST_EMOTE_COUNT],
    pub offer_reward_emote_delay: [u32; QUEST_EMOTE_COUNT],

    // Scripts
    pub start_script: u32,
    pub complete_script: u32,
    pub is_active: bool,
}

impl QuestTemplate {
    /// Check if quest is auto-complete
    pub fn is_auto_complete(&self) -> bool {
        matches!(self.method, QuestMethod::AutoComplete)
    }

    /// Check if quest is repeatable
    pub fn is_repeatable(&self) -> bool {
        self.special_flags.contains(QuestSpecialFlags::REPEATABLE)
    }

    /// Get count of required items
    pub fn get_req_items_count(&self) -> usize {
        self.req_item_id.iter().filter(|&&id| id != 0).count()
    }

    /// Get count of required creatures/GOs
    pub fn get_req_creature_or_go_count(&self) -> usize {
        self.req_creature_or_go_id
            .iter()
            .filter(|&&id| id != 0)
            .count()
    }

    /// Get count of reward choice items
    pub fn get_rew_choice_items_count(&self) -> usize {
        self.rew_choice_item_id
            .iter()
            .filter(|&&id| id != 0)
            .count()
    }

    /// Get count of fixed reward items
    pub fn get_rew_items_count(&self) -> usize {
        self.rew_item_id.iter().filter(|&&id| id != 0).count()
    }

    /// Get count of reputation reward factions
    pub fn get_rew_rep_factions_count(&self) -> usize {
        self.rew_rep_faction.iter().filter(|&&id| id != 0).count()
    }

    /// Check if this quest has any item objectives
    pub fn has_item_objectives(&self) -> bool {
        self.req_item_id.iter().any(|&id| id != 0)
    }

    /// Check if this quest has any creature/GO objectives
    pub fn has_creature_or_go_objectives(&self) -> bool {
        self.req_creature_or_go_id.iter().any(|&id| id != 0)
    }
}

impl Default for QuestTemplate {
    fn default() -> Self {
        Self {
            id: 0,
            method: QuestMethod::Deliver,
            zone_or_sort: 0,
            min_level: 0,
            max_level: 0,
            quest_level: 0,
            quest_type: QuestType::Normal,
            required_classes: 0,
            required_races: 0,
            required_skill: 0,
            required_skill_value: 0,
            required_condition: 0,
            rep_objective_faction: 0,
            rep_objective_value: 0,
            required_min_rep_faction: 0,
            required_min_rep_value: 0,
            required_max_rep_faction: 0,
            required_max_rep_value: 0,
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            breadcrumb_for_quest_id: 0,
            next_quest_in_chain: 0,
            src_item_id: 0,
            src_item_count: 0,
            src_spell: 0,
            req_item_id: [0; QUEST_ITEM_OBJECTIVES_COUNT],
            req_item_count: [0; QUEST_ITEM_OBJECTIVES_COUNT],
            req_source_id: [0; 4],
            req_source_count: [0; 4],
            req_creature_or_go_id: [0; QUEST_OBJECTIVES_COUNT],
            req_creature_or_go_count: [0; QUEST_OBJECTIVES_COUNT],
            req_spell: [0; QUEST_OBJECTIVES_COUNT],
            rew_choice_item_id: [0; QUEST_REWARD_CHOICES_COUNT],
            rew_choice_item_count: [0; QUEST_REWARD_CHOICES_COUNT],
            rew_item_id: [0; QUEST_REWARDS_COUNT],
            rew_item_count: [0; QUEST_REWARDS_COUNT],
            rew_rep_faction: [0; QUEST_REPUTATIONS_COUNT],
            rew_rep_value: [0; QUEST_REPUTATIONS_COUNT],
            rew_rep_spillover_mask: 0,
            rew_xp: 0,
            rew_or_req_money: 0,
            rew_money_max_level: 0,
            rew_spell: 0,
            rew_spell_cast: 0,
            rew_mail_template_id: 0,
            rew_mail_delay_secs: 0,
            rew_mail_money: 0,
            point_map_id: 0,
            point_x: 0.0,
            point_y: 0.0,
            point_opt: 0,
            quest_flags: QuestFlags::NONE,
            special_flags: QuestSpecialFlags::NONE,
            suggested_players: 0,
            limit_time: 0,
            title: String::new(),
            details: String::new(),
            objectives: String::new(),
            offer_reward_text: String::new(),
            request_items_text: String::new(),
            end_text: String::new(),
            objective_text: [String::new(), String::new(), String::new(), String::new()],
            details_emote: [0; QUEST_EMOTE_COUNT],
            details_emote_delay: [0; QUEST_EMOTE_COUNT],
            incomplete_emote: 0,
            complete_emote: 0,
            offer_reward_emote: [0; QUEST_EMOTE_COUNT],
            offer_reward_emote_delay: [0; QUEST_EMOTE_COUNT],
            start_script: 0,
            complete_script: 0,
            is_active: true,
        }
    }
}

/// Quest update state for tracking database changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QuestUpdateState {
    #[default]
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Deleted = 3,
}

/// Player quest progress
#[derive(Debug, Clone)]
pub struct QuestProgress {
    pub quest_id: u32,
    pub status: QuestStatus,
    pub rewarded: bool,
    pub explored: bool,
    pub timer: u32,
    pub reward_choice: u32,
    pub creature_or_go_count: [u32; QUEST_OBJECTIVES_COUNT],
    pub item_count: [u32; QUEST_ITEM_OBJECTIVES_COUNT],
    pub update_state: QuestUpdateState,
}

impl QuestProgress {
    pub fn new(quest_id: u32) -> Self {
        Self {
            quest_id,
            status: QuestStatus::Incomplete,
            rewarded: false,
            explored: false,
            timer: 0,
            reward_choice: 0,
            creature_or_go_count: [0; QUEST_OBJECTIVES_COUNT],
            item_count: [0; QUEST_ITEM_OBJECTIVES_COUNT],
            update_state: QuestUpdateState::New,
        }
    }

    /// Check if all objectives are complete
    pub fn is_complete(&self, template: &QuestTemplate) -> bool {
        // Check creature/GO objectives
        for i in 0..QUEST_OBJECTIVES_COUNT {
            if template.req_creature_or_go_count[i] > 0 {
                if self.creature_or_go_count[i] < template.req_creature_or_go_count[i] {
                    return false;
                }
            }
        }

        // Check item objectives
        for i in 0..QUEST_ITEM_OBJECTIVES_COUNT {
            if template.req_item_count[i] > 0 {
                if self.item_count[i] < template.req_item_count[i] {
                    return false;
                }
            }
        }

        // Check exploration objective
        if template
            .special_flags
            .contains(QuestSpecialFlags::EXPLORATION_OR_EVENT)
            && !self.explored
        {
            return false;
        }

        true
    }

    /// Mark quest as changed for database sync
    pub fn mark_changed(&mut self) {
        if self.update_state == QuestUpdateState::Unchanged {
            self.update_state = QuestUpdateState::Changed;
        }
    }
}

/// Quest relation (creature/GO to quest mapping)
#[derive(Debug, Clone)]
pub struct QuestRelation {
    pub id: u32,
    pub quest: u32,
}

/// Gossip quest data for gossip menu integration
#[derive(Debug, Clone)]
pub struct GossipQuestData {
    pub quest_id: u32,
    pub icon: u32,
    pub level: u32,
    pub title: String,
}
