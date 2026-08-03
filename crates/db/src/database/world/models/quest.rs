//! Quest models for world database
//!
//! Database row structures for quest_template and quest relation tables.

use sqlx::FromRow;

/// Row from quest_template table
#[derive(FromRow, Debug, Clone)]
pub struct QuestTemplateRow {
    pub entry: i64,
    #[sqlx(rename = "Method")]
    pub method: i16,
    #[sqlx(rename = "ZoneOrSort")]
    pub zone_or_sort: i16,
    #[sqlx(rename = "MinLevel")]
    pub min_level: i16,
    #[sqlx(rename = "MaxLevel")]
    pub max_level: i16,
    #[sqlx(rename = "QuestLevel")]
    pub quest_level: i16,
    #[sqlx(rename = "Type")]
    pub quest_type: i32,
    #[sqlx(rename = "RequiredClasses")]
    pub required_classes: i32,
    #[sqlx(rename = "RequiredRaces")]
    pub required_races: i32,
    #[sqlx(rename = "RequiredSkill")]
    pub required_skill: i32,
    #[sqlx(rename = "RequiredSkillValue")]
    pub required_skill_value: i32,
    #[sqlx(rename = "RequiredCondition")]
    pub required_condition: i64,
    #[sqlx(rename = "RepObjectiveFaction")]
    pub rep_objective_faction: i32,
    #[sqlx(rename = "RepObjectiveValue")]
    pub rep_objective_value: i32,
    #[sqlx(rename = "RequiredMinRepFaction")]
    pub required_min_rep_faction: i32,
    #[sqlx(rename = "RequiredMinRepValue")]
    pub required_min_rep_value: i32,
    #[sqlx(rename = "RequiredMaxRepFaction")]
    pub required_max_rep_faction: i32,
    #[sqlx(rename = "RequiredMaxRepValue")]
    pub required_max_rep_value: i32,
    #[sqlx(rename = "SuggestedPlayers")]
    pub suggested_players: i16,
    #[sqlx(rename = "LimitTime")]
    pub limit_time: i64,
    #[sqlx(rename = "QuestFlags")]
    pub quest_flags: i32,
    #[sqlx(rename = "SpecialFlags")]
    pub special_flags: i16,
    #[sqlx(rename = "PrevQuestId")]
    pub prev_quest_id: i32,
    #[sqlx(rename = "NextQuestId")]
    pub next_quest_id: i32,
    #[sqlx(rename = "ExclusiveGroup")]
    pub exclusive_group: i32,
    #[sqlx(rename = "BreadcrumbForQuestId")]
    pub breadcrumb_for_quest_id: i64,
    #[sqlx(rename = "NextQuestInChain")]
    pub next_quest_in_chain: i64,
    #[sqlx(rename = "SrcItemId")]
    pub src_item_id: i64,
    #[sqlx(rename = "SrcItemCount")]
    pub src_item_count: i16,
    #[sqlx(rename = "SrcSpell")]
    pub src_spell: i32,
    #[sqlx(rename = "Title")]
    pub title: Option<String>,
    #[sqlx(rename = "Details")]
    pub details: Option<String>,
    #[sqlx(rename = "Objectives")]
    pub objectives: Option<String>,
    #[sqlx(rename = "OfferRewardText")]
    pub offer_reward_text: Option<String>,
    #[sqlx(rename = "RequestItemsText")]
    pub request_items_text: Option<String>,
    #[sqlx(rename = "EndText")]
    pub end_text: Option<String>,
    #[sqlx(rename = "RewXP")]
    pub rew_xp: i64,
    #[sqlx(rename = "RewOrReqMoney")]
    pub rew_or_req_money: i32,
    #[sqlx(rename = "RewMoneyMaxLevel")]
    pub rew_money_max_level: i64,
    #[sqlx(rename = "RewSpell")]
    pub rew_spell: i32,
    #[sqlx(rename = "RewSpellCast")]
    pub rew_spell_cast: i32,
    #[sqlx(rename = "RewMailTemplateId")]
    pub rew_mail_template_id: i32,
    #[sqlx(rename = "RewMailDelaySecs")]
    pub rew_mail_delay_secs: i64,
    #[sqlx(rename = "RewMailMoney")]
    pub rew_mail_money: i64,
    #[sqlx(rename = "PointMapId")]
    pub point_map_id: i32,
    #[sqlx(rename = "PointX")]
    pub point_x: f32,
    #[sqlx(rename = "PointY")]
    pub point_y: f32,
    #[sqlx(rename = "PointOpt")]
    pub point_opt: i64,
    #[sqlx(rename = "IncompleteEmote")]
    pub incomplete_emote: i32,
    #[sqlx(rename = "CompleteEmote")]
    pub complete_emote: i32,
    #[sqlx(rename = "RewRepSpilloverMask")]
    pub rew_rep_spillover_mask: i16,

    // Objective text fields (1-4)
    #[sqlx(rename = "ObjectiveText1")]
    pub objective_text1: Option<String>,
    #[sqlx(rename = "ObjectiveText2")]
    pub objective_text2: Option<String>,
    #[sqlx(rename = "ObjectiveText3")]
    pub objective_text3: Option<String>,
    #[sqlx(rename = "ObjectiveText4")]
    pub objective_text4: Option<String>,

    // Required item fields (1-4)
    #[sqlx(rename = "ReqItemId1")]
    pub req_item_id1: Option<i64>,
    #[sqlx(rename = "ReqItemId2")]
    pub req_item_id2: Option<i64>,
    #[sqlx(rename = "ReqItemId3")]
    pub req_item_id3: Option<i64>,
    #[sqlx(rename = "ReqItemId4")]
    pub req_item_id4: Option<i64>,
    #[sqlx(rename = "ReqItemCount1")]
    pub req_item_count1: Option<i32>,
    #[sqlx(rename = "ReqItemCount2")]
    pub req_item_count2: Option<i32>,
    #[sqlx(rename = "ReqItemCount3")]
    pub req_item_count3: Option<i32>,
    #[sqlx(rename = "ReqItemCount4")]
    pub req_item_count4: Option<i32>,

    // Required creature/GO fields (1-4)
    #[sqlx(rename = "ReqCreatureOrGOId1")]
    pub req_creature_or_go_id1: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOId2")]
    pub req_creature_or_go_id2: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOId3")]
    pub req_creature_or_go_id3: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOId4")]
    pub req_creature_or_go_id4: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOCount1")]
    pub req_creature_or_go_count1: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOCount2")]
    pub req_creature_or_go_count2: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOCount3")]
    pub req_creature_or_go_count3: Option<i32>,
    #[sqlx(rename = "ReqCreatureOrGOCount4")]
    pub req_creature_or_go_count4: Option<i32>,

    // Reward choice item fields (1-6)
    #[sqlx(rename = "RewChoiceItemId1")]
    pub rew_choice_item_id1: Option<i64>,
    #[sqlx(rename = "RewChoiceItemId2")]
    pub rew_choice_item_id2: Option<i64>,
    #[sqlx(rename = "RewChoiceItemId3")]
    pub rew_choice_item_id3: Option<i64>,
    #[sqlx(rename = "RewChoiceItemId4")]
    pub rew_choice_item_id4: Option<i64>,
    #[sqlx(rename = "RewChoiceItemId5")]
    pub rew_choice_item_id5: Option<i64>,
    #[sqlx(rename = "RewChoiceItemId6")]
    pub rew_choice_item_id6: Option<i64>,
    #[sqlx(rename = "RewChoiceItemCount1")]
    pub rew_choice_item_count1: Option<i32>,
    #[sqlx(rename = "RewChoiceItemCount2")]
    pub rew_choice_item_count2: Option<i32>,
    #[sqlx(rename = "RewChoiceItemCount3")]
    pub rew_choice_item_count3: Option<i32>,
    #[sqlx(rename = "RewChoiceItemCount4")]
    pub rew_choice_item_count4: Option<i32>,
    #[sqlx(rename = "RewChoiceItemCount5")]
    pub rew_choice_item_count5: Option<i32>,
    #[sqlx(rename = "RewChoiceItemCount6")]
    pub rew_choice_item_count6: Option<i32>,

    // Fixed reward item fields (1-4)
    #[sqlx(rename = "RewItemId1")]
    pub rew_item_id1: Option<i64>,
    #[sqlx(rename = "RewItemId2")]
    pub rew_item_id2: Option<i64>,
    #[sqlx(rename = "RewItemId3")]
    pub rew_item_id3: Option<i64>,
    #[sqlx(rename = "RewItemId4")]
    pub rew_item_id4: Option<i64>,
    #[sqlx(rename = "RewItemCount1")]
    pub rew_item_count1: Option<i32>,
    #[sqlx(rename = "RewItemCount2")]
    pub rew_item_count2: Option<i32>,
    #[sqlx(rename = "RewItemCount3")]
    pub rew_item_count3: Option<i32>,
    #[sqlx(rename = "RewItemCount4")]
    pub rew_item_count4: Option<i32>,

    // Reputation reward fields (1-5)
    #[sqlx(rename = "RewRepFaction1")]
    pub rew_rep_faction1: Option<i32>,
    #[sqlx(rename = "RewRepFaction2")]
    pub rew_rep_faction2: Option<i32>,
    #[sqlx(rename = "RewRepFaction3")]
    pub rew_rep_faction3: Option<i32>,
    #[sqlx(rename = "RewRepFaction4")]
    pub rew_rep_faction4: Option<i32>,
    #[sqlx(rename = "RewRepFaction5")]
    pub rew_rep_faction5: Option<i32>,
    #[sqlx(rename = "RewRepValue1")]
    pub rew_rep_value1: Option<i32>,
    #[sqlx(rename = "RewRepValue2")]
    pub rew_rep_value2: Option<i32>,
    #[sqlx(rename = "RewRepValue3")]
    pub rew_rep_value3: Option<i32>,
    #[sqlx(rename = "RewRepValue4")]
    pub rew_rep_value4: Option<i32>,
    #[sqlx(rename = "RewRepValue5")]
    pub rew_rep_value5: Option<i32>,

    // Details emote fields (1-4)
    #[sqlx(rename = "DetailsEmote1")]
    pub details_emote1: Option<i32>,
    #[sqlx(rename = "DetailsEmote2")]
    pub details_emote2: Option<i32>,
    #[sqlx(rename = "DetailsEmote3")]
    pub details_emote3: Option<i32>,
    #[sqlx(rename = "DetailsEmote4")]
    pub details_emote4: Option<i32>,
    #[sqlx(rename = "DetailsEmoteDelay1")]
    pub details_emote_delay1: Option<i64>,
    #[sqlx(rename = "DetailsEmoteDelay2")]
    pub details_emote_delay2: Option<i64>,
    #[sqlx(rename = "DetailsEmoteDelay3")]
    pub details_emote_delay3: Option<i64>,
    #[sqlx(rename = "DetailsEmoteDelay4")]
    pub details_emote_delay4: Option<i64>,

    // Offer reward emote fields (1-4)
    #[sqlx(rename = "OfferRewardEmote1")]
    pub offer_reward_emote1: Option<i32>,
    #[sqlx(rename = "OfferRewardEmote2")]
    pub offer_reward_emote2: Option<i32>,
    #[sqlx(rename = "OfferRewardEmote3")]
    pub offer_reward_emote3: Option<i32>,
    #[sqlx(rename = "OfferRewardEmote4")]
    pub offer_reward_emote4: Option<i32>,
    #[sqlx(rename = "OfferRewardEmoteDelay1")]
    pub offer_reward_emote_delay1: Option<i64>,
    #[sqlx(rename = "OfferRewardEmoteDelay2")]
    pub offer_reward_emote_delay2: Option<i64>,
    #[sqlx(rename = "OfferRewardEmoteDelay3")]
    pub offer_reward_emote_delay3: Option<i64>,
    #[sqlx(rename = "OfferRewardEmoteDelay4")]
    pub offer_reward_emote_delay4: Option<i64>,
}

/// Row from creature_questrelation table (quest starters)
#[derive(FromRow, Debug, Clone)]
pub struct CreatureQuestRelationRow {
    pub id: i64,
    pub quest: i64,
}

/// Row from creature_involvedrelation table (quest enders)
#[derive(FromRow, Debug, Clone)]
pub struct CreatureInvolvedRelationRow {
    pub id: i64,
    pub quest: i64,
}

/// Row from gameobject_questrelation table (GO quest starters)
#[derive(FromRow, Debug, Clone)]
pub struct GameObjectQuestRelationRow {
    pub id: i64,
    pub quest: i64,
}

/// Row from gameobject_involvedrelation table (GO quest enders)
#[derive(FromRow, Debug, Clone)]
pub struct GameObjectInvolvedRelationRow {
    pub id: i64,
    pub quest: i64,
}
