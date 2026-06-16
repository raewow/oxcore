//! Quest System - business logic for quest operations
//!
//! Handles quest giver status, quest validation, accept/complete, and packet sending.
//! Integrates with gossip system for quest menu display.

use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::shared::database::characters::models::quest::{QuestStatusRewardedRow, QuestStatusRow};
use crate::shared::database::characters::repositories::QuestRepositoryTrait;
use crate::shared::messages::gossip::SmsgGossipComplete;
use crate::shared::messages::quest::{
    MsgQuestPushResult, QuestListItem, RequestItemInfo, RewardItemInfo, SmsgQuestConfirmAccept,
    SmsgQuestgiverOfferRewardV2, SmsgQuestgiverQuestComplete, SmsgQuestgiverQuestDetailsV2,
    SmsgQuestgiverQuestListV2, SmsgQuestgiverRequestItemsV2, SmsgQuestgiverStatus,
    SmsgQuestlogFull, SmsgQuestupdateAddItem, SmsgQuestupdateAddKill, SmsgQuestupdateComplete,
    SmsgQuestupdateFailed, SmsgQuestupdateFailedtimer,
};
use crate::shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::common::update_fields::PLAYER_QUEST_LOG_1_1;
use crate::world::game::creature::CreatureManager;
use crate::world::game::inventory::{AddItemResult, GoldResult, InventorySystem};
use crate::world::game::items::ItemManager;
use crate::world::game::player::experience::ExperienceSystem;
use crate::world::game::player::player::QuestShareInfo;
use crate::world::game::player::PlayerManager;
use crate::world::World;

use super::manager::QuestManager;
use super::types::{
    DialogStatus, QuestFlags, QuestProgress, QuestSpecialFlags, QuestStatus, QuestTemplate,
    MAX_QUEST_LOG_SIZE, QUEST_OBJECTIVES_COUNT,
};

/// Maximum distance for quest sharing between players.
pub const QUEST_SHARE_DISTANCE: f32 = 14.0;

/// Quest system - handles business logic and packet sending
pub struct QuestSystem {
    pub manager: Arc<QuestManager>,
    repository: Arc<dyn QuestRepositoryTrait>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
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
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
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

        self.get_quest_giver_status_from_relations(player_guid, start_quests, finish_quests, world)
    }

    /// Calculate quest giver status for a concrete creature or gameobject GUID.
    ///
    /// Follows C++ ordering: script GetDialogStatus first (values 0-7 override),
    /// then falls back to relation-based status computation.
    pub fn get_quest_giver_status_for_guid(
        &self,
        quest_giver_guid: ObjectGuid,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Option<DialogStatus> {
        // Step 1: Resolve questgiver type to get entry
        let (entry, start_quests, finish_quests) =
            self.quest_giver_relations(quest_giver_guid, world)?;

        // Step 2: Script first chance — Lua OnDialogStatus overrides relation status
        // Values 0-7 from script override directly; >7 = fallback
        if let Some(script) = world.managers.lua_mgr.get_gossip_script(entry) {
            let player_snap = build_player_snapshot(player_guid, world);
            let script_status: Option<u32> = world
                .managers
                .lua_mgr
                .with_lua(|lua| script.get_dialog_status(lua, &player_snap));
            if let Some(status) = script_status {
                if status <= 7 {
                    return Some(match status {
                        0 => DialogStatus::None,
                        1 => DialogStatus::Unavailable,
                        2 => DialogStatus::Chat,
                        3 => DialogStatus::Incomplete,
                        4 => DialogStatus::RewardRep,
                        5 => DialogStatus::Available,
                        6 => DialogStatus::RewardOld,
                        7 => DialogStatus::Reward2,
                        _ => unreachable!(),
                    });
                }
            }
        }

        // Also try gameobject script for GO questgivers
        if quest_giver_guid.is_game_object() {
            if let Some(script) = world.managers.lua_mgr.get_game_object_script(entry) {
                let player_snap = build_player_snapshot(player_guid, world);
                let script_status: Option<u32> = world
                    .managers
                    .lua_mgr
                    .with_lua(|lua| script.get_dialog_status(lua, &player_snap));
                if let Some(status) = script_status {
                    if status <= 7 {
                        return Some(match status {
                            0 => DialogStatus::None,
                            1 => DialogStatus::Unavailable,
                            2 => DialogStatus::Chat,
                            3 => DialogStatus::Incomplete,
                            4 => DialogStatus::RewardRep,
                            5 => DialogStatus::Available,
                            6 => DialogStatus::RewardOld,
                            7 => DialogStatus::Reward2,
                            _ => unreachable!(),
                        });
                    }
                }
            }
        }

        // Step 3: Fallback to relation-based status computation

        Some(self.get_quest_giver_status_from_relations(
            player_guid,
            start_quests,
            finish_quests,
            world,
        ))
    }

    fn get_quest_giver_status_from_relations(
        &self,
        player_guid: ObjectGuid,
        start_quests: Vec<u32>,
        finish_quests: Vec<u32>,
        world: &World,
    ) -> DialogStatus {
        let mut status = DialogStatus::None;

        // Get player quest info
        let (active_quests, rewarded_quests, player_level) = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| {
                let active: HashSet<u32> = p.active_quests.iter().map(|q| q.quest_id).collect();
                let rewarded: HashSet<u32> = p.rewarded_quests.iter().copied().collect();
                (active, rewarded, p.level)
            })
            .unwrap_or_else(|| (HashSet::new(), HashSet::new(), 0));

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
                let hide_diff = world.config.quest_low_level_hide_diff;
                if hide_diff > 0
                    && quest.quest_level > 0
                    && (player_level as u32) > quest.quest_level
                    && (player_level as u32) - quest.quest_level >= hide_diff
                {
                    status = status.max(DialogStatus::Chat);
                } else {
                    status = status.max(DialogStatus::Available);
                }
            } else {
                status = status.max(DialogStatus::Unavailable);
            }
        }

        status
    }

    fn quest_giver_relations(
        &self,
        quest_giver_guid: ObjectGuid,
        world: &World,
    ) -> Option<(u32, Vec<u32>, Vec<u32>)> {
        if let Some(entry) = self
            .creature_mgr
            .get_creature(quest_giver_guid)
            .map(|c| c.entry)
        {
            return Some((
                entry,
                self.manager.get_creature_quest_relations(entry),
                self.manager.get_creature_involved_relations(entry),
            ));
        }

        world
            .managers
            .gameobject_mgr
            .get_gameobject(quest_giver_guid)
            .map(|go| {
                (
                    go.entry,
                    self.manager.get_go_quest_relations(go.entry),
                    self.manager.get_go_involved_relations(go.entry),
                )
            })
    }

    pub fn quest_giver_can_start_or_finish(
        &self,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        world: &World,
    ) -> bool {
        self.quest_giver_relations(quest_giver_guid, world)
            .map(|(_, start_quests, finish_quests)| {
                start_quests.contains(&quest_id) || finish_quests.contains(&quest_id)
            })
            .unwrap_or(false)
    }

    pub(crate) fn can_store_reward_items(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        reward_choice: u32,
    ) -> Option<bool> {
        let mut rewards = Vec::new();

        let choice_count = quest.get_rew_choice_items_count() as u32;
        if reward_choice >= choice_count {
            return None;
        }
        if choice_count > 0 {
            let idx = reward_choice as usize;
            rewards.push((
                quest.rew_choice_item_id[idx],
                quest.rew_choice_item_count[idx],
            ));
        }

        for i in 0..super::types::QUEST_REWARDS_COUNT {
            rewards.push((quest.rew_item_id[i], quest.rew_item_count[i]));
        }

        let mut free_slots = self
            .inventory
            .cache()
            .count_free_inventory_slots(player_guid);
        for (item_id, count) in rewards {
            if item_id == 0 || count == 0 {
                continue;
            }

            let max_stack = self
                .item_mgr
                .get_template(item_id)
                .map(|template| template.stackable.max(1))
                .unwrap_or(1);
            let existing_space = if max_stack > 1 {
                self.inventory
                    .find_items_by_entry(player_guid, item_id)
                    .into_iter()
                    .filter_map(|item_guid| {
                        self.inventory
                            .cache()
                            .get_item(player_guid, item_guid)
                            .map(|item| item.read().count)
                    })
                    .map(|carried| max_stack.saturating_sub(carried))
                    .sum::<u32>()
            } else {
                0
            };
            let remaining = count.saturating_sub(existing_space);
            let slots_needed = if remaining == 0 {
                0
            } else {
                (remaining + max_stack - 1) / max_stack
            };

            if slots_needed > free_slots {
                return Some(false);
            }
            free_slots -= slots_needed;
        }

        Some(true)
    }

    /// Validate if player can take quest (12 validation checks)
    pub fn can_take_quest(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        world: &World,
    ) -> bool {
        let Some(player) = self.player_mgr.get_player(player_guid) else {
            return false;
        };

        let active: HashSet<u32> = player.active_quests.iter().map(|q| q.quest_id).collect();
        let rewarded: HashSet<u32> = player.rewarded_quests.iter().copied().collect();

        if !quest.is_active {
            return false;
        }

        // Already active
        if active.contains(&quest.id) {
            return false;
        }

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
                *id == quest.required_skill as u16
                    && (skill_data.current_value as u32) >= quest.required_skill_value
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

        // 8b. Do not offer an earlier chain step or breadcrumb once the next quest is active/rewarded.
        if quest.next_quest_id > 0 {
            let next_quest_id = quest.next_quest_id as u32;
            if active.contains(&next_quest_id) || rewarded.contains(&next_quest_id) {
                return false;
            }
        }
        if quest.next_quest_in_chain > 0
            && (active.contains(&quest.next_quest_in_chain)
                || rewarded.contains(&quest.next_quest_in_chain))
        {
            return false;
        }

        for active_quest_id in &active {
            if let Some(active_quest) = self.manager.get_quest_template(*active_quest_id) {
                if active_quest.breadcrumb_for_quest_id == quest.id as i32 {
                    return false;
                }
            }
        }

        // 9. Reputation requirement (min)
        if quest.required_min_rep_faction != 0 {
            let base_rep = world
                .dbc
                .read()
                .get_faction(quest.required_min_rep_faction)
                .map(|entry| entry.get_base_reputation(player.race, player.class))
                .unwrap_or(0);
            let reputation = player
                .reputation
                .get_standing_by_faction_id(quest.required_min_rep_faction)
                .map(|standing| standing.get_absolute_reputation(base_rep))
                .unwrap_or(base_rep);
            if reputation < quest.required_min_rep_value {
                return false;
            }
        }

        // 10. Reputation requirement (max)
        if quest.required_max_rep_faction != 0 {
            let base_rep = world
                .dbc
                .read()
                .get_faction(quest.required_max_rep_faction)
                .map(|entry| entry.get_base_reputation(player.race, player.class))
                .unwrap_or(0);
            let reputation = player
                .reputation
                .get_standing_by_faction_id(quest.required_max_rep_faction)
                .map(|standing| standing.get_absolute_reputation(base_rep))
                .unwrap_or(base_rep);
            if reputation >= quest.required_max_rep_value {
                return false;
            }
        }

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

    /// Check if player can complete quest (auto-complete or objective completion)
    /// Core completion validation without can_take_quest dependency.
    /// Separated for testability.
    pub(crate) fn can_complete_quest_basic(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
    ) -> bool {
        if quest.id == 0 {
            return false;
        }

        let (active_status, progress) = self
            .player_mgr
            .with_player(player_guid, |p| {
                p.active_quests
                    .iter()
                    .find(|q| q.quest_id == quest.id)
                    .map(|prog| (prog.status, prog.clone()))
            })
            .flatten()
            .unwrap_or((QuestStatus::None, QuestProgress::new(0)));

        if active_status == QuestStatus::Complete {
            return false; // Already complete
        }

        if active_status != QuestStatus::Incomplete {
            return false;
        }

        if progress.quest_id == 0 {
            return false;
        }

        // Check item objectives
        if quest.special_flags.contains(QuestSpecialFlags::DELIVER) {
            for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
                if quest.req_item_count[i] != 0
                    && self
                        .inventory
                        .count_items(player_guid, quest.req_item_id[i])
                        < quest.req_item_count[i]
                {
                    return false;
                }
            }
        }

        // Check creature/GO objectives
        if quest
            .special_flags
            .contains(QuestSpecialFlags::KILL_OR_CAST)
            || quest.special_flags.contains(QuestSpecialFlags::SPEAKTO)
        {
            for i in 0..super::types::QUEST_OBJECTIVES_COUNT {
                if quest.req_creature_or_go_count[i] != 0
                    && progress.creature_or_go_count[i] < quest.req_creature_or_go_count[i]
                {
                    return false;
                }
            }
        }

        // Check exploration
        if quest
            .special_flags
            .contains(QuestSpecialFlags::EXPLORATION_OR_EVENT)
            && !progress.explored
        {
            return false;
        }

        // Check timer
        if quest.special_flags.contains(QuestSpecialFlags::TIMED) && progress.timer == 0 {
            return false;
        }

        // Check money
        if quest.rew_or_req_money < 0 {
            let required_money = (-quest.rew_or_req_money) as u32;
            if self.inventory.get_money(player_guid).unwrap_or(0) < required_money {
                return false;
            }
        }

        // Check reputation objective
        if quest.rep_objective_faction != 0 {
            let rep = self
                .player_mgr
                .with_player(player_guid, |p| {
                    p.reputation
                        .get_standing_by_faction_id(quest.rep_objective_faction)
                        .map(|s| s.get_absolute_reputation(0))
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            if rep < quest.rep_objective_value {
                return false;
            }
        }

        true
    }

    fn can_complete_quest(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        world: &World,
    ) -> bool {
        // Auto-rewarded quests
        if quest.quest_flags.contains(QuestFlags::AUTO_REWARDED) {
            return self.can_take_quest(player_guid, quest, world);
        }

        // Auto-complete quest
        if quest.is_auto_complete() {
            return self.can_take_quest(player_guid, quest, world);
        }

        self.can_complete_quest_basic(player_guid, quest)
    }

    /// Check if player can reward quest (basic version without reward index)
    /// Core reward validation without auto-complete/can_take_quest dependency.
    /// Separated for testability.
    pub(crate) fn can_reward_quest_basic(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
    ) -> bool {
        // Normal quests must be accepted and complete
        let status = self.get_quest_status(player_guid, quest.id);
        if status != QuestStatus::Complete {
            return false;
        }

        // Prevent completing the same quest twice
        let already_rewarded = self
            .player_mgr
            .with_player(player_guid, |p| p.rewarded_quests.contains(&quest.id))
            .unwrap_or(false);
        if already_rewarded && !quest.is_repeatable() {
            return false;
        }

        // Check required items
        if quest.special_flags.contains(QuestSpecialFlags::DELIVER) {
            for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
                if quest.req_item_count[i] != 0
                    && self
                        .inventory
                        .count_items(player_guid, quest.req_item_id[i])
                        < quest.req_item_count[i]
                {
                    return false;
                }
            }
        }

        // Check money
        if quest.rew_or_req_money < 0 {
            let required_money = (-quest.rew_or_req_money) as u32;
            if self.inventory.get_money(player_guid).unwrap_or(0) < required_money {
                return false;
            }
        }

        true
    }

    fn can_reward_quest(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        world: &World,
    ) -> bool {
        // Auto-complete quests must be takable
        if quest.is_auto_complete() {
            if !self.can_take_quest(player_guid, quest, world) {
                return false;
            }
        }

        self.can_reward_quest_basic(player_guid, quest)
    }

    /// Check if player can reward quest with specific reward choice
    fn can_reward_quest_with_reward(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        reward_choice: u32,
        world: &World,
    ) -> bool {
        if !self.can_reward_quest(player_guid, quest, world) {
            return false;
        }

        // Check reward choice can fit in inventory
        let Some(can_store) = self.can_store_reward_items(player_guid, quest, reward_choice) else {
            return false;
        };

        can_store
    }

    /// Check if player can complete a repeatable quest
    fn can_complete_repeatable_quest(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        world: &World,
    ) -> bool {
        if !self.can_take_quest(player_guid, quest, world) {
            return false;
        }

        if quest.special_flags.contains(QuestSpecialFlags::DELIVER) {
            for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
                if quest.req_item_id[i] != 0
                    && quest.req_item_count[i] != 0
                    && self
                        .inventory
                        .count_items(player_guid, quest.req_item_id[i])
                        < quest.req_item_count[i]
                {
                    return false;
                }
            }
        }

        self.can_reward_quest(player_guid, quest, world)
    }

    /// Auto-complete quest (mark as complete, sync item objectives)
    pub(crate) fn complete_quest(
        &self,
        player_guid: ObjectGuid,
        quest_id: u32,
        quest: &QuestTemplate,
    ) {
        self.player_mgr.with_player_mut(player_guid, |p| {
            if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) {
                // Sync item objectives from inventory
                for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
                    if quest.req_item_id[i] != 0 && quest.req_item_count[i] != 0 {
                        let carried = self
                            .inventory
                            .count_items(player_guid, quest.req_item_id[i])
                            .min(quest.req_item_count[i]);
                        if carried > progress.item_count[i] {
                            progress.item_count[i] = carried;
                        }
                    }
                }
                progress.status = QuestStatus::Complete;
                progress.mark_changed();
            }
        });
    }

    /// Get quest status for player
    pub fn get_quest_status(&self, player_guid: ObjectGuid, quest_id: u32) -> QuestStatus {
        let Some(template) = self.manager.get_quest_template(quest_id) else {
            return QuestStatus::None;
        };

        let Some(active_complete) = self.player_mgr.with_player(player_guid, |p| {
            p.active_quests
                .iter()
                .find(|q| q.quest_id == quest_id)
                .map(|progress| progress.is_complete(&template))
        }) else {
            return QuestStatus::None;
        };

        match active_complete {
            Some(true) => QuestStatus::Complete,
            Some(false) => {
                if self.sync_item_objectives_from_inventory(player_guid, &template) {
                    QuestStatus::Complete
                } else {
                    QuestStatus::Incomplete
                }
            }
            None => QuestStatus::None,
        }
    }

    pub(crate) fn inventory_satisfies_required_items(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
    ) -> bool {
        for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
            if quest.req_item_id[i] != 0 && quest.req_item_count[i] > 0 {
                let carried = self
                    .inventory
                    .count_items(player_guid, quest.req_item_id[i]);
                if carried < quest.req_item_count[i] {
                    return false;
                }
            }
        }

        true
    }

    fn sync_item_objectives_from_inventory(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
    ) -> bool {
        self.player_mgr
            .with_player_mut(player_guid, |p| {
                let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest.id)
                else {
                    return false;
                };

                let mut changed = false;
                for i in 0..super::types::QUEST_ITEM_OBJECTIVES_COUNT {
                    if quest.req_item_id[i] == 0 || quest.req_item_count[i] == 0 {
                        continue;
                    }

                    let carried = self
                        .inventory
                        .count_items(player_guid, quest.req_item_id[i])
                        .min(quest.req_item_count[i]);
                    if carried > progress.item_count[i] {
                        progress.item_count[i] = carried;
                        changed = true;
                    }
                }

                if changed {
                    progress.mark_changed();
                }

                progress.is_complete(quest)
            })
            .unwrap_or(false)
    }

    pub(crate) fn active_quest_is_complete_without_sync(
        &self,
        player_guid: ObjectGuid,
        quest_id: u32,
        quest: &QuestTemplate,
    ) -> bool {
        self.player_mgr
            .with_player(player_guid, |p| {
                p.active_quests
                    .iter()
                    .find(|q| q.quest_id == quest_id)
                    .map(|progress| progress.is_complete(quest))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    pub(crate) fn active_quest_is_complete(
        &self,
        player_guid: ObjectGuid,
        quest_id: u32,
        quest: &QuestTemplate,
    ) -> bool {
        self.active_quest_is_complete_without_sync(player_guid, quest_id, quest)
            || self.sync_item_objectives_from_inventory(player_guid, quest)
    }

    fn pack_quest_count_state(progress: &QuestProgress) -> u32 {
        const QUEST_STATE_COMPLETE: u32 = 0x01;
        const QUEST_STATE_FAIL: u32 = 0x02;

        let mut packed = 0u32;
        for i in 0..QUEST_OBJECTIVES_COUNT {
            let count = progress.creature_or_go_count[i].min(63);
            packed |= count << (i as u32 * 6);
        }

        let state = match progress.status {
            QuestStatus::Complete => QUEST_STATE_COMPLETE,
            QuestStatus::Failed => QUEST_STATE_FAIL,
            _ => 0,
        };
        packed | (state << 24)
    }

    /// Send quest giver status packet
    pub fn send_quest_giver_status(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        world: &World,
    ) -> bool {
        let Some(local_status) =
            self.get_quest_giver_status_for_guid(quest_giver_guid, player_guid, world)
        else {
            return false;
        };

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
            guid: quest_giver_guid,
            status,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
        true
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
        self.prepare_quest_menu_from_relations(player_guid, start_quests, finish_quests, world)
    }

    pub fn prepare_gameobject_quest_menu(
        &self,
        player_guid: ObjectGuid,
        entry: u32,
        world: &World,
    ) -> Vec<super::types::GossipQuestData> {
        let start_quests = self.manager.get_go_quest_relations(entry);
        let finish_quests = self.manager.get_go_involved_relations(entry);
        self.prepare_quest_menu_from_relations(player_guid, start_quests, finish_quests, world)
    }

    fn prepare_quest_menu_from_relations(
        &self,
        player_guid: ObjectGuid,
        start_quests: Vec<u32>,
        finish_quests: Vec<u32>,
        world: &World,
    ) -> Vec<super::types::GossipQuestData> {
        let rewarded_quests: HashSet<u32> = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.rewarded_quests.iter().copied().collect())
            .unwrap_or_default();

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

            if status == QuestStatus::Complete && !rewarded_quests.contains(&quest_id) {
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
            activate_accept: true,
            quest_flags: crate::shared::messages::quest::QuestFlags(quest.quest_flags.bits()),
            reward_choices: &reward_choices,
            reward_items: &reward_items,
            money_reward: quest.rew_or_req_money.max(0) as u32,
            rew_spell: quest.rew_spell,
            details_emote: quest.details_emote,
            details_emote_delay: quest.details_emote_delay,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
        Ok(())
    }

    /// Internal helper: add an accepted quest to a player, persist it, and send updates.
    ///
    /// Returns `true` on success. On failure, sends `SmsgQuestlogFull` and returns `false`.
    /// This is shared between direct accept and party confirm accept paths.
    async fn add_accepted_quest(
        &self,
        player_guid: ObjectGuid,
        quest: &QuestTemplate,
        world: &World,
    ) -> bool {
        // Check quest log size
        let can_add = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.active_quests.len() < MAX_QUEST_LOG_SIZE)
            .unwrap_or(false);

        if !can_add {
            let msg = SmsgQuestlogFull;
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return false;
        }

        // Add quest to player and get the slot index
        let Some(slot) = self.player_mgr.with_player_mut(player_guid, |p| {
            let slot = p.active_quests.len();
            p.active_quests.push(QuestProgress::new(quest.id));
            slot
        }) else {
            warn!(
                "Player {:?} not found when adding quest {}",
                player_guid, quest.id
            );
            return false;
        };

        let quest_id_for_spawn = quest.id;
        let accepted_row = QuestStatusRow {
            guid: player_guid.counter(),
            quest: quest_id_for_spawn,
            status: QuestStatus::Incomplete as u8,
            rewarded: false,
            explored: false,
            timer: 0,
            mob_count1: 0,
            mob_count2: 0,
            mob_count3: 0,
            mob_count4: 0,
            item_count1: 0,
            item_count2: 0,
            item_count3: 0,
            item_count4: 0,
            reward_choice: 0,
        };
        let repository = Arc::clone(&self.repository);
        tokio::spawn(async move {
            if let Err(e) = repository.save_quest_status(&accepted_row).await {
                warn!(
                    "Failed to persist accepted quest {} for player {:?}: {}",
                    quest_id_for_spawn, player_guid, e
                );
            }
        });

        // Update PLAYER_QUEST_LOG_* update fields
        const MAX_QUEST_OFFSET: u32 = 3;
        const QUEST_ID_OFFSET: u32 = 0;
        const QUEST_COUNT_STATE_OFFSET: u32 = 1;
        const QUEST_TIME_OFFSET: u32 = 2;

        let slot_u32 = slot as u32;
        let quest_id_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_ID_OFFSET;
        let count_state_field =
            PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_COUNT_STATE_OFFSET;
        let timer_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_TIME_OFFSET;

        let world_guid = ObjectGuid::from_low(player_guid.counter());
        let values_update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                .set_field(quest_id_field, quest.id)
                .set_field(count_state_field, 0)
                .set_field(timer_field, 0),
        ));
        self.broadcast_mgr
            .send_msg_to_player(player_guid, values_update);

        if quest.src_spell != 0 {
            if let Err(e) = world
                .systems
                .spells
                .cast_spell(player_guid, quest.src_spell, Some(player_guid), true, world)
                .await
            {
                warn!(
                    "Failed to cast source spell {} for quest {} on {:?}: {}",
                    quest.src_spell, quest.id, player_guid, e
                );
            }
        }

        if quest.is_auto_complete() || self.sync_item_objectives_from_inventory(player_guid, quest)
        {
            self.player_mgr.with_player_mut(player_guid, |p| {
                if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest.id)
                {
                    if progress.is_complete(quest) || quest.is_auto_complete() {
                        progress.status = QuestStatus::Complete;
                        progress.mark_changed();
                    }
                }
            });

            let complete_msg = SmsgQuestupdateComplete { quest_id: quest.id };
            self.broadcast_mgr
                .send_msg_to_player(player_guid, complete_msg);
        }

        true
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

        // Validate that the quest giver is either a valid NPC starter or the
        // quest-starting item in the player's inventory.
        let creature_entry = self
            .creature_mgr
            .get_creature(quest_giver_guid)
            .map(|c| c.entry);
        let starts_from_object = self
            .quest_giver_relations(quest_giver_guid, world)
            .map(|(_, start_quests, _)| start_quests.contains(&quest_id))
            .unwrap_or(false);
        let starts_from_item = self
            .inventory
            .cache()
            .get_item(player_guid, quest_giver_guid)
            .and_then(|item| self.item_mgr.get_template(item.read().entry))
            .map(|template| template.start_quest == quest_id)
            .unwrap_or(false);

        if !starts_from_object && !starts_from_item {
            warn!(
                "Player {:?} tried to accept quest {} from invalid quest giver {:?}",
                player_guid, quest_id, quest_giver_guid
            );
            self.player_mgr.clear_quest_share_info(player_guid);
            let msg = SmsgGossipComplete;
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Validate can take quest
        if !self.can_take_quest(player_guid, &quest, world) {
            warn!("Player {:?} cannot accept quest {}", player_guid, quest_id);
            self.player_mgr.clear_quest_share_info(player_guid);
            let msg = SmsgGossipComplete;
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Handle stored quest-share info
        if let Some(share) = self.player_mgr.get_quest_share_info(player_guid) {
            if share.quest_id == quest.id {
                // Find the original sharer
                if let Some(sharer) = self.player_mgr.get_player(share.player_guid) {
                    let sharer_pos = sharer.movement.position;
                    let player_pos = self.player_mgr.get_position(player_guid);
                    let same_map = sharer.map_id
                        == self
                            .player_mgr
                            .get_player(player_guid)
                            .map(|p| p.map_id)
                            .unwrap_or(u32::MAX);
                    let within_distance = player_pos
                        .map(|p| p.is_within_range(&sharer_pos, QUEST_SHARE_DISTANCE))
                        .unwrap_or(false);

                    if !same_map || !within_distance {
                        self.player_mgr.clear_quest_share_info(player_guid);
                        let msg = SmsgGossipComplete;
                        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                        let too_far = MsgQuestPushResult {
                            sender_guid: player_guid,
                            msg: 4, // QUEST_PARTY_MSG_TOO_FAR
                        };
                        self.broadcast_mgr
                            .send_msg_to_player(share.player_guid, too_far);
                        return Ok(());
                    }

                    // Notify sharer that the quest was accepted
                    let accepted = MsgQuestPushResult {
                        sender_guid: player_guid,
                        msg: 2, // QUEST_PARTY_MSG_ACCEPT_QUEST
                    };
                    self.broadcast_mgr
                        .send_msg_to_player(share.player_guid, accepted);
                }
                self.player_mgr.clear_quest_share_info(player_guid);
            } else {
                self.player_mgr.clear_quest_share_info(player_guid);
            }
        }

        // Add source item if present
        if quest.src_item_id != 0 && quest.src_item_count > 0 {
            let result = self
                .inventory
                .add_item(player_guid, quest.src_item_id, quest.src_item_count)
                .await;
            if !matches!(result, AddItemResult::Success { .. }) {
                warn!(
                    "Player {:?} cannot accept quest {}: failed to add source item {} x{}: {:?}",
                    player_guid, quest_id, quest.src_item_id, quest.src_item_count, result
                );
                self.player_mgr.clear_quest_share_info(player_guid);
                let msg = SmsgGossipComplete;
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        }

        if !self.add_accepted_quest(player_guid, &quest, world).await {
            self.player_mgr.clear_quest_share_info(player_guid);
            let msg = SmsgGossipComplete;
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Send gossip complete to close quest window
        let msg = SmsgGossipComplete;
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        // Party accept: fanout confirm prompts to eligible group members
        if quest.quest_flags.has_flag(QuestFlags::PARTY_ACCEPT) {
            if let Some(group) = world.systems.group.get_player_group(player_guid) {
                let player_map_id = self.player_mgr.get_player(player_guid).map(|p| p.map_id);
                for member in &group.members {
                    if member.guid == player_guid {
                        continue;
                    }
                    if !member.status.is_online() {
                        continue;
                    }
                    let member_map_id = self.player_mgr.get_player(member.guid).map(|p| p.map_id);
                    if member_map_id != player_map_id {
                        continue;
                    }
                    if !self.can_take_quest(member.guid, &quest, world) {
                        continue;
                    }
                    // Set share info on the member
                    self.player_mgr.set_quest_share_info(
                        member.guid,
                        QuestShareInfo {
                            player_guid,
                            quest_id: quest.id,
                        },
                    );
                    // Close any gossip window on the member
                    let close = SmsgGossipComplete;
                    self.broadcast_mgr.send_msg_to_player(member.guid, close);
                    // Send confirm-accept prompt
                    let confirm = SmsgQuestConfirmAccept {
                        quest_id: quest.id,
                        title: quest.title.clone(),
                        sender_guid: player_guid,
                    };
                    self.broadcast_mgr.send_msg_to_player(member.guid, confirm);
                }
            }
        }

        // Fire OnQuestAccept Lua callback if a gossip script is registered for this NPC
        if let Some(entry) = creature_entry {
            if let Some(script) = world.managers.lua_mgr.get_gossip_script(entry) {
                let player_snap = build_player_snapshot(player_guid, world);
                let actions = world.managers.lua_mgr.with_lua(|lua| {
                    script.on_quest_accept(lua, &player_snap, quest_giver_guid, quest_id)
                });
                if !actions.is_empty() {
                    execute_gossip_actions(actions, player_guid, quest_giver_guid, world).await?;
                }
            }
        }

        info!(
            "Player {:?} accepted quest {} from {:?}",
            player_guid, quest_id, quest_giver_guid
        );
        Ok(())
    }

    /// Handle quest confirm accept
    ///
    /// Called when a player confirms accepting a quest shared by a party member.
    pub async fn handle_quest_confirm_accept(
        &self,
        player_guid: ObjectGuid,
        quest_id: u32,
        world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            warn!("Quest {} not found for confirm accept", quest_id);
            return Ok(());
        };

        // Must have PARTY_ACCEPT flag
        if !quest.quest_flags.has_flag(QuestFlags::PARTY_ACCEPT) {
            return Ok(());
        }

        // Must have matching share info
        let Some(share) = self.player_mgr.get_quest_share_info(player_guid) else {
            return Ok(());
        };
        if share.quest_id != quest_id {
            return Ok(());
        }

        // Original sharer must still be online
        let Some(original) = self.player_mgr.get_player(share.player_guid) else {
            self.player_mgr.clear_quest_share_info(player_guid);
            return Ok(());
        };

        // Group/raid relationship check
        let player_group = world.systems.group.get_player_group(player_guid);
        let original_group = world.systems.group.get_player_group(share.player_guid);
        let in_same_group = player_group.is_some()
            && original_group.is_some()
            && player_group.as_ref().unwrap().id == original_group.as_ref().unwrap().id;

        if quest.quest_type == crate::world::game::npc::quest::types::QuestType::Raid {
            if !in_same_group {
                return Ok(());
            }
        } else {
            // For non-raid quests, require same group (not just raid)
            if !in_same_group {
                return Ok(());
            }
            // Also ensure the group is not a raid for non-raid quests
            if let Some(ref g) = player_group {
                if g.is_raid {
                    return Ok(());
                }
            }
        }

        // Sharable validation: if SHARABLE flag, original player must still have the quest
        if quest.quest_flags.has_flag(QuestFlags::SHARABLE) {
            let original_has_quest = self.player_mgr.with_player(share.player_guid, |p| {
                p.active_quests.iter().any(|q| q.quest_id == quest_id)
            });
            if !original_has_quest.unwrap_or(false) {
                return Ok(());
            }
        }

        // Must have either SHARABLE or PARTY_ACCEPT
        if !quest
            .quest_flags
            .has_flag(QuestFlags::SHARABLE | QuestFlags::PARTY_ACCEPT)
        {
            return Ok(());
        }

        // Can take quest?
        if !self.can_take_quest(player_guid, &quest, world) {
            return Ok(());
        }

        // Add quest without a questgiver object to avoid duplicate DB script execution
        if !self.add_accepted_quest(player_guid, &quest, world).await {
            return Ok(());
        }

        self.player_mgr.clear_quest_share_info(player_guid);

        // Send gossip complete to close the confirmation window
        let msg = SmsgGossipComplete;
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        info!(
            "Player {:?} confirmed accept of shared quest {} from {:?}",
            player_guid, quest_id, share.player_guid
        );
        Ok(())
    }

    /// Handle quest complete request
    ///
    /// Called when the Vanilla client sends CMSG_QUESTGIVER_COMPLETE_QUEST.
    /// This is sent when clicking any active/incomplete/complete quest in the quest list.
    /// - Incomplete quests: send SMSG_QUESTGIVER_REQUEST_ITEMS with completable=false
    /// - Complete quests: send SMSG_QUESTGIVER_REQUEST_ITEMS with completable=true
    pub async fn handle_quest_complete(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            warn!("Cannot complete quest {}: not found", quest_id);
            return Ok(());
        };

        if !self.quest_giver_can_start_or_finish(quest_giver_guid, quest_id, world) {
            warn!(
                "Player {:?} tried to complete quest {} from quest giver {:?} who is not involved in this quest",
                player_guid, quest_id, quest_giver_guid
            );
            return Ok(());
        }

        let status = self.get_quest_status(player_guid, quest_id);

        let completable = if status == QuestStatus::Complete {
            self.can_reward_quest(player_guid, &quest, world)
        } else {
            if quest.is_repeatable() {
                self.can_complete_repeatable_quest(player_guid, &quest, world)
            } else {
                self.can_reward_quest(player_guid, &quest, world)
            }
        };

        self.send_request_items(player_guid, quest_giver_guid, &quest, completable);

        Ok(())
    }

    fn send_request_items(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest: &QuestTemplate,
        completable: bool,
    ) {
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
            completable,
            close_on_cancel: false,
            req_money: if quest.rew_or_req_money < 0 {
                (-quest.rew_or_req_money) as u32
            } else {
                0
            },
            req_items: &req_items,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    pub fn handle_quest_reward_request(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        quest_id: u32,
        world: &World,
    ) -> Result<()> {
        let Some(quest) = self.manager.get_quest_template(quest_id) else {
            return Ok(());
        };

        // Alive check: player must be alive, or creature must be visible for dead
        if !self.player_mgr.is_player_alive(player_guid) {
            let creature_visible_for_dead = self
                .creature_mgr
                .get_creature(quest_giver_guid)
                .map(|c| c.unit_flags & 0x00000002 != 0) // TODO: proper IsVisibleForDead flag
                .unwrap_or(false);
            if !creature_visible_for_dead {
                return Ok(());
            }
        }

        if !self.quest_giver_can_start_or_finish(quest_giver_guid, quest_id, world) {
            warn!(
                "Player {:?} tried to request reward for quest {} from invalid quest giver {:?}",
                player_guid, quest_id, quest_giver_guid
            );
            return Ok(());
        }

        // Auto-complete quest if possible (C++: CanCompleteQuest -> CompleteQuest)
        let status = self.get_quest_status(player_guid, quest_id);
        if status != QuestStatus::Complete {
            if self.can_complete_quest(player_guid, &quest, world) {
                self.complete_quest(player_guid, quest_id, &quest);
            } else {
                return Ok(());
            }
        }

        if !self.can_reward_quest(player_guid, &quest, world) {
            return Ok(());
        }

        self.send_offer_reward(player_guid, quest_giver_guid, &quest);
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
            quest_flags: crate::shared::messages::quest::QuestFlags(quest.quest_flags.bits()),
            rew_spell: quest.rew_spell,
            offer_reward_emote: quest.offer_reward_emote,
            offer_reward_emote_delay: quest.offer_reward_emote_delay,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
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

        // Alive check: player must be alive, or creature must be visible for dead
        if !self.player_mgr.is_player_alive(player_guid) {
            let creature_visible_for_dead = self
                .creature_mgr
                .get_creature(quest_giver_guid)
                .map(|c| c.unit_flags & 0x00000002 != 0) // TODO: proper IsVisibleForDead flag
                .unwrap_or(false);
            if !creature_visible_for_dead {
                return Ok(());
            }
        }

        if !self.quest_giver_can_start_or_finish(quest_giver_guid, quest_id, world) {
            warn!(
                "Player {:?} tried to reward quest {} from invalid quest giver {:?}",
                player_guid, quest_id, quest_giver_guid
            );
            return Ok(());
        }

        if self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.rewarded_quests.contains(&quest_id) && !quest.is_repeatable())
            .unwrap_or(true)
        {
            return Ok(());
        }

        if !self.can_reward_quest_with_reward(player_guid, &quest, reward_choice, world) {
            // Re-send offer reward if reward validation fails
            self.send_offer_reward(player_guid, quest_giver_guid, &quest);
            return Ok(());
        }

        if quest.rew_or_req_money < 0 {
            let required_money = (-quest.rew_or_req_money) as u32;
            if !matches!(
                self.inventory.remove_gold(player_guid, required_money),
                GoldResult::Success { .. }
            ) {
                warn!(
                    "Player {:?} failed to pay {} copper for quest {}",
                    player_guid, required_money, quest_id
                );
                return Ok(());
            }
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

                    let available = self
                        .inventory
                        .cache()
                        .get_item(player_guid, item_guid)
                        .map(|item| item.read().count)
                        .unwrap_or(0);
                    let remove_count = remaining.min(available);
                    if remove_count == 0 {
                        continue;
                    }

                    let remove_result =
                        self.inventory
                            .remove_item(player_guid, item_guid, remove_count);
                    match remove_result {
                        crate::world::game::inventory::RemoveItemResult::ItemRemoved { .. } => {
                            remaining = remaining.saturating_sub(remove_count);
                        }
                        crate::world::game::inventory::RemoveItemResult::CountReduced {
                            ..
                        } => {
                            remaining = remaining.saturating_sub(remove_count);
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

        // 2. Give XP reward or max-level gold conversion
        let wow_patch = world.config.wow_patch.unwrap_or(112);
        let rate_drop_money = world.config.rate_drop_money;
        let max_level = crate::shared::game::experience::MAX_PLAYER_LEVEL;

        let xp_reward = if player_level >= max_level {
            // Max level: give gold instead of XP
            let gold_reward = quest.get_rew_money_max_level_at_complete(
                rate_drop_money,
                wow_patch,
                world.config.no_quest_xp_to_gold,
            );
            if gold_reward > 0 {
                self.inventory.add_gold(player_guid, gold_reward);
            }
            0
        } else {
            let xp = quest.xp_value(player_level as u32);
            if xp > 0 {
                use crate::shared::game::experience::XpSource;
                let _ = self
                    .experience
                    .add_xp(player_guid, xp, XpSource::Quest, None, 0.0);
            }
            xp
        };

        // 3. Give money reward (scaled by rate)
        let money = quest.get_reward_or_req_money(rate_drop_money);
        if money > 0 {
            self.inventory.add_gold(player_guid, money as u32);
        }

        // 4. Give reward items (choice + fixed)
        // Give the chosen reward item
        if quest.get_rew_choice_items_count() > 0 {
            let idx = reward_choice as usize;
            if quest.rew_choice_item_id[idx] != 0 && quest.rew_choice_item_count[idx] > 0 {
                let result = self
                    .inventory
                    .add_item(
                        player_guid,
                        quest.rew_choice_item_id[idx],
                        quest.rew_choice_item_count[idx],
                    )
                    .await;
                if !matches!(result, AddItemResult::Success { .. }) {
                    warn!(
                        "Failed to add chosen reward item {} x{} for quest {} to {:?}: {:?}",
                        quest.rew_choice_item_id[idx],
                        quest.rew_choice_item_count[idx],
                        quest_id,
                        player_guid,
                        result
                    );
                    return Ok(());
                }
            }
        }

        // Give fixed reward items
        for i in 0..super::types::QUEST_REWARDS_COUNT {
            if quest.rew_item_id[i] != 0 && quest.rew_item_count[i] > 0 {
                let result = self
                    .inventory
                    .add_item(player_guid, quest.rew_item_id[i], quest.rew_item_count[i])
                    .await;
                if !matches!(result, AddItemResult::Success { .. }) {
                    warn!(
                        "Failed to add fixed reward item {} x{} for quest {} to {:?}: {:?}",
                        quest.rew_item_id[i],
                        quest.rew_item_count[i],
                        quest_id,
                        player_guid,
                        result
                    );
                    return Ok(());
                }
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
                    warn!(
                        "[QUEST] Failed to grant rep for faction {} on quest {}: {}",
                        faction_id, quest_id, e
                    );
                }
            }
        }

        // Remove from active quests
        let rewarded_choice_item_id = if quest.get_rew_choice_items_count() > 0 {
            quest.rew_choice_item_id[reward_choice as usize]
        } else {
            0
        };
        self.player_mgr.with_player_mut(player_guid, |p| {
            if let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id) {
                progress.reward_choice = rewarded_choice_item_id;
                progress.status = QuestStatus::Complete;
                progress.rewarded = true;
                progress.mark_changed();
            }
            p.active_quests.retain(|q| q.quest_id != quest_id);
            p.rewarded_quests.insert(quest_id);
        });

        let rewarded_row = QuestStatusRewardedRow {
            guid: player_guid.counter(),
            quest: quest_id,
            reward_choice: rewarded_choice_item_id,
        };
        let repository = Arc::clone(&self.repository);
        tokio::spawn(async move {
            if let Err(e) = repository.save_rewarded_quest(&rewarded_row).await {
                warn!(
                    "Failed to persist rewarded quest {} for player {:?}: {}",
                    quest_id, player_guid, e
                );
            }
        });

        // Send completion packet
        let msg = SmsgQuestupdateComplete { quest_id };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        // Send gossip complete to close quest window
        let gossip_complete = SmsgGossipComplete;
        self.broadcast_mgr
            .send_msg_to_player(player_guid, gossip_complete);

        // Send quest complete packet with XP info
        let complete_msg = SmsgQuestgiverQuestComplete {
            quest_id,
            xp: xp_reward,
        };
        self.broadcast_mgr
            .send_msg_to_player(player_guid, complete_msg);

        self.send_quest_giver_status(player_guid, quest_giver_guid, world);

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
            if let Some(entry) = self
                .creature_mgr
                .get_creature(quest_giver_guid)
                .map(|c| c.entry)
            {
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
                            self.send_quest_details(
                                player_guid,
                                quest_giver_guid,
                                next_quest_id,
                                world,
                            )?;
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
        let npc_entry = self
            .creature_mgr
            .get_creature(quest_giver_guid)
            .map(|c| c.entry)
            .unwrap_or(0);
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
        let go_entry = world
            .managers
            .gameobject_mgr
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
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        Ok(())
    }

    pub fn send_gameobject_quest_list(
        &self,
        player_guid: ObjectGuid,
        quest_giver_guid: ObjectGuid,
        entry: u32,
        world: &World,
    ) -> Result<()> {
        let quest_items = self.prepare_gameobject_quest_menu(player_guid, entry, world);

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
            title: "Quests",
            emote_delay: 0,
            emote: 0,
            quests: &quests,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
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

        let rewarded_set: std::collections::HashSet<u32> =
            rewarded_rows.into_iter().map(|r| r.quest).collect();
        let mut seen_active = HashSet::new();

        // Map DB rows → QuestProgress, then insert into player
        let mut active_quests: Vec<super::types::QuestProgress> = active_rows
            .into_iter()
            .filter_map(|row| {
                if row.rewarded || rewarded_set.contains(&row.quest) {
                    debug!(
                        "[QUEST] Skipping rewarded quest {} from active login restore for {:?}",
                        row.quest, player_guid
                    );
                    return None;
                }
                if !seen_active.insert(row.quest) {
                    warn!(
                        "[QUEST] Skipping duplicate active quest {} during login restore for {:?}",
                        row.quest, player_guid
                    );
                    return None;
                }
                if !self.manager.has_quest_template(row.quest) {
                    warn!(
                        "[QUEST] Skipping unknown active quest {} during login restore for {:?}",
                        row.quest, player_guid
                    );
                    return None;
                }

                let status = match row.status {
                    0 => super::types::QuestStatus::None,
                    1 => super::types::QuestStatus::Complete,
                    2 => super::types::QuestStatus::Unavailable,
                    3 => super::types::QuestStatus::Incomplete,
                    4 => super::types::QuestStatus::Available,
                    5 => super::types::QuestStatus::Failed,
                    _ => super::types::QuestStatus::Incomplete,
                };
                if !matches!(
                    status,
                    super::types::QuestStatus::Incomplete
                        | super::types::QuestStatus::Complete
                        | super::types::QuestStatus::Failed
                ) {
                    debug!(
                        "[QUEST] Skipping non-log quest {} with status {:?} during login restore for {:?}",
                        row.quest, status, player_guid
                    );
                    return None;
                }
                Some(super::types::QuestProgress {
                    quest_id: row.quest,
                    status,
                    rewarded: row.rewarded,
                    explored: row.explored,
                    timer: row.timer,
                    reward_choice: row.reward_choice,
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
                })
            })
            .take(MAX_QUEST_LOG_SIZE)
            .collect();

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

                let update_result = self
                    .player_mgr
                    .with_player_mut(player_guid, |p| {
                        if let Some(progress) =
                            p.active_quests.iter_mut().find(|q| q.quest_id == quest_id)
                        {
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
                    })
                    .flatten();

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
                    self.broadcast_mgr
                        .send_msg_to_player(player_guid, complete_msg);
                    info!(
                        "[QUEST] Quest {} complete for player {:?}",
                        quest_id, player_guid
                    );
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

        let online_players: Vec<ObjectGuid> =
            world.session_mgr.get_all_sessions().into_iter().collect();

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
                let expired = self
                    .player_mgr
                    .with_player_mut(player_guid, |p| {
                        if let Some(progress) =
                            p.active_quests.iter_mut().find(|q| q.quest_id == quest_id)
                        {
                            progress.timer = new_timer;
                            progress.mark_changed();
                            new_timer == 0
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);

                if expired {
                    // Mark quest as failed
                    self.player_mgr.with_player_mut(player_guid, |p| {
                        if let Some(progress) =
                            p.active_quests.iter_mut().find(|q| q.quest_id == quest_id)
                        {
                            progress.status = super::types::QuestStatus::Failed;
                            progress.mark_changed();
                        }
                    });

                    // Send failure packets
                    let msg = SmsgQuestupdateFailed { quest_id };
                    self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                    let timer_msg = SmsgQuestupdateFailedtimer { quest_id };
                    self.broadcast_mgr
                        .send_msg_to_player(player_guid, timer_msg);

                    info!(
                        "[QUEST] Quest {} timed out for player {:?}",
                        quest_id, player_guid
                    );
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

        let old_len = self
            .player_mgr
            .with_player_mut(player_guid, |p| {
                let old_len = p.active_quests.len();
                if slot < p.active_quests.len() {
                    p.active_quests.remove(slot);
                }
                old_len
            })
            .unwrap_or(0);

        // Clear PLAYER_QUEST_LOG_* update fields so the client removes the quest from UI
        // Each quest slot uses 3 fields: QUEST_ID, COUNT_STATE, TIMER
        const MAX_QUEST_OFFSET: u32 = 3;
        const QUEST_ID_OFFSET: u32 = 0;
        const QUEST_COUNT_STATE_OFFSET: u32 = 1;
        const QUEST_TIME_OFFSET: u32 = 2;

        // Convert world ObjectGuid to world::common ObjectGuid for the message
        let world_guid = ObjectGuid::from_low(player_guid.counter());

        let active_quests = self
            .player_mgr
            .get_player(player_guid)
            .map(|p| p.active_quests.clone())
            .unwrap_or_default();

        let mut values = ValuesUpdateBlock::new(world_guid, ObjectType::Player);
        for slot_idx in slot..old_len {
            let slot_u32 = slot_idx as u32;
            let quest_id_field =
                PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_ID_OFFSET;
            let count_state_field =
                PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_COUNT_STATE_OFFSET;
            let timer_field =
                PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET + QUEST_TIME_OFFSET;

            if let Some(progress) = active_quests.get(slot_idx) {
                values = values
                    .set_field(quest_id_field, progress.quest_id)
                    .set_field(count_state_field, Self::pack_quest_count_state(progress))
                    .set_field(timer_field, progress.timer);
            } else {
                values = values
                    .set_field(quest_id_field, 0)
                    .set_field(count_state_field, 0)
                    .set_field(timer_field, 0);
            }
        }

        let values_update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values));

        self.broadcast_mgr
            .send_msg_to_player(player_guid, values_update);

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
                let update_result = self
                    .player_mgr
                    .with_player_mut(player_guid, |p| {
                        if let Some(progress) =
                            p.active_quests.iter_mut().find(|q| q.quest_id == quest_id)
                        {
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
                                let slot =
                                    p.active_quests.iter().position(|q| q.quest_id == quest_id);

                                Some((new_count, required, all_counts, is_complete, slot))
                            } else {
                                None // Already at max
                            }
                        } else {
                            None
                        }
                    })
                    .flatten();

                let Some((new_count, required_count, all_counts, is_complete, slot)) =
                    update_result
                else {
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

                    let count_state_field = PLAYER_QUEST_LOG_1_1
                        + (slot as u32) * MAX_QUEST_OFFSET
                        + QUEST_COUNT_STATE_OFFSET;

                    // Pack all 4 objective counters into 6-bit fields
                    let mut packed: u32 = 0;
                    for i in 0..QUEST_OBJECTIVES_COUNT {
                        let count = (all_counts[i] as u32).min(63);
                        packed |= count << (i as u32 * 6);
                    }

                    let world_guid = ObjectGuid::from_low(player_guid.counter());
                    let values_update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
                        ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                            .set_field(count_state_field, packed),
                    ));
                    self.broadcast_mgr
                        .send_msg_to_player(player_guid, values_update);
                }

                // If quest is now complete, notify client
                if is_complete {
                    let complete_msg = SmsgQuestupdateComplete { quest_id };
                    self.broadcast_mgr
                        .send_msg_to_player(player_guid, complete_msg);
                    info!(
                        "[QUEST] Quest {} is now complete for player {:?}",
                        quest_id, player_guid
                    );
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
            tracing::debug!(
                "[QUEST] handle_area_event_complete: quest {} not found",
                quest_id
            );
            return;
        };

        let marked = self
            .player_mgr
            .with_player_mut(player_guid, |p| {
                let Some(progress) = p.active_quests.iter_mut().find(|q| q.quest_id == quest_id)
                else {
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
            })
            .unwrap_or(false);

        if marked {
            let complete_msg = crate::shared::messages::quest::SmsgQuestupdateComplete { quest_id };
            self.broadcast_mgr
                .send_msg_to_player(player_guid, complete_msg);
            tracing::info!(
                "[QUEST] Area event: quest {} marked complete for player {:?}",
                quest_id,
                player_guid
            );
        }
    }

    /// Save all active and rewarded quests for a player to the database
    ///
    /// Called during logout to persist quest progress.
    pub async fn save_player_quests(&self, player_guid: ObjectGuid) -> Result<()> {
        use crate::shared::database::characters::models::quest::{
            QuestStatusRewardedRow, QuestStatusRow,
        };

        let guid = player_guid.counter();
        let Some((active_quests, rewarded_quests)) =
            self.player_mgr.with_player(player_guid, |p| {
                (
                    p.active_quests.clone(),
                    p.rewarded_quests.iter().copied().collect::<Vec<_>>(),
                )
            })
        else {
            return Ok(());
        };

        // Save active quests
        for quest in &active_quests {
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
                reward_choice: quest.reward_choice,
            };

            self.repository.save_quest_status(&row).await?;
        }

        // Save rewarded quests
        for quest_id in &rewarded_quests {
            let row = QuestStatusRewardedRow {
                guid,
                quest: *quest_id,
                reward_choice: 0,
            };

            self.repository.save_rewarded_quest(&row).await?;
        }

        debug!(
            "Saved quest data for player {:?}: {} active, {} rewarded",
            player_guid,
            active_quests.len(),
            rewarded_quests.len()
        );

        Ok(())
    }

    /// Load all quests for a player from the database
    ///
    /// Called during login to restore quest progress.
    pub async fn load_player_quests(&self, player_guid: ObjectGuid) -> Result<()> {
        let guid = player_guid.counter();

        // Load rewarded quests first so active restore can filter stale rewarded rows.
        let rewarded = self.repository.find_rewarded_quests(guid).await?;
        let rewarded_set: HashSet<u32> = rewarded.iter().map(|row| row.quest).collect();

        // Load active quests
        let quest_statuses = self.repository.find_quest_statuses(guid).await?;
        let mut seen_active = HashSet::new();
        for row in quest_statuses {
            if row.rewarded || rewarded_set.contains(&row.quest) {
                continue;
            }
            if !seen_active.insert(row.quest) || !self.manager.has_quest_template(row.quest) {
                continue;
            }
            if self
                .player_mgr
                .get_player(player_guid)
                .map(|p| p.active_quests.len() >= MAX_QUEST_LOG_SIZE)
                .unwrap_or(true)
            {
                break;
            }

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
                reward_choice: row.reward_choice,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::database::Databases;
    use crate::shared::protocol::{ObjectGuid, Position};
    use crate::world::config::Config;
    use crate::world::game::creature::ai::NPC_FLAG_QUEST_GIVER;
    use crate::world::game::creature::{Creature, CreatureTemplate};
    use crate::world::game::gameobject::{GameObject, GameObjectTemplate};
    use crate::world::game::player::Player;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });

        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn test_quest(id: u32) -> QuestTemplate {
        QuestTemplate {
            id,
            title: format!("Quest {id}"),
            is_active: true,
            ..QuestTemplate::default()
        }
    }

    fn test_creature_template(entry: u32, npc_flags: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 35,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 7,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn test_gameobject_template(entry: u32) -> GameObjectTemplate {
        GameObjectTemplate {
            entry,
            go_type: 3,
            display_id: 1,
            name: format!("GameObject {entry}"),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data: [0; 24],
        }
    }

    fn add_player(world: &World, player_guid: ObjectGuid, level: u8) {
        let player = Player::new(player_guid, "Tester".to_string(), 0, 0, 1, level, 1, 1, 0);
        world.managers.player_mgr.add_player(player, 1);
    }

    fn add_creature(world: &World, entry: u32, counter: u32, npc_flags: u32) -> ObjectGuid {
        let guid = ObjectGuid::new_creature(entry, counter);
        let template = test_creature_template(entry, npc_flags);
        world.managers.creature_mgr.add_template(template.clone());
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            entry,
            counter,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        ));
        guid
    }

    fn add_gameobject(world: &World, entry: u32, counter: u32) -> ObjectGuid {
        let guid = ObjectGuid::new_gameobject(entry, counter);
        let template = test_gameobject_template(entry);
        world
            .managers
            .gameobject_mgr
            .add_template_for_test(template.clone());
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(GameObject::new(
                guid,
                entry,
                counter,
                Position::default(),
                0,
                &template,
                [0.0; 4],
                1,
                100,
            ));
        guid
    }

    #[tokio::test]
    async fn creature_complete_involved_quest_returns_reward2() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, 0x00000002);
        add_player(&world, player_guid, 10);

        world
            .systems
            .quest
            .manager
            .add_quest_template(test_quest(1));
        world.systems.quest.manager.add_creature_quest_ender(100, 1);
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.active_quests.push(QuestProgress::new(1));
            });

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Reward2)
        );
    }

    #[tokio::test]
    async fn creature_available_start_quest_returns_available() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, 0x00000002);
        add_player(&world, player_guid, 10);

        let mut quest = test_quest(1);
        quest.min_level = 5;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Available)
        );
    }

    #[tokio::test]
    async fn creature_level_locked_start_quest_returns_unavailable() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, 0x00000002);
        add_player(&world, player_guid, 10);

        let mut quest = test_quest(1);
        quest.min_level = 20;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Unavailable)
        );
    }

    #[tokio::test]
    async fn gameobject_questgiver_uses_gameobject_relations() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let gameobject_guid = add_gameobject(&world, 200, 1);
        add_player(&world, player_guid, 10);

        world
            .systems
            .quest
            .manager
            .add_quest_template(test_quest(1));
        world.systems.quest.manager.add_go_quest_starter(200, 1);

        assert_eq!(
            world.systems.quest.get_quest_giver_status_for_guid(
                gameobject_guid,
                player_guid,
                &world
            ),
            Some(DialogStatus::Available)
        );
    }

    #[tokio::test]
    async fn unknown_questgiver_guid_returns_none() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        add_player(&world, player_guid, 10);

        assert_eq!(
            world.systems.quest.get_quest_giver_status_for_guid(
                ObjectGuid::new_creature(999, 1),
                player_guid,
                &world,
            ),
            None
        );
    }

    #[tokio::test]
    async fn low_level_hidden_quest_returns_chat() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, 0x00000002);
        add_player(&world, player_guid, 60);

        // Enable low-level hiding: diff=10 means quests 10+ levels below player show as Chat
        Arc::make_mut(&mut world.config).quest_low_level_hide_diff = 10;

        let mut quest = test_quest(1);
        quest.quest_level = 40; // player is 60, diff = 20 >= 10 → Chat
        quest.min_level = 35;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Chat)
        );
    }

    #[tokio::test]
    async fn low_level_quest_below_hide_threshold_returns_available() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, 0x00000002);
        add_player(&world, player_guid, 60);

        // Enable low-level hiding: diff=10, quest at lv 55 → diff=5 < 10 → still Available
        Arc::make_mut(&mut world.config).quest_low_level_hide_diff = 10;

        let mut quest = test_quest(1);
        quest.quest_level = 55; // player is 60, diff = 5 < 10 → Available
        quest.min_level = 50;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Available)
        );
    }

    #[tokio::test]
    async fn low_level_quest_hide_disabled_returns_available() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, 0x00000002);
        add_player(&world, player_guid, 60);

        // hide_diff defaults to 0 → disabled, should show Available
        let mut quest = test_quest(1);
        quest.quest_level = 1;
        quest.min_level = 1;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Available)
        );
    }

    // --- Script dialog status override tests ---

    /// Register a Lua gossip script table for testing.
    /// The `on_dialog_status` closure receives (script_table, player_table) and returns a u32.
    fn register_test_gossip_script(
        world: &World,
        entry: u32,
        on_dialog_status: impl Fn(mlua::Table, mlua::Table) -> mlua::Result<u32> + Send + 'static,
    ) {
        world.managers.lua_mgr.with_lua(|lua| {
            let table = lua.create_table().unwrap();
            let func = lua
                .create_function(move |_, (t, p): (mlua::Table, mlua::Table)| {
                    on_dialog_status(t, p)
                })
                .unwrap();
            table.set("OnDialogStatus", func).unwrap();
            world
                .managers
                .lua_mgr
                .register_gossip_script_table(entry, table);
            Ok::<_, mlua::Error>(())
        });
    }

    #[tokio::test]
    async fn script_dialog_status_overrides_to_none() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Register a script that returns None (0) – should override even though quest is available
        let mut quest = test_quest(1);
        quest.quest_level = 1;
        quest.min_level = 1;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        register_test_gossip_script(&world, 100, |_, _| Ok(0u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::None)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_overrides_to_available() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // No quests registered – relation status would be None, but script says Available (5)
        register_test_gossip_script(&world, 100, |_, _| Ok(5u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Available)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_overrides_to_unavailable() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Register available quest + script that says Unavailable (1) – script wins
        let mut quest = test_quest(1);
        quest.quest_level = 1;
        quest.min_level = 1;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        register_test_gossip_script(&world, 100, |_, _| Ok(1u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Unavailable)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_fallback_on_greater_than_7() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Script returns 99 (>7) – should fall through to relation status
        let mut quest = test_quest(1);
        quest.quest_level = 1;
        quest.min_level = 1;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        register_test_gossip_script(&world, 100, |_, _| Ok(99u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Available)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_script_without_callback_does_not_override() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Register a gossip script table WITHOUT OnDialogStatus – should not override
        world.managers.lua_mgr.with_lua(|lua| {
            let table = lua.create_table().unwrap();
            // No OnDialogStatus set
            world
                .managers
                .lua_mgr
                .register_gossip_script_table(100, table);
            Ok::<_, mlua::Error>(())
        });

        let mut quest = test_quest(1);
        quest.quest_level = 1;
        quest.min_level = 1;
        world.systems.quest.manager.add_quest_template(quest);
        world
            .systems
            .quest
            .manager
            .add_creature_quest_starter(100, 1);

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Available)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_reward2_via_script() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Script returns Reward2 (7) – highest status
        register_test_gossip_script(&world, 100, |_, _| Ok(7u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Reward2)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_reward_rep_via_script() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Script returns RewardRep (4)
        register_test_gossip_script(&world, 100, |_, _| Ok(4u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::RewardRep)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_incomplete_via_script() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Script returns Incomplete (3)
        register_test_gossip_script(&world, 100, |_, _| Ok(3u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Incomplete)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_chat_via_script() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Script returns Chat (2)
        register_test_gossip_script(&world, 100, |_, _| Ok(2u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::Chat)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_reward_old_via_script() {
        let mut world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // Script returns RewardOld (6)
        register_test_gossip_script(&world, 100, |_, _| Ok(6u32));

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::RewardOld)
        );
    }

    #[tokio::test]
    async fn script_dialog_status_no_script_does_not_override() {
        let world = test_world();
        let player_guid = ObjectGuid::new_player(1);
        let creature_guid = add_creature(&world, 100, 1, NPC_FLAG_QUEST_GIVER);
        add_player(&world, player_guid, 60);

        // No script registered for entry 100 – should use relation status (None with no quests)

        assert_eq!(
            world
                .systems
                .quest
                .get_quest_giver_status_for_guid(creature_guid, player_guid, &world),
            Some(DialogStatus::None)
        );
    }
}
