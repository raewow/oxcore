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

use crate::messages::gossip::{write_modern_gossip_quest, GossipQuestData};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

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
    fn to_vanilla(&self) -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_QUESTLOG_FULL)
    }

    /// Empty in both protocols.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_COMPLETE);
        packet.write_u32(self.quest_id);
        packet
    }

    /// `QuestUpdateStatus`, per the 1.14 wire format.:
    /// the proxy forwards the quest id unchanged, so the body is identical to vanilla's and only the
    /// opcode differs.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_FAILED);
        packet.write_u32(self.quest_id);
        packet
    }

    /// `QuestUpdateStatus`, per the 1.14 wire format.:
    /// the proxy forwards the quest id unchanged, so the body is identical to vanilla's and only the
    /// opcode differs.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_FAILEDTIMER);
        packet.write_u32(self.quest_id);
        packet
    }

    /// `QuestUpdateStatus`, per the 1.14 wire format.:
    /// the proxy forwards the quest id unchanged, so the body is identical to vanilla's and only the
    /// opcode differs.
    fn to_modern(&self) -> Option<WorldPacket> {
        Some(self.to_vanilla())
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_QUEST_INVALID);
        packet.write_u32(self.reason);
        packet
    }

    /// `QuestGiverInvalidQuest::Write`, per the 1.14 wire format.
    ///
    /// Gains a contribution-reward id and an optional override string. We send neither: the reason
    /// code alone is what selects the client's own error text, which is what vanilla relies on.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.reason);
        writer.write_i32(0); // ContributionRewardID
        writer.write_bit(false); // SendErrorMessage -- use the client's text for this reason
        writer.write_bits(0, 9); // ReasonText length
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_QUESTGIVER_QUEST_INVALID))
    }
}

/// SMSG_QUESTGIVER_QUEST_COMPLETE - Quest reward received
///
/// Sent when player receives quest rewards.
#[derive(Debug, Clone)]
pub struct SmsgQuestgiverQuestComplete<'a> {
    /// Quest ID
    pub quest_id: u32,
    /// XP reward amount
    pub xp: u32,
    /// Money reward amount
    pub money: u32,
    /// Fixed reward items
    pub reward_items: &'a [(u32, u32)],
    /// Whether the quest giver has a follow-up quest to offer straight away
    pub launch_quest: bool,
    /// Whether the quest giver should fall back to its gossip menu
    pub launch_gossip: bool,
}

impl ToWorldPacket for SmsgQuestgiverQuestComplete<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_QUEST_COMPLETE);
        packet.write_u32(self.quest_id);
        packet.write_u32(0x03); // Unknown flag
        packet.write_u32(self.xp);
        packet.write_u32(self.money);
        packet.write_u32(self.reward_items.len() as u32);
        for &(item_id, count) in self.reward_items {
            packet.write_u32(item_id);
            packet.write_u32(count);
        }
        packet
    }

    /// `QuestGiverQuestComplete::Write`, per the 1.14 wire format.
    ///
    /// 1.14 carries a *single* `ItemReward` rather than vanilla's list, so only the first reward item
    /// makes it into the packet. The rest still reach the player — they arrive as inventory updates —
    /// this message just names one of them for the "you receive" toast. Money also widens to i64.
    ///
    /// The item is written with four bits followed by a u32 **without flushing**: `ItemInstance`
    /// starts with a u32, whose write flushes the partial byte, so the layout still lands on a byte
    /// boundary. Our `BitWriter` flushes on byte writes too, so the same code produces the same bytes.
    ///
    /// `LaunchQuest` and `LaunchGossip` tell the client what happens to the quest-giver frame once the
    /// turn-in toast clears. Setting `LaunchQuest` with nothing to follow is actively harmful: the
    /// client waits for a quest dialog that never comes, then retries the whole
    /// hello → complete → request-reward → choose-reward chain every ~100 ms to recover. Both must be
    /// false when the giver has neither a follow-up quest nor a gossip menu.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.quest_id);
        writer.write_u32(self.xp);
        writer.write_i64(i64::from(self.money));
        writer.write_u32(0); // SkillLineIDReward
        writer.write_u32(0); // NumSkillUpsReward

        writer.write_bit(false); // UseQuestReward
        writer.write_bit(self.launch_gossip);
        writer.write_bit(self.launch_quest);
        writer.write_bit(false); // HideChatMessage

        // ItemInstance for the single reward slot.
        let item_id = self.reward_items.first().map_or(0, |&(id, _)| id);
        writer.write_u32(item_id);
        writer.write_u32(0); // RandomPropertiesSeed
        writer.write_u32(0); // RandomPropertiesID
        writer.write_bit(false); // HasItemBonus
        writer.flush_bits();
        writer.write_bits(0, 6); // ItemModList count
        writer.flush_bits();

        Some(writer.finish(Opcode::SMSG_QUESTGIVER_QUEST_COMPLETE))
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
    fn to_vanilla(&self) -> WorldPacket {
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_STATUS);
        packet.write_guid_raw(self.guid.raw());
        packet.write_u32(self.status as u32);
        packet
    }

    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(modern_dialog_status(self.status));
        Some(writer.finish(Opcode::SMSG_QUESTGIVER_STATUS))
    }
}

/// One questgiver's marker, for [`SmsgQuestgiverStatusMultiple`].
#[derive(Debug, Clone)]
pub struct QuestGiverStatusEntry {
    pub guid: ObjectGuid,
    pub status: DialogStatus,
}

/// SMSG_QUESTGIVER_STATUS_MULTIPLE - every visible questgiver's marker at once
///
/// Modern only; 1.12 has no batch form and repaints one NPC at a time. The 1.14 client drives *all*
/// of its markers from this message, so a short or empty list does not mean "no change" — it means
/// the omitted NPCs keep whatever marker they last had.
#[derive(Debug, Clone)]
pub struct SmsgQuestgiverStatusMultiple<'a> {
    pub statuses: &'a [QuestGiverStatusEntry],
}

impl ToWorldPacket for SmsgQuestgiverStatusMultiple<'_> {
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(self.statuses.len() as i32);
        for entry in self.statuses {
            let (high, low) = entry.guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_u32(modern_dialog_status(entry.status));
        }
        Some(writer.finish(Opcode::SMSG_QUESTGIVER_STATUS_MULTIPLE))
    }

    /// No vanilla counterpart, so this is the count-only body a 1.12 client would ignore anyway.
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTGIVER_STATUS_MULTIPLE);
        packet.write_i32(0);
        packet
    }
}

/// Translate vanilla's sequential dialog-status enum into 1.14's flag set.
///
/// 1.14 turned the status into a bit per marker kind and inserted several new ones, so the numbering
/// no longer lines up at all: vanilla's `Available` (5) reads as an invalid combination of
/// `Unavailable | LowLevelAvailable` if passed through. Shared by the single and batch messages so
/// the two cannot disagree about what a marker means.
fn modern_dialog_status(status: DialogStatus) -> u32 {
    match status {
        DialogStatus::None => 0x000000,
        DialogStatus::Unavailable => 0x000002,
        DialogStatus::Chat => 0x000004, // Vanilla's low-level-available slot.
        DialogStatus::Incomplete => 0x000020,
        DialogStatus::RewardRep => 0x000100,
        DialogStatus::Available => 0x000400,
        DialogStatus::RewardOld => 0x000800,
        DialogStatus::Reward2 => 0x001000,
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUESTUPDATE_ADD_KILL);
        packet.write_u32(self.quest_id);
        packet.write_u32(self.entry);
        packet.write_u32(self.count);
        packet.write_u32(self.required_count);
        packet.write_guid_raw(self.guid.raw());
        packet
    }

    /// `QuestUpdateAddCredit::Write`, per the 1.14 wire format. 1.14 renamed the message
    /// `SMSG_QUEST_UPDATE_ADD_CREDIT`; the opcode table carries the new wire value under the vanilla
    /// name.
    ///
    /// Reordered and narrowed: the victim GUID leads, both counts drop to u16, and an objective type
    /// byte is appended. `ObjectiveType` 0 is "kill a creature", which is the only thing vanilla
    /// sends this message for — item pickups go through the quest-log objective fields instead.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // VictimGUID
        writer.write_u32(self.quest_id);
        writer.write_i32(self.entry as i32); // ObjectID
        writer.write_u16(self.count.min(u32::from(u16::MAX)) as u16);
        writer.write_u16(self.required_count.min(u32::from(u16::MAX)) as u16);
        writer.write_u8(0); // ObjectiveType: monster kill
        Some(writer.finish(Opcode::SMSG_QUESTUPDATE_ADD_KILL))
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `QuestGiverQuestListMessage::Write`, per the 1.14 wire format.
    ///
    /// Reordered rather than renumbered: the greeting moves to the *end*, behind an 11-bit length
    /// prefix, and the quest count widens from u8 to i32 and moves ahead of the emote fields. Each
    /// entry is the same `ClientGossipQuest` shape the gossip menu uses, so it shares that writer.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);

        writer.write_u32(self.emote_delay);
        writer.write_u32(self.emote);
        writer.write_i32(self.quests.len() as i32);

        let greeting = self.title.as_bytes();
        writer.write_bits(greeting.len() as u32, 11);
        writer.flush_bits();

        for quest in self.quests {
            write_modern_gossip_quest(
                &mut writer,
                &GossipQuestData {
                    quest_id: quest.quest_id,
                    icon: quest.icon,
                    level: quest.level,
                    title: quest.title.clone(),
                },
            );
        }

        // The greeting trails the list, where vanilla puts it right after the GUID.
        writer.write_bytes(greeting);

        Some(writer.finish(Opcode::SMSG_QUESTGIVER_QUEST_LIST))
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

/// `StatusFlags` for `SMSG_QUESTGIVER_REQUEST_ITEMS`, which gates the Complete button.
///
/// Only bit `0x04` distinguishes the two; the rest are on in both cases and enable the frame's
/// ordinary controls. They are constants rather than a computed mask because the client is the only
/// consumer and it only ever sees these two.
const STATUS_FLAGS_CAN_COMPLETE: u32 = 0xDF;
const STATUS_FLAGS_INCOMPLETE: u32 = 0xDB;

impl ToWorldPacket for SmsgQuestgiverRequestItemsV2<'_> {
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `QuestGiverRequestItems::Write`, per the 1.14 wire format.
    ///
    /// Vanilla's four opaque trailing flag words collapse into a single `StatusFlags`, the strings
    /// move to the end behind 9- and 12-bit lengths, and each required item gains a `Flags` word.
    /// `MoneyToGet` becomes signed.
    ///
    /// `StatusFlags` is **not** vanilla's flag word passed through. It is a bit set the client reads
    /// to decide which controls the frame gets, and it has exactly two useful values: `0xDF` when the
    /// quest can be handed in and `0xDB` when it cannot. Forwarding vanilla's `0x03`/`0x00` leaves
    /// nearly every bit clear, and the client responds by drawing the frame with no completion text
    /// and no Complete button — it looks like a dialog that failed to load rather than a parse error.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.guid.entry()); // QuestGiverCreatureID
        writer.write_u32(self.quest_id);
        writer.write_u32(0); // CompEmoteDelay -- vanilla always sends 0 here too
        writer.write_u32(if self.completable {
            self.complete_emote
        } else {
            self.incomplete_emote
        });
        writer.write_u32(0); // Flags
        writer.write_u32(0); // FlagsEx
        writer.write_u32(0); // SuggestPartyMembers
        writer.write_i32(self.req_money as i32);
        writer.write_i32(self.req_items.len() as i32);
        writer.write_i32(0); // Currency count
        writer.write_u32(if self.completable {
            STATUS_FLAGS_CAN_COMPLETE
        } else {
            STATUS_FLAGS_INCOMPLETE
        });

        for item in self.req_items {
            writer.write_u32(item.item_id); // ObjectID
            writer.write_u32(item.count); // Amount
            writer.write_u32(0); // Flags
        }
        // No currencies in Classic Era, so nothing follows.

        writer.write_bit(self.close_on_cancel); // AutoLaunched
        writer.flush_bits();

        let title = self.title.as_bytes();
        let completion = self.request_items_text.as_bytes();
        writer.write_bits(title.len() as u32, 9);
        writer.write_bits(completion.len() as u32, 12);

        writer.write_bytes(title);
        writer.write_bytes(completion);

        Some(writer.finish(Opcode::SMSG_QUESTGIVER_REQUEST_ITEMS))
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
    /// Spell cast when the reward is claimed
    pub rew_spell_cast: u32,
    /// Offer reward emotes
    pub offer_reward_emote: [u32; QUEST_EMOTE_COUNT],
    /// Offer reward emote delays
    pub offer_reward_emote_delay: [u32; QUEST_EMOTE_COUNT],
}

impl ToWorldPacket for SmsgQuestgiverOfferRewardV2<'_> {
    fn to_vanilla(&self) -> WorldPacket {
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
        packet.write_u32(self.rew_spell_cast);

        packet
    }

    /// `QuestGiverOfferRewardMessage::Write` plus the nested `QuestGiverOfferReward::Write`.
    ///
    /// The nesting is the notable part: the inner block — GUID, ids, flags, emotes, then the whole
    /// reward array — is written *first*, and the outer message's portraits and six bit-packed string
    /// lengths follow it, with the strings last. Vanilla interleaves text and rewards instead.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();

        // --- QuestGiverOfferReward (the inner block) ---
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u32(self.guid.entry()); // QuestGiverCreatureID
        writer.write_u32(self.quest_id);
        writer.write_u32(self.quest_flags.0); // Flags
        writer.write_u32(0); // FlagsEx
        writer.write_u32(0); // SuggestedPartyMembers

        let emotes: Vec<(u32, u32)> = self
            .offer_reward_emote
            .iter()
            .zip(&self.offer_reward_emote_delay)
            .filter(|(emote, _)| **emote != 0)
            .map(|(emote, delay)| (*emote, *delay))
            .collect();
        writer.write_i32(emotes.len() as i32);
        for (emote, delay) in &emotes {
            writer.write_u32(*emote);
            writer.write_u32(*delay);
        }

        writer.write_bit(self.enable_next); // AutoLaunched
        writer.write_bit(false); // Unused
        writer.flush_bits();

        write_modern_quest_rewards(
            &mut writer,
            self.reward_choices,
            self.reward_items,
            self.money_reward,
            0,
            self.rew_spell,
        );

        // --- QuestGiverOfferRewardMessage (the outer message) ---
        writer.write_u32(0); // QuestPackageID
        writer.write_u32(0); // PortraitGiver
        writer.write_u32(0); // PortraitGiverMount
        writer.write_u32(0); // PortraitGiverModelSceneID
        writer.write_u32(0); // PortraitTurnIn

        let title = self.title.as_bytes();
        let reward_text = self.offer_reward_text.as_bytes();
        writer.write_bits(title.len() as u32, 9);
        writer.write_bits(reward_text.len() as u32, 12);
        writer.write_bits(0, 10); // PortraitGiverText
        writer.write_bits(0, 8); // PortraitGiverName
        writer.write_bits(0, 10); // PortraitTurnInText
        writer.write_bits(0, 8); // PortraitTurnInName
                                 // No FlushBits here: the strings are written straight after, and `write_bytes`
                                 // flushes the partial byte itself.

        writer.write_bytes(title);
        writer.write_bytes(reward_text);

        Some(writer.finish(Opcode::SMSG_QUESTGIVER_OFFER_REWARD))
    }
}

// =============================================================================
// Modern quest-reward encoding
// =============================================================================

/// Fixed array sizes 1.14 writes unconditionally in `QuestRewards`, per the 1.14 wire format.
///
/// These are *not* counts of what we have — the client reads exactly this many entries every time, so
/// a short reward list is zero-padded. Getting one wrong shifts everything after it.
const MODERN_REWARD_ITEM_COUNT: usize = 4;
const MODERN_REWARD_CHOICE_COUNT: usize = 6;
const MODERN_REWARD_REPUTATION_COUNT: usize = 5;
const MODERN_REWARD_CURRENCY_COUNT: usize = 4;
const MODERN_REWARD_DISPLAY_SPELL_COUNT: usize = 3;

/// `QuestRewards::Write`, per the 1.14 wire format.
///
/// Shared by the quest-details and offer-reward dialogs, which carry the identical block. Vanilla
/// sends length-prefixed lists of `(item, count, display)`; 1.14 sends fixed-width arrays and splits
/// the choice items into their own `QuestChoiceItem` shape with an `ItemInstance` inside.
fn write_modern_quest_rewards(
    writer: &mut BitWriter,
    choices: &[RewardItemInfo],
    items: &[RewardItemInfo],
    money: u32,
    xp: u32,
    spell_completion_id: u32,
) {
    writer.write_u32(choices.len() as u32); // ChoiceItemCount
    writer.write_u32(items.len() as u32); // ItemCount

    for index in 0..MODERN_REWARD_ITEM_COUNT {
        let item = items.get(index);
        writer.write_u32(item.map_or(0, |i| i.item_id));
        writer.write_u32(item.map_or(0, |i| i.count));
    }

    writer.write_u32(money);
    writer.write_u32(xp);
    writer.write_u64(0); // ArtifactXP -- retail only
    writer.write_u32(0); // ArtifactCategoryID
    writer.write_u32(0); // Honor
    writer.write_u32(0); // Title
    writer.write_u32(0); // FactionFlags

    for _ in 0..MODERN_REWARD_REPUTATION_COUNT {
        writer.write_u32(0); // FactionID
        writer.write_i32(0); // FactionValue
        writer.write_i32(0); // FactionOverride
                             // Every cap is seeded at 7, so match it rather than zero.
        writer.write_i32(7); // FactionCapIn
    }

    for _ in 0..MODERN_REWARD_DISPLAY_SPELL_COUNT {
        writer.write_i32(0); // SpellCompletionDisplayID
    }
    writer.write_u32(spell_completion_id);

    for _ in 0..MODERN_REWARD_CURRENCY_COUNT {
        writer.write_u32(0); // CurrencyID
        writer.write_u32(0); // CurrencyQty
    }

    writer.write_u32(0); // SkillLineID
    writer.write_u32(0); // NumSkillUps
    writer.write_u32(0); // TreasurePickerID

    for index in 0..MODERN_REWARD_CHOICE_COUNT {
        write_modern_quest_choice_item(writer, choices.get(index));
    }

    writer.write_bit(false); // IsBoostSpell
    writer.flush_bits();
}

/// `QuestChoiceItem::Write` with the `ItemInstance` it contains.
///
/// An absent choice still writes the whole shape with zeros — the array is fixed-width.
fn write_modern_quest_choice_item(writer: &mut BitWriter, choice: Option<&RewardItemInfo>) {
    writer.write_bits(0, 2); // LootItemType: 0 = item
                             // ItemInstance
    writer.write_u32(choice.map_or(0, |c| c.item_id));
    writer.write_u32(0); // RandomPropertiesSeed
    writer.write_u32(0); // RandomPropertiesID
    writer.write_bit(false); // HasItemBonus
    writer.flush_bits();
    writer.write_bits(0, 6); // ItemModList count
    writer.flush_bits();
    writer.write_u32(choice.map_or(0, |c| c.count)); // Quantity
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `QuestGiverQuestDetails::Write`, per the 1.14 wire format.
    ///
    /// A reshape. Everything variable-length moves to the end: all four counts and the fixed fields
    /// come first, then the arrays, then a bit run holding *seven* string lengths at differing widths
    /// (9, 12, 12, 10, 8, 10, 8) and four flags, then the reward block, then the strings themselves.
    ///
    /// Fields with no 1.12 source — quest packages, portraits, objective ids, the session bonus — are
    /// written as defaults rather than guessed at. The `HIDDEN_REWARDS` flag is
    /// honoured the same way `to_vanilla` honours it.
    fn to_modern(&self) -> Option<WorldPacket> {
        let hidden = self.quest_flags.has_flag(QuestFlags::HIDDEN_REWARDS);
        let choices: &[RewardItemInfo] = if hidden { &[] } else { self.reward_choices };
        let items: &[RewardItemInfo] = if hidden { &[] } else { self.reward_items };
        let money = if hidden { 0 } else { self.money_reward };

        let mut writer = BitWriter::new();
        let (high, low) = self.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // QuestGiverGUID
        writer.write_packed_guid_128(0, 0); // InformUnit -- no quest-sharing source here

        writer.write_u32(self.quest_id);
        writer.write_i32(0); // QuestPackageID
        writer.write_u32(0); // PortraitGiver
        writer.write_u32(0); // PortraitGiverMount
        writer.write_u32(0); // PortraitGiverModelSceneID
        writer.write_u32(0); // PortraitTurnIn
        writer.write_u32(self.quest_flags.0); // Flags
        writer.write_u32(0); // FlagsEx
        writer.write_u32(0); // SuggestedPartyMembers

        // Emotes are a *counted* array here, where vanilla always writes QUEST_EMOTE_COUNT and pads.
        let emotes: Vec<(u32, u32)> = self
            .details_emote
            .iter()
            .zip(&self.details_emote_delay)
            .filter(|(emote, _)| **emote != 0)
            .map(|(emote, delay)| (*emote, *delay))
            .collect();

        writer.write_i32(0); // LearnSpells count
        writer.write_i32(emotes.len() as i32);
        // Objectives are the 1.14 quest-objective system, which has no 1.12 equivalent: vanilla
        // carries objectives only as the display text below.
        writer.write_i32(0); // Objectives count
        writer.write_i32(0); // QuestStartItemID
        writer.write_i32(0); // QuestSessionBonus

        for (emote, delay) in &emotes {
            writer.write_u32(*emote); // Type
            writer.write_u32(*delay); // Delay
        }

        let title = self.title.as_bytes();
        let description = self.details.as_bytes();
        let log_description = self.objectives.as_bytes();

        writer.write_bits(title.len() as u32, 9);
        writer.write_bits(description.len() as u32, 12);
        writer.write_bits(log_description.len() as u32, 12);
        writer.write_bits(0, 10); // PortraitGiverText
        writer.write_bits(0, 8); // PortraitGiverName
        writer.write_bits(0, 10); // PortraitTurnInText
        writer.write_bits(0, 8); // PortraitTurnInName
        writer.write_bit(self.activate_accept); // AutoLaunched
        writer.write_bit(false); // unused in client
        writer.write_bit(false); // StartCheat
        writer.write_bit(true); // DisplayPopup
        writer.flush_bits();

        write_modern_quest_rewards(&mut writer, choices, items, money, 0, self.rew_spell);

        writer.write_bytes(title);
        writer.write_bytes(description);
        writer.write_bytes(log_description);
        // The four portrait strings had zero lengths above, so nothing follows.

        Some(writer.finish(Opcode::SMSG_QUESTGIVER_QUEST_DETAILS))
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
    pub min_level: u32,
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
    pub rew_spell_cast: u32,
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
    pub suggested_players: u32,
    pub limit_time: u32,
    pub rew_rep_faction: [u32; 5],
    pub rew_rep_value: [i32; 5],
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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// 1.14's query response is a new template format rather than a widened vanilla body. Fields
    /// with no 1.12 source use harmless defaults.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.quest_id);
        writer.write_bit(true); // Allow
        writer.flush_bits();

        writer.write_u32(self.quest_id);
        writer.write_i32(self.method as i32);
        writer.write_i32(self.quest_level as i32);
        writer.write_i32(0); // QuestScalingFactionGroup
        writer.write_i32(255); // QuestMaxScalingLevel
        writer.write_u32(0); // QuestPackageID
        writer.write_i32(self.min_level as i32);
        writer.write_i32(self.zone_or_sort);
        writer.write_u32(self.quest_type);
        writer.write_u32(self.suggested_players);
        writer.write_u32(self.next_quest_in_chain);
        writer.write_u32(0); // RewardXPDifficulty
        writer.write_f32(1.0); // RewardXPMultiplier
        writer.write_i32(self.rew_or_req_money.max(0));
        writer.write_u32(0); // RewardMoneyDifficulty
        writer.write_f32(1.0); // RewardMoneyMultiplier
        writer.write_u32(self.rew_money_max_level);
        writer.write_u32(self.rew_spell);
        writer.write_u32(0);
        writer.write_u32(0); // RewardDisplaySpell
        writer.write_u32(self.rew_spell_cast);
        writer.write_u32(0); // RewardHonor
        writer.write_f32(0.0); // RewardKillHonor
        writer.write_i32(0); // RewardArtifactXPDifficulty
        writer.write_f32(1.0); // RewardArtifactXPMultiplier
        writer.write_i32(0); // RewardArtifactCategoryID
        writer.write_u32(self.src_item_id);
        writer.write_u32(self.quest_flags.0);
        writer.write_u32(0); // FlagsEx
        writer.write_u32(0); // FlagsEx2

        for index in 0..QUEST_REWARDS_COUNT {
            writer.write_u32(self.rew_item_id[index]);
            writer.write_u32(self.rew_item_count[index]);
            writer.write_i32(0); // ItemDrop
            writer.write_i32(0); // ItemDropQuantity
        }
        for index in 0..QUEST_REWARD_CHOICES_COUNT {
            writer.write_u32(self.rew_choice_item_id[index]);
            writer.write_u32(self.rew_choice_item_count[index]);
            writer.write_u32(0); // Choice.DisplayID
        }

        writer.write_u32(self.point_map_id);
        writer.write_f32(self.point_x);
        writer.write_f32(self.point_y);
        writer.write_u32(self.point_opt);
        writer.write_u32(0); // RewardTitle
        writer.write_i32(0); // RewardArenaPoints
        writer.write_u32(0); // RewardSkillLineID
        writer.write_u32(0); // RewardNumSkillUps
        writer.write_u32(0); // PortraitGiver
        writer.write_u32(0); // PortraitGiverMount
        writer.write_u32(0); // PortraitTurnIn
        writer.write_i32(0); // Unknown_2_5_2
        for index in 0..5 {
            writer.write_u32(self.rew_rep_faction[index]);
            writer.write_i32(self.rew_rep_value[index]);
            writer.write_i32(0); // RewardFactionOverride
            writer.write_i32(7); // RewardFactionCapIn
        }
        writer.write_u32(0); // RewardFactionFlags
        for _ in 0..4 {
            writer.write_u32(0); // RewardCurrencyID
            writer.write_u32(0); // RewardCurrencyQty
        }
        writer.write_u32(890); // AcceptedSoundKitID
        writer.write_u32(878); // CompleteSoundKitID
        writer.write_u32(0); // AreaGroupID
        writer.write_u32(self.limit_time);

        let mut objectives: Vec<(u8, i32, i32)> = Vec::new();
        if self.rep_objective_faction != 0 && self.rep_objective_value != 0 {
            objectives.push((
                6,
                self.rep_objective_faction as i32,
                self.rep_objective_value,
            ));
        }
        if self.rew_or_req_money < 0 {
            objectives.push((8, 0, -self.rew_or_req_money));
        }
        for objective in &self.objectives_data {
            if objective.creature_or_go_id != 0 && objective.creature_or_go_count != 0 {
                let (kind, object_id) = if objective.creature_or_go_id < 0 {
                    (2, -objective.creature_or_go_id)
                } else {
                    (0, objective.creature_or_go_id)
                };
                objectives.push((kind, object_id, objective.creature_or_go_count as i32));
            }
        }
        for objective in &self.objectives_data {
            if objective.item_id != 0 && objective.item_count != 0 {
                objectives.push((1, objective.item_id as i32, objective.item_count as i32));
            }
        }

        writer.write_i32(objectives.len() as i32);
        writer.write_i64(511); // AllowableRaces
        writer.write_i32(0); // TreasurePickerID
        writer.write_i32(0); // Expansion

        writer.write_bits(self.title.len() as u32, 9); // LogTitle
        writer.write_bits(self.objectives.len() as u32, 12); // LogDescription
        writer.write_bits(self.details.len() as u32, 12); // QuestDescription
        writer.write_bits(self.end_text.len() as u32, 9); // AreaDescription
        writer.write_bits(0, 10); // PortraitGiverText
        writer.write_bits(0, 8); // PortraitGiverName
        writer.write_bits(0, 10); // PortraitTurnInText
        writer.write_bits(0, 8); // PortraitTurnInName
        writer.write_bits(0, 11); // QuestCompletionLog
        writer.write_bit(false); // ReadyForTranslation
        writer.flush_bits();

        for (index, (kind, object_id, amount)) in objectives.iter().enumerate() {
            let description = self
                .objective_text
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            writer.write_u32(index as u32);
            writer.write_u8(*kind);
            writer.write_u8(index as u8); // StorageIndex
            writer.write_i32(*object_id);
            writer.write_i32(*amount);
            writer.write_u32(0); // Flags
            writer.write_u32(0); // Flags2
            writer.write_f32(0.0); // ProgressBarWeight
            writer.write_i32(0); // VisualEffects.Count
            writer.write_bits(description.len() as u32, 8);
            writer.flush_bits();
            writer.write_string_raw(description);
        }

        for text in [self.title, self.objectives, self.details, self.end_text] {
            writer.write_string_raw(text);
        }

        Some(writer.finish(Opcode::SMSG_QUEST_QUERY_RESPONSE))
    }
}

/// SMSG_QUEST_CONFIRM_ACCEPT - Quest confirm accept response
///
/// Sent in response to CMSG_QUEST_CONFIRM_ACCEPT to confirm quest was added.
/// Serializes: quest_id, title, sender_guid (packed).
#[derive(Debug, Clone)]
pub struct SmsgQuestConfirmAccept {
    /// Quest ID
    pub quest_id: u32,
    /// Quest title
    pub title: String,
    /// GUID of the player who accepted the quest (the party member who shares it).
    pub sender_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgQuestConfirmAccept {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_QUEST_CONFIRM_ACCEPT);
        packet.write_u32(self.quest_id);
        packet.write_string(&self.title);
        packet.write_packed_guid(self.sender_guid);
        packet
    }
}

/// MSG_QUEST_PUSH_RESULT - Quest push result notification
///
/// Bidirectional packet used to notify the original sharer about the
/// accept/decline/distance/etc. status of a shared quest offer.
/// Serializes: sender_guid (packed), msg (u8).
#[derive(Debug, Clone)]
pub struct MsgQuestPushResult {
    /// GUID of the player who received the push.
    pub sender_guid: ObjectGuid,
    /// Result code from QuestShareMessages.
    pub msg: u8,
}

impl ToWorldPacket for MsgQuestPushResult {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_QUEST_PUSH_RESULT);
        packet.write_packed_guid(self.sender_guid);
        packet.write_u8(self.msg);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::bitbuf::BitReader;
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_questlog_full() {
        let msg = SmsgQuestlogFull;
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTLOG_FULL);
    }

    #[test]
    fn test_smsg_questupdate_complete() {
        let msg = SmsgQuestupdateComplete { quest_id: 123 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_COMPLETE);
    }

    #[test]
    fn test_smsg_questupdate_failed() {
        let msg = SmsgQuestupdateFailed { quest_id: 123 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_FAILED);
    }

    #[test]
    fn test_smsg_questupdate_failedtimer() {
        let msg = SmsgQuestupdateFailedtimer { quest_id: 123 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_FAILEDTIMER);
    }

    #[test]
    fn test_smsg_questgiver_quest_invalid() {
        let msg = SmsgQuestgiverQuestInvalid { reason: 1 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_INVALID);
    }

    #[test]
    fn test_smsg_questgiver_quest_complete() {
        let msg = SmsgQuestgiverQuestComplete {
            quest_id: 123,
            xp: 1000,
            money: 500,
            reward_items: &[(456, 2)],
            launch_quest: false,
            launch_gossip: false,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_COMPLETE);
        assert_eq!(packet.data().len(), 28);
        assert_eq!(
            u32::from_le_bytes(packet.data()[12..16].try_into().unwrap()),
            500
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[16..20].try_into().unwrap()),
            1
        );
    }

    #[test]
    fn test_smsg_questgiver_offer_reward_includes_cast_spell() {
        let msg = SmsgQuestgiverOfferRewardV2 {
            guid: ObjectGuid::from_low(1),
            quest_id: 783,
            title: "",
            offer_reward_text: "",
            enable_next: true,
            reward_choices: &[],
            reward_items: &[],
            money_reward: 0,
            quest_flags: QuestFlags(0),
            rew_spell: 10,
            rew_spell_cast: 20,
            offer_reward_emote: [0; QUEST_EMOTE_COUNT],
            offer_reward_emote_delay: [0; QUEST_EMOTE_COUNT],
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_OFFER_REWARD);
        assert_eq!(
            u32::from_le_bytes(packet.data()[packet.data().len() - 4..].try_into().unwrap()),
            20
        );
    }

    #[test]
    fn test_smsg_questupdate_add_item() {
        let msg = SmsgQuestupdateAddItem {
            item_id: 456,
            count: 5,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_ADD_ITEM);
    }

    #[test]
    fn test_smsg_questgiver_status() {
        let msg = SmsgQuestgiverStatus {
            guid: ObjectGuid::from_low(789),
            status: DialogStatus::Available,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTGIVER_STATUS);
    }

    #[test]
    fn modern_questgiver_status_uses_modern_flag_values() {
        let packet = SmsgQuestgiverStatus {
            guid: ObjectGuid::new_creature(197, 42),
            status: DialogStatus::Available,
        }
        .to_modern()
        .expect("modern status packet");
        let mut reader = BitReader::new(packet.data());
        assert!(reader.read_packed_guid_128().is_some());
        assert_eq!(reader.read_u32(), Some(0x000400));
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
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUESTUPDATE_ADD_KILL);
    }

    #[test]
    fn test_smsg_quest_query_response() {
        let objectives_data = [
            QuestObjectiveData {
                creature_or_go_id: 7,
                creature_or_go_count: 3,
                item_id: 11,
                item_count: 2,
            },
            QuestObjectiveData::default(),
            QuestObjectiveData::default(),
            QuestObjectiveData::default(),
        ];
        let objective_text = [
            String::from("A"),
            String::from("B"),
            String::from("C"),
            String::from("D"),
        ];

        let msg = SmsgQuestQueryResponseV2 {
            quest_id: 123,
            method: 2,
            quest_level: 60,
            min_level: 1,
            zone_or_sort: -42,
            quest_type: 81,
            rep_objective_faction: 77,
            rep_objective_value: -12,
            next_quest_in_chain: 999,
            rew_or_req_money: 555,
            rew_money_max_level: 777,
            rew_spell: 888,
            rew_spell_cast: 889,
            src_item_id: 999,
            quest_flags: QuestFlags::default(),
            rew_item_id: [1, 2, 3, 4],
            rew_item_count: [5, 6, 7, 8],
            rew_choice_item_id: [9, 10, 11, 12, 13, 14],
            rew_choice_item_count: [15, 16, 17, 18, 19, 20],
            point_map_id: 21,
            point_x: 1.5,
            point_y: -2.5,
            point_opt: 22,
            suggested_players: 1,
            limit_time: 0,
            rew_rep_faction: [0; 5],
            rew_rep_value: [0; 5],
            title: "Quest title",
            objectives: "Quest objectives",
            details: "Quest details",
            end_text: "Quest end",
            objectives_data: objectives_data.clone(),
            objective_text: &objective_text,
        };

        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUEST_QUERY_RESPONSE);
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            123
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[4..8].try_into().unwrap()),
            2
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[8..12].try_into().unwrap()),
            60
        );
        assert_eq!(
            i32::from_le_bytes(packet.data()[12..16].try_into().unwrap()),
            -42
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[16..20].try_into().unwrap()),
            81
        );
        assert_eq!(
            u32::from_le_bytes(packet.data()[20..24].try_into().unwrap()),
            77
        );
        assert_eq!(
            i32::from_le_bytes(packet.data()[24..28].try_into().unwrap()),
            -12
        );

        let modern = msg.to_modern().expect("modern quest response");
        assert_eq!(modern.opcode(), Opcode::SMSG_QUEST_QUERY_RESPONSE);
        assert_eq!(&modern.data()[0..4], &123u32.to_le_bytes());
        assert_eq!(modern.data()[4], 0x80, "Allow");
    }

    #[test]
    fn test_smsg_quest_confirm_accept_with_sender_guid() {
        let msg = SmsgQuestConfirmAccept {
            quest_id: 42,
            title: "Shared Quest".to_string(),
            sender_guid: ObjectGuid::from_low(7),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_QUEST_CONFIRM_ACCEPT);
        assert_eq!(
            u32::from_le_bytes(packet.data()[0..4].try_into().unwrap()),
            42
        );
        // Title should be serialized after quest_id
        assert!(packet.data().len() > 4);
    }

    #[test]
    fn test_msg_quest_push_result() {
        let msg = MsgQuestPushResult {
            sender_guid: ObjectGuid::from_low(5),
            msg: 4, // QUEST_PARTY_MSG_TOO_FAR
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::MSG_QUEST_PUSH_RESULT);
    }
}

#[cfg(test)]
mod modern_quest_dialog_tests {
    use super::*;
    use crate::protocol::bitbuf::BitReader;

    fn details(
        choices: &[RewardItemInfo],
        items: &[RewardItemInfo],
        flags: QuestFlags,
    ) -> WorldPacket {
        SmsgQuestgiverQuestDetailsV2 {
            guid: ObjectGuid::new_creature(197, 42),
            quest_id: 5,
            title: "Kobold Camp Cleanup",
            details: "The kobolds have been troubling us.",
            objectives: "Slay 10 kobolds.",
            activate_accept: false,
            quest_flags: flags,
            reward_choices: choices,
            reward_items: items,
            money_reward: 120,
            rew_spell: 0,
            details_emote: [1, 0, 0, 0],
            details_emote_delay: [0, 0, 0, 0],
        }
        .to_modern()
        .expect("quest details must encode for modern")
    }

    /// The reward block is fixed-width: the client reads 4 items, 6 choices, 5 reputations and
    /// 4 currencies every time regardless of what the quest actually gives. If the length tracked
    /// the input, every field after the block would shift.
    #[test]
    fn the_reward_block_is_the_same_size_however_many_rewards_there_are() {
        let one = RewardItemInfo {
            item_id: 1,
            count: 1,
            display_id: 1,
        };
        let none = details(&[], &[], QuestFlags(0));
        let some = details(&[one.clone()], &[one.clone(), one.clone()], QuestFlags(0));

        assert_eq!(
            none.size(),
            some.size(),
            "rewards must be zero-padded, not length-prefixed"
        );
    }

    /// `HIDDEN_REWARDS` must blank the rewards on modern the same way it does on vanilla, or the
    /// client spoils quests that are meant to conceal their payout.
    #[test]
    fn hidden_rewards_are_withheld_but_the_block_still_has_its_shape() {
        let one = RewardItemInfo {
            item_id: 1234,
            count: 5,
            display_id: 1,
        };
        let shown = details(&[], &[one.clone()], QuestFlags(0));
        let hidden = details(&[], &[one], QuestFlags(QuestFlags::HIDDEN_REWARDS));

        assert_eq!(shown.size(), hidden.size(), "same shape either way");
        assert_ne!(
            shown.contents(),
            hidden.contents(),
            "hiding must actually change the payload"
        );
        // The item id must not survive anywhere in the hidden body.
        assert!(
            !hidden
                .contents()
                .windows(4)
                .any(|w| w == 1234u32.to_le_bytes()),
            "hidden reward item leaked into the packet"
        );
    }

    /// 1.14 moves every string to the end behind a bit-packed length. A body that does not grow by
    /// the text length means the strings were dropped or written in the wrong place.
    #[test]
    fn strings_trail_the_body() {
        let short = SmsgQuestgiverQuestDetailsV2 {
            guid: ObjectGuid::new_creature(197, 42),
            quest_id: 5,
            title: "a",
            details: "b",
            objectives: "c",
            activate_accept: false,
            quest_flags: QuestFlags(0),
            reward_choices: &[],
            reward_items: &[],
            money_reward: 0,
            rew_spell: 0,
            details_emote: [0; QUEST_EMOTE_COUNT],
            details_emote_delay: [0; QUEST_EMOTE_COUNT],
        }
        .to_modern()
        .unwrap();
        let long = details(&[], &[], QuestFlags(0));

        let extra = "Kobold Camp Cleanup".len()
            + "The kobolds have been troubling us.".len()
            + "Slay 10 kobolds.".len()
            - 3;
        // The long case also carries one emote (two u32s) that the short case does not.
        assert_eq!(long.size(), short.size() + extra + 8);
    }

    /// The turn-in dialog's `StatusFlags` word is what the client uses to enable its Complete button.
    ///
    /// Pinned to the literal `0xDF`/`0xDB` rather than just "the two differ" because the failure this
    /// guards against passed that weaker check: forwarding vanilla's `0x03`/`0x00` also produces two
    /// distinct packets, and the client answers it by drawing a turn-in frame with no completion text
    /// and no Complete button.
    #[test]
    fn request_items_reports_completability() {
        let build = |completable| {
            SmsgQuestgiverRequestItemsV2 {
                guid: ObjectGuid::new_creature(197, 42),
                quest_id: 5,
                title: "t",
                request_items_text: "r",
                complete_emote: 1,
                incomplete_emote: 2,
                completable,
                close_on_cancel: true,
                req_money: 0,
                req_items: &[],
            }
            .to_modern()
            .unwrap()
        };

        let status_flags = |completable| {
            let packet = build(completable);
            let mut reader = BitReader::new(packet.contents());
            let (high, low) = reader.read_packed_guid_128().unwrap();
            assert_eq!(ObjectGuid::from_guid128(high, low).entry(), 197);
            // QuestGiverCreatureID, then the nine words between it and StatusFlags.
            assert_eq!(reader.read_u32().unwrap(), 197);
            for _ in 0..9 {
                reader.read_u32().unwrap();
            }
            reader.read_u32().unwrap()
        };

        assert_eq!(status_flags(true), 0xDF);
        assert_eq!(status_flags(false), 0xDB);
        assert_eq!(build(true).size(), build(false).size());
    }
}
