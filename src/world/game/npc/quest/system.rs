//! Quest System - business logic for quest operations
//!
//! Handles quest giver status, quest validation, accept/complete, and packet sending.
//! Integrates with gossip system for quest menu display.

use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::shared::database::characters::repositories::QuestRepositoryTrait;
use crate::shared::messages::gossip::SmsgGossipComplete;
use crate::shared::database::characters::models::quest::{QuestStatusRewardedRow, QuestStatusRow};
use crate::shared::messages::quest::{
    QuestListItem, RequestItemInfo, RewardItemInfo, SmsgQuestgiverOfferRewardV2,
    SmsgQuestgiverQuestComplete, SmsgQuestgiverQuestDetailsV2, SmsgQuestgiverQuestListV2,
    SmsgQuestgiverRequestItemsV2, SmsgQuestgiverStatus, SmsgQuestlogFull,
    SmsgQuestupdateAddItem, SmsgQuestupdateAddKill, SmsgQuestupdateComplete,
    SmsgQuestupdateFailed, SmsgQuestupdateFailedtimer,
};
use crate::shared::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::common::update_fields::PLAYER_QUEST_LOG_1_1;
use crate::world::game::broadcast_mgr::{BroadcastManager, BroadcastManagerTrait};
use crate::world::game::creature::CreatureManager;
use crate::world::game::inventory::InventorySystem;
use crate::world::game::items::ItemManager;
use crate::world::game::player::experience::ExperienceSystem;
use crate::world::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::world::game::player::PlayerManager;
use crate::world::World;

use super::manager::QuestManager;
use super::types::{
    DialogStatus, QuestProgress, QuestSpecialFlags, QuestStatus, QuestTemplate,
    MAX_QUEST_LOG_SIZE, QUEST_OBJECTIVES_COUNT,
};

/// Quest system - handles business logic and packet sending
pub struct QuestSystem {
    pub manager: Arc<QuestManager>,
    repository: Arc<dyn QuestRepositoryTrait>,
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,
    creature_mgr: Arc<CreatureManager>,
    item_mgr: Arc<ItemManager>,
    inventory: Arc<InventorySystem>,
    experience: Arc<ExperienceSystem>,
}

impl QuestSystem {
    /// Create a new quest system
    pub fn new(
        manager: Arc<QuestManager>,
        repository: Arc<dyn QuestRepositoryTrait>,
        broadcast_mgr: Arc<BroadcastManager>,
        player_mgr: Arc<PlayerManager>,
        creature_mgr: Arc<CreatureManager>,
        item_mgr: Arc<ItemManager>,
        inventory: Arc<InventorySystem>,
        experience: Arc<ExperienceSystem>,
    ) -> Self {
        Self {
            manager,
            repository,
            broadcast_mgr,
            player_mgr,
            creature_mgr,
            item_mgr,
            inventory,
            experience,
        }
    }

    /// Calculate quest giver status for player (shown as icon above NPC)
    pub fn get_quest_giver_status(
        &self,
        entry: u32,
        player_guid: ObjectGuid,
        world: &World,
    ) -> DialogStatus {
        let start_quests = self.manager.get_creature_quest_relations(entry);
        let finish_quests = self.manager.get_creature_involved_relations(entry);

        let mut status = DialogStatus::None;

        // Get player quest info
        let (active_quests, rewarded_quests) = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| {
                let active: HashSet<u32> = p.active_quests.iter().map(|q| q.quest_id).collect();
                let rewarded: HashSet<u32> = p.rewarded_quests.iter().copied().collect();
                (active, rewarded)
            })
            .unwrap_or_default();

        // Priority 1: Check completable quests (higher priority)
        for &quest_id in &finish_quests {
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };

            if !quest.is_active {
                continue;
            }

            let quest_status = self.get_quest_status(player_guid, quest_id);
            let is_rewarded = rewarded_quests.contains(&quest_id);

            // Auto-complete quests can be turned in immediately
            let can_turn_in = if quest.is_auto_complete() {
                quest_status == QuestStatus::Incomplete
                    || self.can_take_quest(player_guid, &quest, world)
            } else {
                quest_status == QuestStatus::Complete && !is_rewarded
            };

            if can_turn_in {
                if quest.is_repeatable() && quest.is_auto_complete() {
                    status = status.max(DialogStatus::RewardRep);
                } else {
                    return DialogStatus::Reward2;
                }
            } else if quest_status == QuestStatus::Incomplete {
                status = status.max(DialogStatus::Incomplete);
            }
        }

        // Priority 2: Check available quests
        for &quest_id in &start_quests {
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };

            if !quest.is_active {
                continue;
            }

            // Skip if already rewarded (non-repeatable)
            if rewarded_quests.contains(&quest_id) && !quest.is_repeatable() {
                continue;
            }

            // Skip if already active
            if active_quests.contains(&quest_id) {
                continue;
            }

            // Check if can take
            if self.can_take_quest(player_guid, &quest, world) {
                status = status.max(DialogStatus::Available);
            } else {
                status = status.max(DialogStatus::Unavailable);
            }
        }

        status
    }

    /// Validate if player can take quest (12 validation checks)
    pub fn can_take_quest(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        _world: &World,
    ) -> bool {
        let Some(player) = self.player_mgr.get_player(player_guid) else {
            return false;
        };

        let active: HashSet<u32> = player.active_quests.iter().map(|q| q.quest_id).collect();
        let rewarded: HashSet<u32> = player.rewarded_quests.iter().copied().collect();

        // 1. Already rewarded (non-repeatable)
        if rewarded.contains(&quest.id) && !quest.is_repeatable() {
            return false;
        }

        // 2. Level requirements
        if player.level < quest.min_level as u8 {
            return false;
        }
        if quest.max_level > 0 && player.level > quest.max_level as u8 {
            return false;
        }

        // 3. Class requirement
        if quest.required_classes != 0 {
            let class_mask = 1 << (player.class - 1);
            if (quest.required_classes & class_mask) == 0 {
                return false;
            }
        }

        // 4. Race requirement
        if quest.required_races != 0 {
            let race_mask = 1 << (player.race - 1);
            if (quest.required_races & race_mask) == 0 {
                return false;
            }
        }

        // 5. Skill requirement
        if quest.required_skill != 0 {
            let has_skill = player.skills.skills.iter().any(|(id, skill_data)| {
                *id == quest.required_skill as u16 && (skill_data.current_value as u32) >= quest.required_skill_value
            });
            if !has_skill {
                return false;
            }
        }

        // 6. Previous quest requirement
        if quest.prev_quest_id > 0 {
            if !rewarded.contains(&(quest.prev_quest_id as u32)) {
                return false;
            }
        } else if quest.prev_quest_id < 0 {
            if rewarded.contains(&((-quest.prev_quest_id) as u32)) {
                return false;
            }
        }

        // 7. Exclusive group
        if quest.exclusive_group != 0 {
            for active_quest_id in &active {
                if let Some(active_quest) = self.manager.get_quest_template(*active_quest_id) {
                    if active_quest.exclusive_group == quest.exclusive_group {
                        return false;
                    }
                }
            }
        }

        // 8. Breadcrumb check
        if quest.breadcrumb_for_quest_id > 0 {
            let breadcrumb_target = quest.breadcrumb_for_quest_id as u32;
            if active.contains(&breadcrumb_target) || rewarded.contains(&breadcrumb_target) {
                return false;
            }
        }

        // 9. Reputation requirement (min)
        // TODO: Implement reputation requirement check using ReputationSystem
        // For now, skip this check as it requires looking up faction_id -> rep_list_id mapping
        let _ = quest.required_min_rep_faction;
        let _ = quest.required_min_rep_value;

        // 10. Reputation requirement (max)
        // TODO: Implement reputation requirement check using ReputationSystem
        let _ = quest.required_max_rep_faction;
        let _ = quest.required_max_rep_value;

        // 11. Timed quest check
        if quest.special_flags.contains(QuestSpecialFlags::TIMED) {
            for active_quest_id in &active {
                if let Some(active_quest) = self.manager.get_quest_template(*active_quest_id) {
                    if active_quest
                        .special_flags
                        .contains(QuestSpecialFlags::TIMED)
                    {
                        return false;
                    }
                }
            }
        }

        // 12. Required condition check
        // TODO: Implement condition system integration when ConditionSystem is available
        // For now, skip condition check if required_condition > 0 (allow quest to be taken)
        // This should be replaced with actual condition validation:
        // if quest.required_condition > 0 {
        //     if !world.managers.condition_mgr.is_condition_satisfied(
        //         quest.required_condition,
        //         player_guid,
        //         world,
        //     ) {
        //         return false;
        //     }
        // }

        true
    }

    /// Get quest status for player
    pub fn get_quest_status(&self, player_guid: ObjectGuid, quest_id: u32) -> QuestStatus {
        self.player_mgr
            .get_player(player_guid)
            .map(|p| {
                if let Some(progress) = p.active_quests.iter().find(|q| q.quest_id == quest_id) {
                    if let Some(template) = self.manager.get_quest_template(quest_id) {
                        if progress.is_complete(&template) {
                            QuestStatus::Complete
                        } else {
                            QuestStatus::Incomplete
                        }
                    } else {
                        QuestStatus::Incomplete
                    }
                } else if p.rewarded_quests.contains(&quest_id) {
                    QuestStatus::Complete
                } else {
                    QuestStatus::None
                }
            })
            .unwrap_or(QuestStatus::None)
    }

    /// Send quest giver status packet
    pub fn send_quest_giver_status(
        &self,
        player_guid: ObjectGuid,
        creature_guid: ObjectGuid,
        world: &World,
    ) {
        let Some(entry) = self
            .creature_mgr
            .get_creature(creature_guid)
            .map(|c| c.entry)
        else {
            return;
        };

        let local_status = self.get_quest_giver_status(entry, player_guid, world);

        // Convert local DialogStatus to message DialogStatus
        let status = match local_status {
            DialogStatus::None => crate::shared::messages::quest::DialogStatus::None,
            DialogStatus::Unavailable => crate::shared::messages::quest::DialogStatus::Unavailable,
            DialogStatus::Chat => crate::shared::messages::quest::DialogStatus::Chat,
            DialogStatus::Incomplete => crate::shared::messages::quest::DialogStatus::Incomplete,
            DialogStatus::RewardRep => crate::shared::messages::quest::DialogStatus::RewardRep,
            DialogStatus::Available => crate::shared::messages::quest::DialogStatus::Available,
            DialogStatus::RewardOld => crate::shared::messages::quest::DialogStatus::RewardOld,
            DialogStatus::Reward2 => crate::shared::messages::quest::DialogStatus::Reward2,
        };

        let msg = SmsgQuestgiverStatus {
            guid: creature_guid,
            status,
        };

        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;
    }

    /// Prepare quest menu items for gossip integration
    pub fn prepare_quest_menu(
        &self,
        player_guid: ObjectGuid,
        entry: u32,
        world: &World,
    ) -> Vec<super::types::GossipQuestData> {
        let start_quests = self.manager.get_creature_quest_relations(entry);
        let finish_quests = self.manager.get_creature_involved_relations(entry);

        let mut quest_items = Vec::new();
        let mut seen = HashSet::new();

        // Add completable quests first
        for &quest_id in &finish_quests {
            if !seen.insert(quest_id) {
                continue;
            }
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };

            let status = self.get_quest_status(player_guid, quest_id);

            if status == QuestStatus::Complete {
                // Yellow ? for complete (ready to turn in)
                quest_items.push(super::types::GossipQuestData {
                    quest_id,
                    icon: DialogStatus::Reward2 as u32,
                    level: quest.quest_level,
                    title: quest.title.clone(),
                });
            } else if status == QuestStatus::Incomplete {
                // Gray ? for incomplete (not ready yet)
                quest_items.push(super::types::GossipQuestData {
                    quest_id,
                    icon: DialogStatus::Incomplete as u32,
                    level: quest.quest_level,
                    title: quest.title.clone(),
                });
            }
        }

        // Add available quests (only those the player can actually take)
        for &quest_id in &start_quests {
            if !seen.insert(quest_id) {
                continue;
            }
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };

            let status = self.get_quest_status(player_guid, quest_id);

            // Only show quests that are not started AND can be taken (validates prerequisites)
            if status == QuestStatus::None && self.can_take_quest(player_guid, &quest, world) {
                let icon = if quest.is_auto_complete() || quest.is_repeatable() {
                    DialogStatus::RewardRep as u32
                } else {
                    DialogStatus::Available as u32
                };
                quest_items.push(super::types::GossipQuestData {
                    quest_id,
                    icon,
                    level: quest.quest_level,
                    title: quest.title.clone(),
                });
            }
        }

        quest_items
    }

    /// Send quest details when player clicks quest in gossip menu
    pub fn send_quest_details(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        _world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            warn!("Quest {} not found", quest_id);
            return Ok(());
        };

        // Build reward choices with display IDs
        let reward_choices: Vec<RewardItemInfo> = quest
            .rew_choice_item_id
            .iter()
            .zip(quest.rew_choice_item_count.iter())
            .filter(|(id, _)| **id != 0)
            .map(|(id, count)| {
                let display_id = self
                    .item_mgr
                    .get_template(*id)
                    .map(|t| t.display_id)
                    .unwrap_or(0);
                RewardItemInfo {
                    item_id: *id,
                    count: *count,
                    display_id,
                }
            })
            .collect();

        // Build reward items with display IDs
        let reward_items: Vec<RewardItemInfo> = quest
            .rew_item_id
            .iter()
            .zip(quest.rew_item_count.iter())
            .filter(|(id, _)| **id != 0)
            .map(|(id, count)| {
                let display_id = self
                    .item_mgr
                    .get_template(*id)
                    .map(|t| t.display_id)
                    .unwrap_or(0);
                RewardItemInfo {
                    item_id: *id,
                    count: *count,
                    display_id,
                }
            })
            .collect();

        let msg = SmsgQuestgiverQuestDetailsV2 {
            guid: quest_giver_guid,
            quest_id: quest.id,
            title: &quest.title,
            details: &quest.details,
            objectives: &quest.objectives,
            activate_accept: false,
            quest_flags: crate::shared::messages::quest::QuestFlags(quest.quest_flags.bits()),
            reward_choices: &reward_choices,
            reward_items: &reward_items,
            money_reward: quest.rew_or_req_money.max(0) as u32,
            rew_spell: quest.rew_spell,
            details_emote: quest.details_emote,
            details_emote_delay: quest.details_emote_delay,
        };

        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;
        Ok(())
    }

    /// Handle quest accept
    pub async fn handle_quest_accept(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            warn!("Cannot accept quest {}: not found", quest_id);
            return Ok(());
        };

        // Validate that the NPC is a valid quest starter for this quest
        let Some(entry) = self
            .creature_mgr
            .get_creature(quest_giver_guid)
            .map(|c| c.entry)
        else {
            warn!("Quest giver {:?} not found", quest_giver_guid);
            return Ok(());
        };

        let start_quests = self.manager.get_creature_quest_relations(entry);
        if !start_quests.contains(&quest_id) {
            warn!(
                "Player {:?} tried to accept quest {} from NPC {:?} who is not a quest starter for this quest",
                player_guid, quest_id, quest_giver_guid
            );
            return Ok(());
        }

        // Validate can take quest
        if !self.can_take_quest(player_guid, &quest, world) {
            warn!("Player {:?} cannot accept quest {}", player_guid, quest_id);
            return Ok(());
        }

        // Check quest log size
        let can_add = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.active_quests.len() < MAX_QUEST_LOG_SIZE)
            .unwrap_or(false);

        if !can_add {
            let msg = SmsgQuestlogFull;
            self.broadcast_mgr
                .send_msg_to_player(player_guid, msg)
                ;
            return Ok(());
        }

        // Add quest to player and get the slot index
        let Some(slot) = self.player_mgr.with_player_mut(player_guid, |p| {
            let slot = p.active_quests.len();
            p.active_quests.push(QuestProgress::new(quest_id));
            slot
        }) else {
            warn!("Player {:?} not found when accepting quest {}", player_guid, quest_id);
            return Ok(());
        };

        // Update PLAYER_QUEST_LOG_* update fields so the client shows the quest
        // Each quest slot uses 3 fields: QUEST_ID, COUNT_STATE, TIMER
        const MAX_QUEST_OFFSET: u32 = 3;
        const QUEST_ID_OFFSET: u32 = 0;
        const QUEST_COUNT_STATE_OFFSET: u32 = 1;
        const QUEST_TIME_OFFSET: u32 = 2;

        let slot_u32 = slot as u32;
        let quest_id_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_ID_OFFSET;
        let count_state_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_COUNT_STATE_OFFSET;
        let timer_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_TIME_OFFSET;

        // Convert world ObjectGuid to world::common ObjectGuid for the message
        let world_guid = ObjectGuid::from_low(player_guid.counter());

        let values_update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                .set_field(quest_id_field, quest_id)
                .set_field(count_state_field, 0)  // Initialize count/state to 0
                .set_field(timer_field, 0),       // Set timer to 0
        ));

        self.broadcast_mgr
            .send_msg_to_player(player_guid, values_update)
            ;

        // Send gossip complete to close quest window
        let msg = SmsgGossipComplete;
        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;

        // Fire OnQuestAccept Lua callback if a gossip script is registered for this NPC
        if let Some(script) = world.managers.lua_mgr.get_gossip_script(entry) {
            let player_snap = build_player_snapshot(player_guid, world);
            let actions = world.managers.lua_mgr.with_lua(|lua| {
                script.on_quest_accept(lua, &player_snap, quest_giver_guid, quest_id)
            });
            if !actions.is_empty() {
                execute_gossip_actions(actions, player_guid, quest_giver_guid, world).await?;
            }
        }

        info!(
            "Player {:?} accepted quest {} from {:?}",
            player_guid, quest_id, quest_giver_guid
        );
        Ok(())
    }

    /// Handle quest complete request
    ///
    /// Called when the Vanilla client sends CMSG_QUESTGIVER_COMPLETE_QUEST.
    /// This is sent when clicking any active/incomplete/complete quest in the quest list.
    /// - Incomplete quests: send SMSG_QUESTGIVER_REQUEST_ITEMS with completable=false
    /// - Complete quests: send SMSG_QUESTGIVER_OFFER_REWARD
    pub async fn handle_quest_complete(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        _world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            warn!("Cannot complete quest {}: not found", quest_id);
            return Ok(());
        };

        // Validate quest giver can complete this quest
        let Some(entry) = self
            .creature_mgr
            .get_creature(quest_giver_guid)
            .map(|c| c.entry)
        else {
            warn!("Quest giver {:?} not found", quest_giver_guid);
            return Ok(());
        };

        // Validate that the NPC is involved in this quest - check both starter and ender relations
        let finish_quests = self.manager.get_creature_involved_relations(entry);
        let start_quests = self.manager.get_creature_quest_relations(entry);

        if !finish_quests.contains(&quest_id) && !start_quests.contains(&quest_id) {
            warn!(
                "Player {:?} tried to complete quest {} from NPC {:?} who is not involved in this quest",
                player_guid, quest_id, quest_giver_guid
            );
            return Ok(());
        }

        // Check quest completion status
        let is_complete = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| {
                if let Some(progress) = p.active_quests.iter().find(|q| q.quest_id == quest_id) {
                    progress.is_complete(&quest)
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if !is_complete && !quest.is_auto_complete() {
            // Quest not complete - send request items dialog showing what's still needed
            let req_items: Vec<RequestItemInfo> = quest
                .req_item_id
                .iter()
                .zip(quest.req_item_count.iter())
                .filter(|(id, _)| **id != 0)
                .map(|(id, count)| {
                    let display_id = self
                        .item_mgr
                        .get_template(*id)
                        .map(|t| t.display_id)
                        .unwrap_or(0);
                    RequestItemInfo {
                        item_id: *id,
                        count: *count,
                        display_id,
                    }
                })
                .collect();

            let msg = SmsgQuestgiverRequestItemsV2 {
                guid: quest_giver_guid,
                quest_id: quest.id,
                title: &quest.title,
                request_items_text: &quest.request_items_text,
                complete_emote: quest.complete_emote,
                incomplete_emote: quest.incomplete_emote,
                completable: false,
                close_on_cancel: false,
                req_money: if quest.rew_or_req_money < 0 {
                    (-quest.rew_or_req_money) as u32
                } else {
                    0
                },
                req_items: &req_items,
            };
            self.broadcast_mgr
                .send_msg_to_player(player_guid, msg)
                ;
            return Ok(());
        }

        // Quest is complete (or auto-complete) - send offer reward dialog
        self.send_offer_reward(player_guid, quest_giver_guid, &quest)
            ;

        Ok(())
    }

    /// Send SMSG_QUESTGIVER_OFFER_REWARD to show the reward selection dialog
    fn send_offer_reward(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest: &QuestTemplate,
    ) {
        // Build reward choices with display IDs
        let reward_choices: Vec<RewardItemInfo> = quest
            .rew_choice_item_id
            .iter()
            .zip(quest.rew_choice_item_count.iter())
            .filter(|(id, _)| **id != 0)
            .map(|(id, count)| {
                let display_id = self
                    .item_mgr
                    .get_template(*id)
                    .map(|t| t.display_id)
                    .unwrap_or(0);
                RewardItemInfo {
                    item_id: *id,
                    count: *count,
                    display_id,
                }
            })
            .collect();

        // Build fixed reward items with display IDs
        let reward_items: Vec<RewardItemInfo> = quest
            .rew_item_id
            .iter()
            .zip(quest.rew_item_count.iter())
            .filter(|(id, _)| **id != 0)
            .map(|(id, count)| {
                let display_id = self
                    .item_mgr
                    .get_template(*id)
                    .map(|t| t.display_id)
                    .unwrap_or(0);
                RewardItemInfo {
                    item_id: *id,
                    count: *count,
                    display_id,
                }
            })
            .collect();

        let msg = SmsgQuestgiverOfferRewardV2 {
            guid: quest_giver_guid,
            quest_id: quest.id,
            title: &quest.title,
            offer_reward_text: &quest.offer_reward_text,
            enable_next: quest.is_auto_complete(),
            reward_choices: &reward_choices,
            reward_items: &reward_items,
            money_reward: quest.rew_or_req_money.max(0) as u32,
            rew_spell: quest.rew_spell,
            rew_spell_cast: quest.rew_spell_cast,
            offer_reward_emote: quest.offer_reward_emote,
            offer_reward_emote_delay: quest.offer_reward_emote_delay,
        };

        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;
    }

    /// Handle quest reward selection
    pub async fn handle_quest_reward(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        reward_choice: u32,
        world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            return Ok(());
        };

        // Validate quest is complete
        let is_complete = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| {
                if let Some(progress) = p.active_quests.iter().find(|q| q.quest_id == quest_id) {
                    progress.is_complete(&quest)
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if !is_complete && !quest.is_auto_complete() {
            return Ok(());
        }

        // Get player level for XP calculation
        let player_level = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.level)
            .unwrap_or(1);

        // 1. Remove required items from inventory
        for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
            if quest.req_item_id[i] != 0 && quest.req_item_count[i] > 0 {
                let item_id = quest.req_item_id[i];
                let count = quest.req_item_count[i];

                // Find and remove items from inventory
                let items_to_remove = self.inventory.find_items_by_entry(player_guid, item_id);
                let mut remaining = count;

                for item_guid in items_to_remove {
                    if remaining == 0 {
                        break;
                    }

                    // Remove items - the inventory system will handle counting
                    let remove_result = self
                        .inventory
                        .remove_item(player_guid, item_guid, remaining);
                    match remove_result {
                        crate::world::game::inventory::RemoveItemResult::ItemRemoved {
                            ..
                        } => {
                            // Get the count that was removed by checking before removal
                            remaining = 0;
                        }
                        crate::world::game::inventory::RemoveItemResult::CountReduced {
                            new_count,
                            ..
                        } => {
                            // Partial removal - calculate how many were removed
                            let removed = count - new_count;
                            remaining = remaining.saturating_sub(removed);
                        }
                        crate::world::game::inventory::RemoveItemResult::InsufficientCount => {
                            // Not enough items, skip
                            warn!(
                                "Insufficient count for item {} when completing quest",
                                item_id
                            );
                        }
                        _ => {
                            warn!(
                                "Failed to remove item {} from player {:?} for quest completion",
                                item_id, player_guid
                            );
                        }
                    }
                }
            }
        }

        // 2. Give XP reward
        let xp_reward = if quest.rew_xp > 0 {
            quest.rew_xp
        } else {
            // Calculate XP based on player level and quest level
            // Formula: Base XP * (quest level / player level)
            let base_xp = crate::world::game::player::experience::calculate_xp_for_level(
                quest.quest_level as u8,
            );
            let level_diff_factor = if player_level > quest.quest_level as u8 {
                let diff = player_level - quest.quest_level as u8;
                if diff >= 5 {
                    0 // Gray quest, no XP
                } else {
                    100 - (diff * 20) // -20% per level above quest level
                }
            } else {
                100
            };
            (base_xp * level_diff_factor as u32) / 100
        };

        if xp_reward > 0 {
            use crate::shared::game::experience::XpSource;
            let _ = self
                .experience
                .add_xp(player_guid, xp_reward, XpSource::Quest, None, 0.0)
                ;
        }

        // 3. Give money reward (if positive)
        if quest.rew_or_req_money > 0 {
            let money = quest.rew_or_req_money as u32;
            self.inventory.add_gold(player_guid, money);
        }

        // 4. Give reward items (choice + fixed)
        // Give the chosen reward item
        let choice_index = reward_choice as usize;
        if choice_index > 0 && choice_index <= super::types::QUEST_REWARD_CHOICES_COUNT {
            let idx = choice_index - 1; // Convert to 0-based
            if quest.rew_choice_item_id[idx] != 0 && quest.rew_choice_item_count[idx] > 0 {
                self.inventory
                    .add_item(
                        player_guid,
                        quest.rew_choice_item_id[idx],
                        quest.rew_choice_item_count[idx],
                    )
                    ;
            }
        }

        // Give fixed reward items
        for i in 0..super::types::QUEST_REWARDS_COUNT {
            if quest.rew_item_id[i] != 0 && quest.rew_item_count[i] > 0 {
                self.inventory
                    .add_item(player_guid, quest.rew_item_id[i], quest.rew_item_count[i])
                    ;
            }
        }

        // 5. Give reputation rewards
        for i in 0..super::types::QUEST_REPUTATIONS_COUNT {
            if quest.rew_rep_faction[i] != 0 && quest.rew_rep_value[i] != 0 {
                let faction_id = quest.rew_rep_faction[i];
                let rep_value = quest.rew_rep_value[i];
                if let Err(e) = world.systems.reputation.modify_reputation(
                    player_guid,
                    faction_id,
                    rep_value,
                    world,
                ) {
                    warn!("[QUEST] Failed to grant rep for faction {} on quest {}: {}", faction_id, quest_id, e);
                }
            }
        }

        // 6. Cast reward spell if any
        // TODO: Implement spell casting when SpellSystem integration is available
        // if quest.rew_spell > 0 {
        //     world.systems.spell.cast_spell(player_guid, quest.rew_spell);
        // }

        // Remove from active quests
        self.player_mgr.with_player_mut(player_guid, |p| {
            p.active_quests.retain(|q| q.quest_id != quest_id);
            p.rewarded_quests.insert(quest_id);
        });

        // Send completion packet
        let msg = SmsgQuestupdateComplete { quest_id };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;

        // Send gossip complete to close quest window
        let gossip_complete = SmsgGossipComplete;
        self.broadcast_mgr
            .send_msg_to_player(player_guid, gossip_complete)
            ;

        // Send quest complete packet with XP info
        let complete_msg = SmsgQuestgiverQuestComplete {
            quest_id,
            xp: xp_reward,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, complete_msg)
            ;

        info!(
            "Player {:?} completed quest {} with reward choice {} (XP: {}, Money: {})",
            player_guid,
            quest_id,
            reward_choice,
            xp_reward,
            quest.rew_or_req_money.max(0)
        );

        // Check for follow-up quest in chain
        if quest.next_quest_in_chain != 0 {
            let next_quest_id = quest.next_quest_in_chain;
            
            // Get NPC entry to check if they can give the next quest
            if let Some(entry) = self.creature_mgr.get_creature(quest_giver_guid).map(|c| c.entry) {
                // Check if this NPC can start the next quest
                let start_quests = self.manager.get_creature_quest_relations(entry);
                
                if start_quests.contains(&next_quest_id) {
                    // Check if the player can take the next quest
                    if let Some(next_quest) = self.manager.get_quest_template(next_quest_id) {
                        if self.can_take_quest(player_guid, &next_quest, world) {
                            info!(
                                "Showing follow-up quest {} to player {:?} from NPC {:?}",
                                next_quest_id, player_guid, quest_giver_guid
                            );
                            
                            // Send the follow-up quest details
                            self.send_quest_details(player_guid, quest_giver_guid, next_quest_id, world)?;
                        } else {
                            info!(
                                "Player {:?} cannot take follow-up quest {} (prerequisites not met)",
                                player_guid, next_quest_id
                            );
                        }
                    }
                } else {
                    info!(
                        "NPC {:?} cannot give follow-up quest {} (not in quest relations)",
                        quest_giver_guid, next_quest_id
                    );
                }
            }
        }

        // Fire OnQuestRewarded Lua callback if a gossip script is registered for this NPC
        let npc_entry = self.creature_mgr.get_creature(quest_giver_guid).map(|c| c.entry).unwrap_or(0);
        if npc_entry > 0 {
            if let Some(script) = world.managers.lua_mgr.get_gossip_script(npc_entry) {
                let player_snap = build_player_snapshot(player_guid, world);
                let actions = world.managers.lua_mgr.with_lua(|lua| {
                    script.on_quest_rewarded(lua, &player_snap, quest_giver_guid, quest_id)
                });
                if !actions.is_empty() {
                    execute_gossip_actions(actions, player_guid, quest_giver_guid, world).await?;
                }
            }
        }

        // Fire OnQuestRewarded Lua callback for GO quest givers
        let go_entry = world.managers.gameobject_mgr
            .with_gameobject(quest_giver_guid, |go| go.entry)
            .unwrap_or(0);
        if go_entry > 0 {
            if let Some(script) = world.managers.lua_mgr.get_game_object_script(go_entry) {
                let player_snap = build_player_snapshot(player_guid, world);
                let actions = world.managers.lua_mgr.with_lua(|lua| {
                    script.on_quest_rewarded(lua, &player_snap, quest_giver_guid, quest_id)
                });
                if !actions.is_empty() {
                    execute_gossip_actions(actions, player_guid, quest_giver_guid, world).await?;
                }
            }
        }

        Ok(())
    }

    /// Send quest giver quest list
    pub fn send_quest_giver_quest_list(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        entry: u32,
        world: &World,
    ) -> Result<()> {
        let quest_items = self.prepare_quest_menu(player_guid, entry, world);

        let quests: Vec<QuestListItem> = quest_items
            .iter()
            .map(|q| QuestListItem {
                quest_id: q.quest_id,
                icon: q.icon,
                level: q.level,
                title: q.title.clone(),
            })
            .collect();

        let msg = SmsgQuestgiverQuestListV2 {
            guid: quest_giver_guid,
            title: "",
            emote_delay: 0,
            emote: 0,
            quests: &quests,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, msg)
            ;

        Ok(())
    }

    /// Initialize the quest system
    pub async fn init(&self) -> Result<()> {

        Ok(())
    }

    /// Shutdown the quest system
    pub async fn shutdown(&self) -> Result<()> {

        Ok(())
    }

    /// Load quest state from DB rows into the player on login.
    ///
    /// Populates `active_quests` and `rewarded_quests` from previously-saved DB rows,
    /// then sends PLAYER_QUEST_LOG_* update fields so the quest log UI shows correctly.
    pub fn load_from_db(
        &self,
        player_guid: ObjectGuid,
        active_rows: Vec<QuestStatusRow>,
        rewarded_rows: Vec<QuestStatusRewardedRow>,
    ) {
        use super::types::QuestUpdateState;

        // Map DB rows → QuestProgress, then insert into player
        let mut active_quests: Vec<super::types::QuestProgress> = active_rows
            .into_iter()
            .map(|row| {
                let status = match row.status {
                    0 => super::types::QuestStatus::None,
                    1 => super::types::QuestStatus::Complete,
                    2 => super::types::QuestStatus::Unavailable,
                    3 => super::types::QuestStatus::Incomplete,
                    4 => super::types::QuestStatus::Available,
                    5 => super::types::QuestStatus::Failed,
                    _ => super::types::QuestStatus::Incomplete,
                };
                super::types::QuestProgress {
                    quest_id: row.quest,
                    status,
                    rewarded: row.rewarded,
                    explored: row.explored,
                    timer: row.timer,
                    reward_choice: 0,
                    creature_or_go_count: [
                        row.mob_count1,
                        row.mob_count2,
                        row.mob_count3,
                        row.mob_count4,
                    ],
                    item_count: [
                        row.item_count1,
                        row.item_count2,
                        row.item_count3,
                        row.item_count4,
                    ],
                    update_state: QuestUpdateState::Unchanged,
                }
            })
            .collect();

        let rewarded_set: std::collections::HashSet<u32> =
            rewarded_rows.into_iter().map(|r| r.quest).collect();

        // Restore into player state
        self.player_mgr.with_player_mut(player_guid, |player| {
            player.active_quests = active_quests.clone();
            player.rewarded_quests = rewarded_set.clone();
        });

        info!(
            "[QUEST] Loaded {} active quests and {} rewarded quests for player {:?}",
            active_quests.len(),
            rewarded_set.len(),
            player_guid
        );
        // Quest log fields are now included in the player's CREATE_OBJECT2 packet
        // (build_player_create_block_for_player), so no separate values update is needed here.
    }

    /// Cleanup quest state on logout.
    ///
    /// Quest saving is handled by `save_player_quests` which is called separately
    /// during logout cleanup. This method handles any non-save logout logic.
    pub async fn on_player_logout(&self, player_guid: ObjectGuid) -> Result<()> {
        // Quest saving is done by save_player_quests() in perform_logout_cleanup.
        // Nothing else to clean up for quests currently.
        Ok(())
    }

    /// Check if a player has an active quest that requires the given item.
    ///
    /// Used by the loot system to filter quest-only drops.
    pub fn player_has_quest_for_item(&self, player_guid: ObjectGuid, item_id: u32) -> bool {
        use super::types::QUEST_ITEM_OBJECTIVES_COUNT;

        let active_quest_ids: Vec<u32> = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.active_quests.iter().map(|q| q.quest_id).collect())
            .unwrap_or_default();

        for quest_id in active_quest_ids {
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };
            for i in 0..QUEST_ITEM_OBJECTIVES_COUNT {
                if quest.req_item_id[i] == item_id {
                    return true;
                }
            }
        }

        false
    }

    /// Handle item added to inventory — check quest item objectives.
    ///
    /// Called after any item enters the player's inventory (loot, buy, etc.).
    /// Mirrors MaNGOS `Player::ItemAddedQuestCheck`.
    pub fn handle_item_added(&self, player_guid: ObjectGuid, item_id: u32, count: u32) {
        use super::types::{QuestSpecialFlags, QUEST_ITEM_OBJECTIVES_COUNT};

        let active_quests: Vec<u32> = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.active_quests.iter().map(|q| q.quest_id).collect())
            .unwrap_or_default();

        for quest_id in active_quests {
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };

            // Only process quests with item delivery objectives
            if !quest.special_flags.contains(QuestSpecialFlags::DELIVER) {
                continue;
            }

            for i in 0..QUEST_ITEM_OBJECTIVES_COUNT {
                if quest.req_item_id[i] != item_id || quest.req_item_count[i] == 0 {
                    continue;
                }

                let update_result = self.player_mgr.with_player_mut(player_guid, |p| {
                    if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) {
                        let current = progress.item_count[i];
                        let required = quest.req_item_count[i];

                        if current < required {
                            let new_count = (current + count).min(required);
                            progress.item_count[i] = new_count;
                            progress.mark_changed();
                            let is_complete = progress.is_complete(&quest);
                            Some((new_count, required, is_complete))
                        } else {
                            None // Already at max
                        }
                    } else {
                        None
                    }
                }).flatten();

                let Some((new_count, req_count, is_complete)) = update_result else {
                    continue;
                };

                // Send SMSG_QUESTUPDATE_ADD_ITEM
                let msg = SmsgQuestupdateAddItem {
                    item_id,
                    count: new_count,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                info!(
                    "[QUEST] Item credit: player {:?} quest {} item {} — {}/{}",
                    player_guid, quest_id, item_id, new_count, req_count
                );

                if is_complete {
                    let complete_msg = SmsgQuestupdateComplete { quest_id };
                    self.broadcast_mgr.send_msg_to_player(player_guid, complete_msg);
                    info!("[QUEST] Quest {} complete for player {:?}", quest_id, player_guid);
                }

                break; // Only one slot per quest per item
            }
        }
    }

    /// Tick quest timers — expire timed quests.
    ///
    /// Called every world update tick. Decrements timers for timed quests
    /// and marks them failed when the timer reaches zero.
    pub fn update_quest_timers(&self, diff_ms: u32, world: &World) {
        use super::types::QuestSpecialFlags;

        let online_players: Vec<ObjectGuid> = world
            .session_mgr
            .get_all_sessions()
            .into_iter()
            .collect();

        for player_guid in online_players {
            // Collect quests with active timers
            let timed_quests: Vec<(u32, u32)> = self
                .player_mgr
                .get_player(player_guid)
                .map(|p| {
                    p.active_quests
                        .iter()
                        .filter_map(|q| {
                            if q.timer > 0 {
                                Some((q.quest_id, q.timer))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            for (quest_id, timer) in timed_quests {
                let new_timer = timer.saturating_sub(diff_ms);

                // Update timer and check for expiry
                let expired = self.player_mgr.with_player_mut(player_guid, |p| {
                    if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) {
                        progress.timer = new_timer;
                        progress.mark_changed();
                        new_timer == 0
                    } else {
                        false
                    }
                }).unwrap_or(false);

                if expired {
                    // Mark quest as failed
                    self.player_mgr.with_player_mut(player_guid, |p| {
                        if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) {
                            progress.status = super::types::QuestStatus::Failed;
                            progress.mark_changed();
                        }
                    });

                    // Send failure packets
                    let msg = SmsgQuestupdateFailed { quest_id };
                    self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                    let timer_msg = SmsgQuestupdateFailedtimer { quest_id };
                    self.broadcast_mgr.send_msg_to_player(player_guid, timer_msg);

                    info!("[QUEST] Quest {} timed out for player {:?}", quest_id, player_guid);
                }
            }
        }
    }

    /// Abandon a quest
    ///
    /// Removes the quest from the player's active quests, deletes from database,
    /// and sends update to clear the quest from the client's quest log UI.
    pub async fn abandon_quest(
        &self,
        player_guid: ObjectGuid,
        quest_id: u32,
    ) -> Result<Option<usize>> {
        // Find the quest and its slot
        let slot_opt: Option<Option<usize>> = self.player_mgr.with_player_mut(player_guid, |p| {
            p.active_quests.iter().position(|q| q.quest_id == quest_id)
        });

        let slot: usize = match slot_opt.flatten() {
            Some(s) => s,
            None => {
                warn!(
                    "Player {:?} tried to abandon quest {} which is not active",
                    player_guid, quest_id
                );
                return Ok(None);
            }
        };

        // Remove from database first (before updating cache)
        self.repository
            .delete_quest_status(player_guid.counter(), quest_id)
            .await?;

        // Remove from player's active quests
        self.player_mgr.with_player_mut(player_guid, |p| {
            p.active_quests.retain(|q| q.quest_id != quest_id);
        });

        // Clear PLAYER_QUEST_LOG_* update fields so the client removes the quest from UI
        // Each quest slot uses 3 fields: QUEST_ID, COUNT_STATE, TIMER
        const MAX_QUEST_OFFSET: u32 = 3;
        const QUEST_ID_OFFSET: u32 = 0;
        const QUEST_COUNT_STATE_OFFSET: u32 = 1;
        const QUEST_TIME_OFFSET: u32 = 2;

        let slot_u32 = slot as u32;
        let quest_id_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_ID_OFFSET;
        let count_state_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_COUNT_STATE_OFFSET;
        let timer_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_TIME_OFFSET;

        // Convert world ObjectGuid to world::common ObjectGuid for the message
        let world_guid = ObjectGuid::from_low(player_guid.counter());

        let values_update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                .set_field(quest_id_field, 0)      // Clear quest ID
                .set_field(count_state_field, 0)   // Clear count/state
                .set_field(timer_field, 0),        // Clear timer
        ));

        self.broadcast_mgr
            .send_msg_to_player(player_guid, values_update)
            ;

        info!(
            "Player {:?} abandoned quest {} from slot {:?}",
            player_guid, quest_id, slot
        );

        Ok(Some(slot))
    }

    /// Handle kill credit for quest objectives when a creature dies.
    ///
    /// Called from the creature death system when a creature with a loot_recipient dies.
    /// Checks all active quests for matching creature kill objectives, increments counts,
    /// sends progress packets, and marks quests complete when all objectives are met.
    pub fn handle_kill_credit(
        &self,
        player_guid: ObjectGuid,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) {
        // Get player's active quest IDs and their creature objectives
        let active_quests: Vec<u32> = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.active_quests.iter().map(|q| q.quest_id).collect())
            .unwrap_or_default();

        if active_quests.is_empty() {
            return;
        }

        for quest_id in active_quests {
            let Some(quest) = self.manager.get_quest_template(quest_id) else {
                continue;
            };

            // Check each objective slot for a matching creature/GO entry
            for obj_idx in 0..QUEST_OBJECTIVES_COUNT {
                let required_entry = quest.req_creature_or_go_id[obj_idx];

                // Positive = creature entry, negative = gameobject entry
                let matches = if required_entry > 0 {
                    required_entry == creature_entry as i32
                } else if required_entry < 0 {
                    (-required_entry) == creature_entry as i32
                } else {
                    false
                };

                if !matches {
                    continue;
                }

                // Found a matching objective — check and update count
                let update_result = self.player_mgr.with_player_mut(player_guid, |p| {
                    if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) {
                        let current = progress.creature_or_go_count[obj_idx];
                        let required = quest.req_creature_or_go_count[obj_idx];

                        if current < required {
                            progress.creature_or_go_count[obj_idx] += 1;
                            progress.mark_changed();
                            let new_count = progress.creature_or_go_count[obj_idx];

                            // Collect all counts for quest log field update
                            let all_counts = progress.creature_or_go_count;
                            let is_complete = progress.is_complete(&quest);

                            // Find quest slot index
                            let slot = p.active_quests.iter().position(|q| q.quest_id == quest_id);

                            Some((new_count, required, all_counts, is_complete, slot))
                        } else {
                            None // Already at max
                        }
                    } else {
                        None
                    }
                }).flatten();

                let Some((new_count, required_count, all_counts, is_complete, slot)) = update_result else {
                    break; // Quest not found or already at max
                };

                // Entry for packet: GameObjects use 0x80000000 flag
                let entry_for_packet = if required_entry < 0 {
                    ((-required_entry) as u32) | 0x80000000
                } else {
                    required_entry as u32
                };

                // Send SMSG_QUESTUPDATE_ADD_KILL
                let kill_msg = SmsgQuestupdateAddKill {
                    quest_id,
                    entry: entry_for_packet,
                    count: new_count,
                    required_count,
                    guid: creature_guid,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, kill_msg);

                info!(
                    "[QUEST] Kill credit: player {:?} quest {} objective {} — {}/{}",
                    player_guid, quest_id, obj_idx, new_count, required_count
                );

                // Update quest log fields (6-bit packed counters)
                if let Some(slot) = slot {
                    const MAX_QUEST_OFFSET: u32 = 3;
                    const QUEST_COUNT_STATE_OFFSET: u32 = 1;

                    let count_state_field =
                        PLAYER_QUEST_LOG_1_1 + (slot as u32) * MAX_QUEST_OFFSET + QUEST_COUNT_STATE_OFFSET;

                    // Pack all 4 objective counters into 6-bit fields
                    let mut packed: u32 = 0;
                    for i in 0..QUEST_OBJECTIVES_COUNT {
                        let count = (all_counts[i] as u32).min(63);
                        packed |= count << (i as u32 * 6);
                    }

                    let world_guid =
                        ObjectGuid::from_low(player_guid.counter());
                    let values_update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
                        ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                            .set_field(count_state_field, packed),
                    ));
                    self.broadcast_mgr.send_msg_to_player(player_guid, values_update);
                }

                // If quest is now complete, notify client
                if is_complete {
                    let complete_msg = SmsgQuestupdateComplete { quest_id };
                    self.broadcast_mgr.send_msg_to_player(player_guid, complete_msg);
                    info!("[QUEST] Quest {} is now complete for player {:?}", quest_id, player_guid);
                }

                break; // Only update first matching objective per quest
            }
        }
    }

    /// Mark all objectives of a quest complete for a player (area trigger / event completion).
    ///
    /// Called from Lua `COMPLETE_QUEST` actions fired by area triggers or zone scripts.
    /// Sets all creature/go kill counts to their required values, marks the quest as
    /// QuestStatus::Complete, and sends the completion packet.
    pub fn handle_area_event_complete(&self, player_guid: ObjectGuid, quest_id: u32) {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            tracing::debug!("[QUEST] handle_area_event_complete: quest {} not found", quest_id);
            return;
        };

        let marked = self.player_mgr.with_player_mut(player_guid, |p| {
            let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) else {
                return false;
            };
            if progress.is_complete(&quest) {
                return false; // Already complete
            }
            // Fill all creature/go objective counts to their required values
            for i in 0..4 {
                progress.creature_or_go_count[i] = quest.req_creature_or_go_count[i];
            }
            // Fill all item counts
            for i in 0..4 {
                progress.item_count[i] = quest.req_item_count[i];
            }
            progress.explored = true;
            progress.mark_changed();
            true
        }).unwrap_or(false);

        if marked {
            let complete_msg = crate::shared::messages::quest::SmsgQuestupdateComplete { quest_id };
            self.broadcast_mgr.send_msg_to_player(player_guid, complete_msg);
            tracing::info!("[QUEST] Area event: quest {} marked complete for player {:?}", quest_id, player_guid);
        }
    }

    /// Save all active and rewarded quests for a player to the database
    ///
    /// Called during logout to persist quest progress.
    pub async fn save_player_quests(&self, player_guid: ObjectGuid) -> Result<()> {
        use crate::shared::database::characters::models::quest::{
            QuestStatusRow, QuestStatusRewardedRow,
        };

        let player_opt = self.player_mgr.get_player(player_guid);
        if player_opt.is_none() {
            return Ok(());
        }

        let player = player_opt.unwrap();
        let guid = player_guid.counter();

        // Save active quests
        for quest in &player.active_quests {
            let status: u8 = match quest.status {
                QuestStatus::None => 0,
                QuestStatus::Complete => 1,
                QuestStatus::Unavailable => 2,
                QuestStatus::Incomplete => 3,
                QuestStatus::Available => 4,
                QuestStatus::Failed => 5,
            };

            let row = QuestStatusRow {
                guid,
                quest: quest.quest_id,
                status,
                rewarded: quest.rewarded,
                explored: quest.explored,
                timer: quest.timer,
                mob_count1: quest.creature_or_go_count[0],
                mob_count2: quest.creature_or_go_count[1],
                mob_count3: quest.creature_or_go_count[2],
                mob_count4: quest.creature_or_go_count[3],
                item_count1: quest.item_count[0],
                item_count2: quest.item_count[1],
                item_count3: quest.item_count[2],
                item_count4: quest.item_count[3],
            };

            self.repository.save_quest_status(&row).await?;
        }

        // Save rewarded quests
        for &quest_id in &player.rewarded_quests {
            let row = QuestStatusRewardedRow {
                guid,
                quest: quest_id,
            };

            self.repository.save_rewarded_quest(&row).await?;
        }

        drop(player);

        debug!(
            "Saved quest data for player {:?}: {} active, {} rewarded",
            player_guid,
            self.player_mgr
                .get_player(player_guid)
                .map(|p| p.active_quests.len())
                .unwrap_or(0),
            self.player_mgr
                .get_player(player_guid)
                .map(|p| p.rewarded_quests.len())
                .unwrap_or(0)
        );

        Ok(())
    }

    /// Load all quests for a player from the database
    ///
    /// Called during login to restore quest progress.
    pub async fn load_player_quests(&self, player_guid: ObjectGuid) -> Result<()> {
        let guid = player_guid.counter();

        // Load active quests
        let quest_statuses = self.repository.find_quest_statuses(guid).await?;

        for row in quest_statuses {
            let status = match row.status {
                0 => QuestStatus::None,
                1 => QuestStatus::Complete,
                2 => QuestStatus::Unavailable,
                3 => QuestStatus::Incomplete,
                4 => QuestStatus::Available,
                5 => QuestStatus::Failed,
                _ => QuestStatus::Incomplete,
            };

            let progress = QuestProgress {
                quest_id: row.quest,
                status,
                rewarded: row.rewarded,
                explored: row.explored,
                timer: row.timer,
                reward_choice: 0, // Not saved in DB, will be set when quest is completed
                creature_or_go_count: [
                    row.mob_count1,
                    row.mob_count2,
                    row.mob_count3,
                    row.mob_count4,
                ],
                item_count: [
                    row.item_count1,
                    row.item_count2,
                    row.item_count3,
                    row.item_count4,
                ],
                update_state: super::types::QuestUpdateState::Unchanged,
            };

            self.player_mgr.with_player_mut(player_guid, |p| {
                p.active_quests.push(progress);
            });
        }

        // Load rewarded quests
        let rewarded = self.repository.find_rewarded_quests(guid).await?;

        for row in rewarded {
            self.player_mgr.with_player_mut(player_guid, |p| {
                p.rewarded_quests.insert(row.quest);
            });
        }

        info!(
            "Loaded quest data for player {:?}: {} active, {} rewarded",
            player_guid,
            self.player_mgr
                .get_player(player_guid)
                .map(|p| p.active_quests.len())
                .unwrap_or(0),
            self.player_mgr
                .get_player(player_guid)
                .map(|p| p.rewarded_quests.len())
                .unwrap_or(0)
        );

        Ok(())
    }
}
