//! Quest system message structs
//!
//! This module contains type-safe message structures for all quest-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgQuestlogFull`] - Quest log is full, cannot accept more quests
//! - [`SmsgQuestupdateComplete`] - Quest objective completed
//! - [`SmsgQuestupdateFailed`] - Quest failed (e.g., timed quest expired)
//! - [`SmsgQuestupdateFailedtimer`] - Quest timer expired
//! - [`SmsgQuestgiverQuestInvalid`] - Quest is invalid for this player
//! - [`SmsgQuestgiverQuestComplete`] - Quest reward received
//! - [`SmsgQuestupdateAddItem`] - Item objective progress update
//! - [`SmsgQuestupdateAddQuest`] - Quest added to quest log
//! - [`SmsgQuestgiverStatus`] - Quest status indicator above NPC head
//! - [`SmsgQuestupdateAddKill`] - Kill objective progress update
//! - [`SmsgQuestgiverQuestList`] - List of quests from quest giver
//! - [`SmsgQuestgiverRequestItems`] - Request items for quest completion
//! - [`SmsgQuestgiverOfferReward`] - Show quest rewards
//! - [`SmsgQuestgiverQuestDetails`] - Show quest details
//! - [`SmsgQuestQueryResponse`] - Quest information response

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

/// Quest dialog status (determines icon above NPC)
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

/// Quest flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestFlags(pub u32);

impl QuestFlags {
    pub const NONE: u32 = 0x00000000;
    pub const STAY_ALIVE: u32 = 0x00000001;
    pub const PARTY_ACCEPT: u32 = 0x00000002;
    pub const EXPLORATION: u32 = 0x00000004;
    pub const SHARABLE: u32 = 0x00000008;
    pub const EPIC: u32 = 0x00000020;
    pub const RAID: u32 = 0x00000040;
    pub const HIDDEN_REWARDS: u32 = 0x00000200;
    pub const AUTO_REWARDED: u32 = 0x00000400;

    pub fn has_flag(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
}

/// Maximum number of objectives per quest
pub const QUEST_OBJECTIVES_COUNT: usize = 4;

/// Maximum number of item objectives per quest
pub const QUEST_ITEM_OBJECTIVES_COUNT: usize = 4;

/// Maximum number of reward choices per quest
pub const QUEST_REWARD_CHOICES_COUNT: usize = 6;

/// Maximum number of fixed rewards per quest
pub const QUEST_REWARDS_COUNT: usize = 4;

/// Maximum number of emotes per quest
pub const QUEST_EMOTE_COUNT: usize = 4;

// ============================================================================
// Simple Messages (no complex dependencies)
// ============================================================================

/// SMSG_QUESTLOG_FULL - Quest log is full, cannot accept more quests
///
/// Sent when player tries to accept a quest but their quest log is full.
#[derive(Debug, Clone)]
pub struct SmsgQuestlogFull;

impl ToWorldPacket for SmsgQuestlogFull {
    fn to_world_packet(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_QUESTLOG_FULL)
    }
}

/// SMSG_QUESTUPDATE_COMPLETE - Quest objective completed
///
/// Sent when a quest's objectives are complete and it's ready to turn in.
#[derive(Debug, Clone)]
pub struct SmsgQuestupdateComplete {
    /// Quest ID
    pub quest_id: u32,
}

impl ToWorldPacket for SmsgQuestupdateComplete {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_COMPLETE);
        packet.write_u32(self.quest_id);
        packet
    }
}

/// SMSG_QUESTUPDATE_FAILED - Quest failed
///
/// Sent when a quest fails (e.g., timed quest expires).
#[derive(Debug, Clone)]
pub struct SmsgQuestupdateFailed {
    /// Quest ID
    pub quest_id: u32,
}

impl ToWorldPacket for SmsgQuestupdateFailed {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_FAILED);
        packet.write_u32(self.quest_id);
        packet
    }
}

/// SMSG_QUESTUPDATE_FAILEDTIMER - Quest timer expired
///
/// Sent when a timed quest's timer expires.
#[derive(Debug, Clone)]
pub struct SmsgQuestupdateFailedtimer {
    /// Quest ID
    pub quest_id: u32,
}

impl ToWorldPacket for SmsgQuestupdateFailedtimer {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_FAILEDTIMER);
        packet.write_u32(self.quest_id);
        packet
    }
}

/// SMSG_QUESTGIVER_QUEST_INVALID - Quest is invalid
///
/// Sent when a quest cannot be accepted for some reason.
#[derive(Debug, Clone)]
pub struct SmsgQuestgiverQuestInvalid {
    /// Reason code
    pub reason: u32,
}

impl ToWorldPacket for SmsgQuestgiverQuestInvalid {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_QUEST_INVALID);
        packet.write_u32(self.reason);
        packet
    }
}

/// SMSG_QUESTGIVER_QUEST_COMPLETE - Quest reward received
///
/// Sent when player receives quest rewards.
#[derive(Debug, Clone)]
pub struct SmsgQuestgiverQuestComplete {
    /// Quest ID
    pub quest_id: u32,
    /// XP reward amount
    pub xp: u32,
}

impl ToWorldPacket for SmsgQuestgiverQuestComplete {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_QUEST_COMPLETE);
        packet.write_u32(self.quest_id);
        packet.write_u32(0x03); // Unknown flag
        packet.write_u32(self.xp);
        packet
    }
}

/// SMSG_QUESTUPDATE_ADD_ITEM - Item objective progress update
///
/// Sent when player gains an item for a quest objective.
#[derive(Debug, Clone)]
pub struct SmsgQuestupdateAddItem {
    /// Item template ID
    pub item_id: u32,
    /// Number of items collected
    pub count: u32,
}

impl ToWorldPacket for SmsgQuestupdateAddItem {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_ADD_ITEM);
        packet.write_u32(self.item_id);
        packet.write_u32(self.count);
        packet
    }
}

/// SMSG_QUESTGIVER_STATUS - Quest status indicator
///
/// Sent to show the quest marker above NPC heads (yellow !, gray ?, etc.).
#[derive(Debug, Clone)]
pub struct SmsgQuestgiverStatus {
    /// GUID of the quest giver
    pub guid: ObjectGuid,
    /// Quest dialog status
    pub status: DialogStatus,
}

impl ToWorldPacket for SmsgQuestgiverStatus {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_STATUS);
        packet.write_guid_raw(self.guid.raw());
        packet.write_u32(self.status as u32);
        packet
    }
}

/// SMSG_QUESTUPDATE_ADD_KILL - Kill objective progress update
///
/// Sent when player makes progress on a creature kill objective.
#[derive(Debug, Clone)]
pub struct SmsgQuestupdateAddKill {
    /// Quest ID
    pub quest_id: u32,
    /// Creature entry ID
    pub entry: u32,
    /// Current kill count
    pub count: u32,
    /// Required kill count
    pub required_count: u32,
    /// GUID of the killed creature
    pub guid: ObjectGuid,
}

impl ToWorldPacket for SmsgQuestupdateAddKill {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_ADD_KILL);
        packet.write_u32(self.quest_id);
        packet.write_u32(self.entry);
        packet.write_u32(self.count);
        packet.write_u32(self.required_count);
        packet.write_guid_raw(self.guid.raw());
        packet
    }
}

// ============================================================================
// V2 Packet Structs (world compatible, no ObjectMgr dependency)
// ============================================================================

/// Quest data for quest list
#[derive(Debug, Clone)]
pub struct QuestListItem {
    pub quest_id: u32,
    pub icon: u32,
    pub level: u32,
    pub title: String,
}

/// SMSG_QUESTGIVER_QUEST_LIST - List of quests from quest giver (V2)
///
/// Sent when player interacts with a quest giver NPC.
/// V2 version: takes pre-resolved quest data instead of ObjectMgr.
pub struct SmsgQuestgiverQuestListV2<'a> {
    /// GUID of the quest giver
    pub guid: ObjectGuid,
    /// Greeting text/title
    pub title: &'a str,
    /// Emote delay in milliseconds
    pub emote_delay: u32,
    /// Emote ID to play
    pub emote: u32,
    /// List of quests with titles pre-resolved
    pub quests: &'a [QuestListItem],
}

impl ToWorldPacket for SmsgQuestgiverQuestListV2<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_QUEST_LIST);
        packet.write_guid_raw(self.guid.raw());
        packet.write_string(self.title);
        packet.write_u32(self.emote_delay);
        packet.write_u32(self.emote);
        packet.write_u8(self.quests.len() as u8);

        for quest in self.quests {
            packet.write_u32(quest.quest_id);
            packet.write_u32(quest.icon);
            packet.write_u32(quest.level);
            packet.write_string(&quest.title);
        }

        packet
    }
}

/// Required item info for quest request items packet
#[derive(Debug, Clone, Default)]
pub struct RequestItemInfo {
    pub item_id: u32,
    pub count: u32,
    pub display_id: u32,
}

/// SMSG_QUESTGIVER_REQUEST_ITEMS - Request items for quest completion (V2)
///
/// Sent to show the quest turn-in dialog with required items.
/// V2 version: takes pre-resolved item display IDs instead of ObjectMgr.
pub struct SmsgQuestgiverRequestItemsV2<'a> {
    /// GUID of the quest giver
    pub guid: ObjectGuid,
    /// Quest ID
    pub quest_id: u32,
    /// Quest title
    pub title: &'a str,
    /// Request items text
    pub request_items_text: &'a str,
    /// Complete emote
    pub complete_emote: u32,
    /// Incomplete emote
    pub incomplete_emote: u32,
    /// Whether the quest is completable
    pub completable: bool,
    /// Whether to close window on cancel
    pub close_on_cancel: bool,
    /// Required money (only if negative, otherwise 0)
    pub req_money: u32,
    /// Required items with display IDs pre-resolved
    pub req_items: &'a [RequestItemInfo],
}

impl ToWorldPacket for SmsgQuestgiverRequestItemsV2<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_REQUEST_ITEMS);

        packet.write_guid_raw(self.guid.raw());
        packet.write_u32(self.quest_id);
        packet.write_cstring(self.title);
        packet.write_cstring(self.request_items_text);

        // Emote delay (always 0x00)
        packet.write_u32(0x00);

        // Emote ID (complete or incomplete based on completable)
        let emote_id = if self.completable {
            self.complete_emote
        } else {
            self.incomplete_emote
        };
        packet.write_u32(emote_id);

        // Close Window after cancel
        packet.write_u32(if self.close_on_cancel { 0x01 } else { 0x00 });

        // Required Money
        packet.write_u32(self.req_money);

        // Required items count
        packet.write_u32(self.req_items.len() as u32);

        // Required items
        for item in self.req_items {
            packet.write_u32(item.item_id);
            packet.write_u32(item.count);
            packet.write_u32(item.display_id);
        }

        // Flags (matching core's structure)
        packet.write_u32(0x02); // flags1

        if !self.completable {
            packet.write_u32(0x00); // flags2
        } else {
            packet.write_u32(0x03); // flags2
        }

        packet.write_u32(0x04); // flags3
        packet.write_u32(0x08); // flags4 (vanilla 1.12.1)

        packet
    }
}

/// Reward item info
#[derive(Debug, Clone, Default)]
pub struct RewardItemInfo {
    pub item_id: u32,
    pub count: u32,
    pub display_id: u32,
}

/// SMSG_QUESTGIVER_OFFER_REWARD - Show quest rewards (V2)
///
/// Sent to show the quest reward selection dialog.
/// V2 version: takes pre-resolved item display IDs instead of ObjectMgr.
pub struct SmsgQuestgiverOfferRewardV2<'a> {
    /// GUID of the quest giver
    pub guid: ObjectGuid,
    /// Quest ID
    pub quest_id: u32,
    /// Quest title
    pub title: &'a str,
    /// Offer reward text
    pub offer_reward_text: &'a str,
    /// Whether to enable auto-finish
    pub enable_next: bool,
    /// Reward choice items with display IDs pre-resolved
    pub reward_choices: &'a [RewardItemInfo],
    /// Fixed reward items with display IDs pre-resolved
    pub reward_items: &'a [RewardItemInfo],
    /// Money reward
    pub money_reward: u32,
    /// Quest flags
    pub quest_flags: QuestFlags,
    /// Reward spell
    pub rew_spell: u32,
    /// Offer reward emotes
    pub offer_reward_emote: [u32; QUEST_EMOTE_COUNT],
    /// Offer reward emote delays
    pub offer_reward_emote_delay: [u32; QUEST_EMOTE_COUNT],
}

impl ToWorldPacket for SmsgQuestgiverOfferRewardV2<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_OFFER_REWARD);

        packet.write_guid_raw(self.guid.raw());
        packet.write_u32(self.quest_id);
        packet.write_string(self.title);
        packet.write_string(self.offer_reward_text);
        packet.write_u32(if self.enable_next { 1 } else { 0 }); // Auto finish

        // Emotes
        let emote_count = self.offer_reward_emote.iter().filter(|&&e| e != 0).count();
        packet.write_u32(emote_count as u32);
        for i in 0..emote_count {
            packet.write_u32(self.offer_reward_emote_delay[i]);
            packet.write_u32(self.offer_reward_emote[i]);
        }

        // Reward choice items
        packet.write_u32(self.reward_choices.len() as u32);
        for item in self.reward_choices {
            packet.write_u32(item.item_id);
            packet.write_u32(item.count);
            packet.write_u32(item.display_id);
        }

        // Fixed reward items
        packet.write_u32(self.reward_items.len() as u32);
        for item in self.reward_items {
            packet.write_u32(item.item_id);
            packet.write_u32(item.count);
            packet.write_u32(item.display_id);
        }

        // Money reward
        packet.write_u32(self.money_reward);
        packet.write_u32(self.quest_flags.0);
        packet.write_u32(self.rew_spell);

        packet
    }
}

/// SMSG_QUESTGIVER_QUEST_DETAILS - Show quest details (V2)
///
/// Sent to show the quest accept dialog with full quest details.
/// V2 version: takes pre-resolved item display IDs instead of ObjectMgr.
pub struct SmsgQuestgiverQuestDetailsV2<'a> {
    /// GUID of the quest giver
    pub guid: ObjectGuid,
    /// Quest ID
    pub quest_id: u32,
    /// Quest title
    pub title: &'a str,
    /// Quest details text
    pub details: &'a str,
    /// Quest objectives text
    pub objectives: &'a str,
    /// Whether to activate auto-accept
    pub activate_accept: bool,
    /// Quest flags
    pub quest_flags: QuestFlags,
    /// Reward choice items with display IDs pre-resolved
    pub reward_choices: &'a [RewardItemInfo],
    /// Fixed reward items with display IDs pre-resolved
    pub reward_items: &'a [RewardItemInfo],
    /// Money reward
    pub money_reward: u32,
    /// Reward spell
    pub rew_spell: u32,
    /// Details emotes
    pub details_emote: [u32; QUEST_EMOTE_COUNT],
    /// Details emote delays
    pub details_emote_delay: [u32; QUEST_EMOTE_COUNT],
}

impl ToWorldPacket for SmsgQuestgiverQuestDetailsV2<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_QUEST_DETAILS);

        packet.write_guid_raw(self.guid.raw());
        packet.write_u32(self.quest_id);
        packet.write_cstring(self.title);
        packet.write_cstring(self.details);
        packet.write_cstring(self.objectives);
        packet.write_u32(if self.activate_accept { 1 } else { 0 }); // Auto finish

        // Handle hidden rewards
        if self.quest_flags.has_flag(QuestFlags::HIDDEN_REWARDS) {
            // Rewarded chosen items hidden
            packet.write_u32(0);
            // Rewarded items hidden
            packet.write_u32(0);
            // Rewarded money hidden
            packet.write_u32(0);
        } else {
            // Reward choice items
            packet.write_u32(self.reward_choices.len() as u32);
            for item in self.reward_choices {
                packet.write_u32(item.item_id);
                packet.write_u32(item.count);
                packet.write_u32(item.display_id);
            }

            // Fixed reward items
            packet.write_u32(self.reward_items.len() as u32);
            for item in self.reward_items {
                packet.write_u32(item.item_id);
                packet.write_u32(item.count);
                packet.write_u32(item.display_id);
            }

            // Money reward
            packet.write_u32(self.money_reward);
        }

        packet.write_u32(self.rew_spell);
        // Note: rew_spell_cast is NOT in SMSG_QUESTGIVER_QUEST_DETAILS for 1.12.1
        // It's only in SMSG_QUESTGIVER_OFFER_REWARD

        // Emotes (always write QUEST_EMOTE_COUNT, matching core)
        packet.write_u32(QUEST_EMOTE_COUNT as u32);
        for i in 0..QUEST_EMOTE_COUNT {
            packet.write_u32(self.details_emote[i]);
            packet.write_u32(self.details_emote_delay[i]); // delay between emotes in ms
        }

        packet
    }
}

/// Objective data for quest query response
#[derive(Debug, Clone, Default)]
pub struct QuestObjectiveData {
    pub creature_or_go_id: i32,
    pub creature_or_go_count: u32,
    pub item_id: u32,
    pub item_count: u32,
}

/// SMSG_QUEST_QUERY_RESPONSE - Quest information response (V2)
///
/// Sent in response to a quest query request from the client.
/// V2 version: takes all quest data directly instead of QuestTemplate reference.
pub struct SmsgQuestQueryResponseV2<'a> {
    /// Quest ID
    pub quest_id: u32,
    /// Quest method (0=auto, 1=disabled, 2=deliver)
    pub method: u32,
    /// Quest level
    pub quest_level: u32,
    /// Zone or sort ID
    pub zone_or_sort: i32,
    /// Quest type
    pub quest_type: u32,
    /// Reputation objective faction
    pub rep_objective_faction: u32,
    /// Reputation objective value
    pub rep_objective_value: i32,
    /// Next quest in chain
    pub next_quest_in_chain: u32,
    /// Money reward
    pub rew_or_req_money: i32,
    /// Money reward at max level
    pub rew_money_max_level: u32,
    /// Reward spell
    pub rew_spell: u32,
    /// Source item ID
    pub src_item_id: u32,
    /// Quest flags
    pub quest_flags: QuestFlags,
    /// Fixed reward items
    pub rew_item_id: [u32; QUEST_REWARDS_COUNT],
    /// Fixed reward item counts
    pub rew_item_count: [u32; QUEST_REWARDS_COUNT],
    /// Reward choice items
    pub rew_choice_item_id: [u32; QUEST_REWARD_CHOICES_COUNT],
    /// Reward choice item counts
    pub rew_choice_item_count: [u32; QUEST_REWARD_CHOICES_COUNT],
    /// Point of interest map ID
    pub point_map_id: u32,
    /// Point of interest X
    pub point_x: f32,
    /// Point of interest Y
    pub point_y: f32,
    /// Point of interest option
    pub point_opt: u32,
    /// Quest title
    pub title: &'a str,
    /// Quest objectives summary
    pub objectives: &'a str,
    /// Quest details
    pub details: &'a str,
    /// End text
    pub end_text: &'a str,
    /// Objectives data
    pub objectives_data: [QuestObjectiveData; QUEST_OBJECTIVES_COUNT],
    /// Objective texts
    pub objective_text: &'a [String; QUEST_OBJECTIVES_COUNT],
}

impl ToWorldPacket for SmsgQuestQueryResponseV2<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUEST_QUERY_RESPONSE);

        packet.write_u32(self.quest_id);
        packet.write_u32(self.method);
        packet.write_u32(self.quest_level);
        packet.write_i32(self.zone_or_sort);
        packet.write_u32(self.quest_type);
        packet.write_u32(self.rep_objective_faction);
        packet.write_u32(self.rep_objective_value as u32);
        packet.write_u32(0); // RequiredOpositeRepFaction
        packet.write_u32(0); // RequiredOpositeRepValue
        packet.write_u32(self.next_quest_in_chain);

        // Money reward (hidden if QUEST_FLAGS_HIDDEN_REWARDS)
        if self.quest_flags.has_flag(QuestFlags::HIDDEN_REWARDS) {
            packet.write_u32(0);
        } else {
            packet.write_u32(self.rew_or_req_money as u32);
        }

        packet.write_u32(self.rew_money_max_level);
        packet.write_u32(self.rew_spell);
        packet.write_u32(self.src_item_id);
        packet.write_u32(self.quest_flags.0);

        // Fixed rewards
        for i in 0..QUEST_REWARDS_COUNT {
            packet.write_u32(self.rew_item_id[i]);
            packet.write_u32(self.rew_item_count[i]);
        }

        // Reward choices
        for i in 0..QUEST_REWARD_CHOICES_COUNT {
            packet.write_u32(self.rew_choice_item_id[i]);
            packet.write_u32(self.rew_choice_item_count[i]);
        }

        packet.write_u32(self.point_map_id);
        packet.write_f32(self.point_x);
        packet.write_f32(self.point_y);
        packet.write_u32(self.point_opt);

        packet.write_string(self.title);
        packet.write_string(self.objectives);
        packet.write_string(self.details);
        packet.write_string(self.end_text);

        // Objectives
        for obj in &self.objectives_data {
            // Creature/GO ID (GO has 0x80000000 flag)
            let id = if obj.creature_or_go_id < 0 {
                // GameObject ID - encode with 0x80000000 flag
                ((-obj.creature_or_go_id) as u32) | 0x80000000u32
            } else {
                // Creature ID
                obj.creature_or_go_id as u32
            };
            packet.write_u32(id);
            packet.write_u32(obj.creature_or_go_count);
            packet.write_u32(obj.item_id);
            packet.write_u32(obj.item_count);
        }

        // Objective text
        for text in self.objective_text.iter() {
            packet.write_string(text);
        }

        packet
    }
}

/// SMSG_QUEST_CONFIRM_ACCEPT - Quest confirm accept response
///
/// Sent in response to CMSG_QUEST_CONFIRM_ACCEPT to confirm quest was added.
#[derive(Debug, Clone)]
pub struct SmsgQuestConfirmAccept {
    /// Quest ID
    pub quest_id: u32,
    /// Quest title
    pub title: String,
}

impl ToWorldPacket for SmsgQuestConfirmAccept {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUEST_CONFIRM_ACCEPT);
        packet.write_u32(self.quest_id);
        packet.write_string(&self.title);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_questlog_full() {
        let msg = SmsgQuestlogFull;
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTLOG_FULL);
    }

    #[test]
    fn test_smsg_questupdate_complete() {
        let msg = SmsgQuestupdateComplete { quest_id: 123 };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_COMPLETE);
    }

    #[test]
    fn test_smsg_questupdate_failed() {
        let msg = SmsgQuestupdateFailed { quest_id: 123 };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_FAILED);
    }

    #[test]
    fn test_smsg_questupdate_failedtimer() {
        let msg = SmsgQuestupdateFailedtimer { quest_id: 123 };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_FAILEDTIMER);
    }

    #[test]
    fn test_smsg_questgiver_quest_invalid() {
        let msg = SmsgQuestgiverQuestInvalid { reason: 1 };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_INVALID);
    }

    #[test]
    fn test_smsg_questgiver_quest_complete() {
        let msg = SmsgQuestgiverQuestComplete {
            quest_id: 123,
            xp: 1000,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_COMPLETE);
    }

    #[test]
    fn test_smsg_questupdate_add_item() {
        let msg = SmsgQuestupdateAddItem {
            item_id: 456,
            count: 5,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_ADD_ITEM);
    }

    #[test]
    fn test_smsg_questgiver_status() {
        let msg = SmsgQuestgiverStatus {
            guid: ObjectGuid::from_low(789),
            status: DialogStatus::Available,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_STATUS);
    }

    #[test]
    fn test_smsg_questupdate_add_kill() {
        let msg = SmsgQuestupdateAddKill {
            quest_id: 123,
            entry: 456,
            count: 3,
            required_count: 10,
            guid: ObjectGuid::from_low(789),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_ADD_KILL);
    }
}
