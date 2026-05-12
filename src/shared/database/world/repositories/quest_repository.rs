//! Quest repository for world database
//!
//! Loads quest templates and quest relations from the world database.

use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

use crate::shared::database::world::models::quest::*;
use crate::world::game::npc::quest::{
    QuestFlags, QuestManager, QuestMethod, QuestSpecialFlags, QuestTemplate, QuestType,
};

/// Repository for loading quest data from world database
pub struct QuestTemplateRepository;

impl QuestTemplateRepository {
    /// Load all quest data from database
    pub async fn load(manager: &QuestManager, world_db: &MySqlPool) -> Result<()> {
        Self::load_quest_templates(manager, world_db).await?;
        Self::load_creature_quest_relations(manager, world_db).await?;
        Self::load_creature_involved_relations(manager, world_db).await?;
        Self::load_go_quest_relations(manager, world_db).await?;
        Self::load_go_involved_relations(manager, world_db).await?;

        Ok(())
    }

    /// Load quest templates from quest_template table
    /// Filters for highest patch per entry in code to avoid slow subquery with large text columns
    async fn load_quest_templates(manager: &QuestManager, world_db: &MySqlPool) -> Result<()> {
        // Use a simpler query without subquery - filter for max patch in code
        // This avoids performance issues with subqueries on tables with large text columns
        let rows = sqlx::query_as::<_, QuestTemplateRow>(
            r#"SELECT entry, Method, ZoneOrSort, MinLevel, MaxLevel, QuestLevel, Type,
                       RequiredClasses, RequiredRaces, RequiredSkill, RequiredSkillValue,
                       RequiredCondition, RepObjectiveFaction, RepObjectiveValue,
                       RequiredMinRepFaction, RequiredMinRepValue, RequiredMaxRepFaction, RequiredMaxRepValue,
                       SuggestedPlayers, LimitTime, QuestFlags, SpecialFlags,
                       PrevQuestId, NextQuestId, ExclusiveGroup, BreadcrumbForQuestId, NextQuestInChain,
                       SrcItemId, SrcItemCount, SrcSpell,
                       Title, Details, Objectives, OfferRewardText, RequestItemsText, EndText,
                       RewXP, RewOrReqMoney, RewMoneyMaxLevel, RewSpell, RewSpellCast,
                       RewMailTemplateId, RewMailDelaySecs, RewMailMoney,
                       PointMapId, PointX, PointY, PointOpt,
                       IncompleteEmote, CompleteEmote, RewRepSpilloverMask,
                       ObjectiveText1, ObjectiveText2, ObjectiveText3, ObjectiveText4,
                       ReqItemId1, ReqItemId2, ReqItemId3, ReqItemId4,
                       ReqItemCount1, ReqItemCount2, ReqItemCount3, ReqItemCount4,
                       ReqCreatureOrGOId1, ReqCreatureOrGOId2, ReqCreatureOrGOId3, ReqCreatureOrGOId4,
                       ReqCreatureOrGOCount1, ReqCreatureOrGOCount2, ReqCreatureOrGOCount3, ReqCreatureOrGOCount4,
                       RewChoiceItemId1, RewChoiceItemId2, RewChoiceItemId3, RewChoiceItemId4, RewChoiceItemId5, RewChoiceItemId6,
                       RewChoiceItemCount1, RewChoiceItemCount2, RewChoiceItemCount3, RewChoiceItemCount4, RewChoiceItemCount5, RewChoiceItemCount6,
                       RewItemId1, RewItemId2, RewItemId3, RewItemId4,
                       RewItemCount1, RewItemCount2, RewItemCount3, RewItemCount4,
                       RewRepFaction1, RewRepFaction2, RewRepFaction3, RewRepFaction4, RewRepFaction5,
                       RewRepValue1, RewRepValue2, RewRepValue3, RewRepValue4, RewRepValue5,
                       DetailsEmote1, DetailsEmote2, DetailsEmote3, DetailsEmote4,
                       DetailsEmoteDelay1, DetailsEmoteDelay2, DetailsEmoteDelay3, DetailsEmoteDelay4,
                       OfferRewardEmote1, OfferRewardEmote2, OfferRewardEmote3, OfferRewardEmote4,
                       OfferRewardEmoteDelay1, OfferRewardEmoteDelay2, OfferRewardEmoteDelay3, OfferRewardEmoteDelay4
                FROM quest_template
                WHERE patch <= 10
                ORDER BY entry, patch DESC"#
        )
        .fetch_all(world_db)
        .await
        .context("Failed to fetch quest templates")?;

        // Keep only the highest patch for each entry
        let mut last_entry: Option<u32> = None;
        for row in rows {
            // Skip duplicate entries (we ordered by entry, patch DESC so first one is highest patch)
            if last_entry == Some(row.entry) {
                continue;
            }
            last_entry = Some(row.entry);
            
            let template = Self::row_to_template(row);
            manager.add_quest_template(template);
        }

        tracing::info!(
            "Loaded {} quest templates",
            manager.quest_template_count()
        );

        Ok(())
    }

    /// Convert database row to QuestTemplate
    fn row_to_template(row: QuestTemplateRow) -> QuestTemplate {
        let method = match row.method {
            0 => QuestMethod::AutoComplete,
            1 => QuestMethod::Disabled,
            _ => QuestMethod::Deliver,
        };

        let quest_type = match row.quest_type {
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
        };

        let is_active = method != QuestMethod::Disabled;

        QuestTemplate {
            id: row.entry,
            method,
            zone_or_sort: row.zone_or_sort,
            min_level: row.min_level,
            max_level: row.max_level,
            quest_level: row.quest_level,
            quest_type,
            required_classes: row.required_classes,
            required_races: row.required_races,
            required_skill: row.required_skill,
            required_skill_value: row.required_skill_value,
            required_condition: row.required_condition,
            rep_objective_faction: row.rep_objective_faction,
            rep_objective_value: row.rep_objective_value,
            required_min_rep_faction: row.required_min_rep_faction,
            required_min_rep_value: row.required_min_rep_value,
            required_max_rep_faction: row.required_max_rep_faction,
            required_max_rep_value: row.required_max_rep_value,
            prev_quest_id: row.prev_quest_id,
            next_quest_id: row.next_quest_id,
            exclusive_group: row.exclusive_group,
            breadcrumb_for_quest_id: row.breadcrumb_for_quest_id as i32,
            next_quest_in_chain: row.next_quest_in_chain,
            src_item_id: row.src_item_id,
            src_item_count: row.src_item_count,
            src_spell: row.src_spell,
            req_item_id: [
                row.req_item_id1.unwrap_or(0),
                row.req_item_id2.unwrap_or(0),
                row.req_item_id3.unwrap_or(0),
                row.req_item_id4.unwrap_or(0),
            ],
            req_item_count: [
                row.req_item_count1.unwrap_or(0),
                row.req_item_count2.unwrap_or(0),
                row.req_item_count3.unwrap_or(0),
                row.req_item_count4.unwrap_or(0),
            ],
            req_source_id: [0; 4], // TODO: Load if needed
            req_source_count: [0; 4],
            req_creature_or_go_id: [
                row.req_creature_or_go_id1.unwrap_or(0),
                row.req_creature_or_go_id2.unwrap_or(0),
                row.req_creature_or_go_id3.unwrap_or(0),
                row.req_creature_or_go_id4.unwrap_or(0),
            ],
            req_creature_or_go_count: [
                row.req_creature_or_go_count1.unwrap_or(0),
                row.req_creature_or_go_count2.unwrap_or(0),
                row.req_creature_or_go_count3.unwrap_or(0),
                row.req_creature_or_go_count4.unwrap_or(0),
            ],
            req_spell: [0; 4], // TODO: Load if needed
            rew_choice_item_id: [
                row.rew_choice_item_id1.unwrap_or(0),
                row.rew_choice_item_id2.unwrap_or(0),
                row.rew_choice_item_id3.unwrap_or(0),
                row.rew_choice_item_id4.unwrap_or(0),
                row.rew_choice_item_id5.unwrap_or(0),
                row.rew_choice_item_id6.unwrap_or(0),
            ],
            rew_choice_item_count: [
                row.rew_choice_item_count1.unwrap_or(0),
                row.rew_choice_item_count2.unwrap_or(0),
                row.rew_choice_item_count3.unwrap_or(0),
                row.rew_choice_item_count4.unwrap_or(0),
                row.rew_choice_item_count5.unwrap_or(0),
                row.rew_choice_item_count6.unwrap_or(0),
            ],
            rew_item_id: [
                row.rew_item_id1.unwrap_or(0),
                row.rew_item_id2.unwrap_or(0),
                row.rew_item_id3.unwrap_or(0),
                row.rew_item_id4.unwrap_or(0),
            ],
            rew_item_count: [
                row.rew_item_count1.unwrap_or(0),
                row.rew_item_count2.unwrap_or(0),
                row.rew_item_count3.unwrap_or(0),
                row.rew_item_count4.unwrap_or(0),
            ],
            rew_rep_faction: [
                row.rew_rep_faction1.unwrap_or(0),
                row.rew_rep_faction2.unwrap_or(0),
                row.rew_rep_faction3.unwrap_or(0),
                row.rew_rep_faction4.unwrap_or(0),
                row.rew_rep_faction5.unwrap_or(0),
            ],
            rew_rep_value: [
                row.rew_rep_value1.unwrap_or(0),
                row.rew_rep_value2.unwrap_or(0),
                row.rew_rep_value3.unwrap_or(0),
                row.rew_rep_value4.unwrap_or(0),
                row.rew_rep_value5.unwrap_or(0),
            ],
            rew_rep_spillover_mask: row.rew_rep_spillover_mask,
            rew_xp: row.rew_xp,
            rew_or_req_money: row.rew_or_req_money,
            rew_money_max_level: row.rew_money_max_level,
            rew_spell: row.rew_spell,
            rew_spell_cast: row.rew_spell_cast,
            rew_mail_template_id: row.rew_mail_template_id,
            rew_mail_delay_secs: row.rew_mail_delay_secs,
            rew_mail_money: row.rew_mail_money,
            point_map_id: row.point_map_id,
            point_x: row.point_x,
            point_y: row.point_y,
            point_opt: row.point_opt,
            quest_flags: QuestFlags::from_bits_truncate(row.quest_flags),
            special_flags: QuestSpecialFlags::from_bits_truncate(row.special_flags),
            suggested_players: row.suggested_players,
            limit_time: row.limit_time,
            title: row.title.unwrap_or_default(),
            details: row.details.unwrap_or_default(),
            objectives: row.objectives.unwrap_or_default(),
            offer_reward_text: row.offer_reward_text.unwrap_or_default(),
            request_items_text: row.request_items_text.unwrap_or_default(),
            end_text: row.end_text.unwrap_or_default(),
            objective_text: [
                row.objective_text1.unwrap_or_default(),
                row.objective_text2.unwrap_or_default(),
                row.objective_text3.unwrap_or_default(),
                row.objective_text4.unwrap_or_default(),
            ],
            details_emote: [
                row.details_emote1.unwrap_or(0),
                row.details_emote2.unwrap_or(0),
                row.details_emote3.unwrap_or(0),
                row.details_emote4.unwrap_or(0),
            ],
            details_emote_delay: [
                row.details_emote_delay1.unwrap_or(0),
                row.details_emote_delay2.unwrap_or(0),
                row.details_emote_delay3.unwrap_or(0),
                row.details_emote_delay4.unwrap_or(0),
            ],
            incomplete_emote: row.incomplete_emote,
            complete_emote: row.complete_emote,
            offer_reward_emote: [
                row.offer_reward_emote1.unwrap_or(0),
                row.offer_reward_emote2.unwrap_or(0),
                row.offer_reward_emote3.unwrap_or(0),
                row.offer_reward_emote4.unwrap_or(0),
            ],
            offer_reward_emote_delay: [
                row.offer_reward_emote_delay1.unwrap_or(0),
                row.offer_reward_emote_delay2.unwrap_or(0),
                row.offer_reward_emote_delay3.unwrap_or(0),
                row.offer_reward_emote_delay4.unwrap_or(0),
            ],
            start_script: 0, // TODO: Load if column exists
            complete_script: 0,
            is_active,
        }
    }

    /// Load creature quest starters from creature_questrelation table
    async fn load_creature_quest_relations(
        manager: &QuestManager,
        world_db: &MySqlPool,
    ) -> Result<()> {
        let rows = sqlx::query_as::<_, CreatureQuestRelationRow>(
            "SELECT id, quest FROM creature_questrelation"
        )
        .fetch_all(world_db)
        .await
        .context("Failed to fetch creature quest relations")?;

        for row in rows {
            manager.add_creature_quest_starter(row.id, row.quest);
        }

        tracing::info!(
            "Loaded {} creature quest starters",
            manager.creature_starter_count()
        );

        Ok(())
    }

    /// Load creature quest enders from creature_involvedrelation table
    async fn load_creature_involved_relations(
        manager: &QuestManager,
        world_db: &MySqlPool,
    ) -> Result<()> {
        let rows = sqlx::query_as::<_, CreatureInvolvedRelationRow>(
            "SELECT id, quest FROM creature_involvedrelation"
        )
        .fetch_all(world_db)
        .await
        .context("Failed to fetch creature involved relations")?;

        for row in rows {
            manager.add_creature_quest_ender(row.id, row.quest);
        }

        tracing::info!(
            "Loaded {} creature quest enders",
            manager.creature_ender_count()
        );

        Ok(())
    }

    /// Load GameObject quest starters from gameobject_questrelation table
    async fn load_go_quest_relations(manager: &QuestManager, world_db: &MySqlPool) -> Result<()> {
        let rows = sqlx::query_as::<_, GameObjectQuestRelationRow>(
            "SELECT id, quest FROM gameobject_questrelation"
        )
        .fetch_all(world_db)
        .await
        .context("Failed to fetch GO quest relations")?;

        for row in rows {
            manager.add_go_quest_starter(row.id, row.quest);
        }

        Ok(())
    }

    /// Load GameObject quest enders from gameobject_involvedrelation table
    async fn load_go_involved_relations(
        manager: &QuestManager,
        world_db: &MySqlPool,
    ) -> Result<()> {
        let rows = sqlx::query_as::<_, GameObjectInvolvedRelationRow>(
            "SELECT id, quest FROM gameobject_involvedrelation"
        )
        .fetch_all(world_db)
        .await
        .context("Failed to fetch GO involved relations")?;

        for row in rows {
            manager.add_go_quest_ender(row.id, row.quest);
        }

        Ok(())
    }
}