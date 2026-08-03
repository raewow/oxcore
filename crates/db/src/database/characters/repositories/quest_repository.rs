use super::super::models::quest::*;
use super::super::{PgQuestRepository, PgQuestStatusRow};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[async_trait]
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait QuestRepositoryTrait: Send + Sync {
    async fn find_quest_statuses(&self, guid: u32) -> Result<Vec<QuestStatusRow>>;
    async fn find_rewarded_quests(&self, guid: u32) -> Result<Vec<QuestStatusRewardedRow>>;
    async fn find_quest_status(&self, guid: u32, quest_id: u32) -> Result<Option<QuestStatusRow>>;
    async fn has_completed_quest(&self, guid: u32, quest_id: u32) -> Result<bool>;
    async fn save_quest_status(&self, quest_status: &QuestStatusRow) -> Result<()>;
    async fn save_rewarded_quest(&self, quest_rewarded: &QuestStatusRewardedRow) -> Result<()>;
    async fn delete_quest_status(&self, guid: u32, quest_id: u32) -> Result<()>;
    async fn delete_all_quest_statuses(&self, guid: u32) -> Result<()>;
    async fn delete_all_rewarded_quests(&self, guid: u32) -> Result<()>;
}

pub struct QuestRepository {
    pool: Arc<PgPool>,
}

impl QuestRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgQuestRepository {
        PgQuestRepository::new(Arc::clone(&self.pool))
    }
    fn row(row: PgQuestStatusRow) -> Result<QuestStatusRow> {
        Ok(QuestStatusRow {
            guid: row.guid.try_into()?,
            quest: row.quest.try_into()?,
            status: row.status.try_into()?,
            rewarded: row.rewarded,
            explored: row.explored,
            timer: row.timer.try_into()?,
            mob_count1: row.mob_count1.try_into()?,
            mob_count2: row.mob_count2.try_into()?,
            mob_count3: row.mob_count3.try_into()?,
            mob_count4: row.mob_count4.try_into()?,
            item_count1: row.item_count1.try_into()?,
            item_count2: row.item_count2.try_into()?,
            item_count3: row.item_count3.try_into()?,
            item_count4: row.item_count4.try_into()?,
            reward_choice: row.reward_choice.try_into()?,
        })
    }
    fn dto(row: &QuestStatusRow) -> PgQuestStatusRow {
        PgQuestStatusRow {
            guid: row.guid.into(),
            quest: row.quest.into(),
            status: row.status.into(),
            rewarded: row.rewarded,
            explored: row.explored,
            timer: row.timer.into(),
            mob_count1: row.mob_count1.into(),
            mob_count2: row.mob_count2.into(),
            mob_count3: row.mob_count3.into(),
            mob_count4: row.mob_count4.into(),
            item_count1: row.item_count1.into(),
            item_count2: row.item_count2.into(),
            item_count3: row.item_count3.into(),
            item_count4: row.item_count4.into(),
            reward_choice: row.reward_choice.into(),
        }
    }
}

#[async_trait]
impl QuestRepositoryTrait for QuestRepository {
    async fn find_quest_statuses(&self, guid: u32) -> Result<Vec<QuestStatusRow>> {
        self.pg()
            .load(guid.into())
            .await?
            .into_iter()
            .map(Self::row)
            .collect()
    }
    async fn find_rewarded_quests(&self, guid: u32) -> Result<Vec<QuestStatusRewardedRow>> {
        self.pg()
            .find_rewarded(guid.into())
            .await?
            .into_iter()
            .map(|row| {
                Ok(QuestStatusRewardedRow {
                    guid: row.guid.try_into()?,
                    quest: row.quest.try_into()?,
                    reward_choice: row.reward_choice.try_into()?,
                })
            })
            .collect()
    }
    async fn find_quest_status(&self, guid: u32, quest_id: u32) -> Result<Option<QuestStatusRow>> {
        self.pg()
            .find(guid.into(), quest_id.into())
            .await?
            .map(Self::row)
            .transpose()
    }
    async fn has_completed_quest(&self, guid: u32, quest_id: u32) -> Result<bool> {
        Ok(self
            .pg()
            .find(guid.into(), quest_id.into())
            .await?
            .is_some_and(|row| row.rewarded))
    }
    async fn save_quest_status(&self, row: &QuestStatusRow) -> Result<()> {
        self.pg().save(&Self::dto(row)).await
    }
    async fn save_rewarded_quest(&self, row: &QuestStatusRewardedRow) -> Result<()> {
        self.pg()
            .save(&PgQuestStatusRow {
                guid: row.guid.into(),
                quest: row.quest.into(),
                status: 1,
                rewarded: true,
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
                reward_choice: row.reward_choice.into(),
            })
            .await
    }
    async fn delete_quest_status(&self, guid: u32, quest_id: u32) -> Result<()> {
        self.pg().delete(guid.into(), quest_id.into()).await
    }
    async fn delete_all_quest_statuses(&self, guid: u32) -> Result<()> {
        self.pg().delete_all(guid.into(), false).await
    }
    async fn delete_all_rewarded_quests(&self, guid: u32) -> Result<()> {
        self.pg().delete_all(guid.into(), true).await
    }
}
