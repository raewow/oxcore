use super::super::models::quest::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

/// Trait for quest repository operations
/// This enables mocking for unit tests
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait QuestRepositoryTrait: Send + Sync {
    // Query methods
    async fn find_quest_statuses(&self, guid: u32) -> Result<Vec<QuestStatusRow>>;
    async fn find_rewarded_quests(&self, guid: u32) -> Result<Vec<QuestStatusRewardedRow>>;
    async fn find_quest_status(&self, guid: u32, quest_id: u32) -> Result<Option<QuestStatusRow>>;
    async fn has_completed_quest(&self, guid: u32, quest_id: u32) -> Result<bool>;

    // Command methods
    async fn save_quest_status(&self, quest_status: &QuestStatusRow) -> Result<()>;
    async fn save_rewarded_quest(&self, quest_rewarded: &QuestStatusRewardedRow) -> Result<()>;
    async fn delete_quest_status(&self, guid: u32, quest_id: u32) -> Result<()>;
    async fn delete_all_quest_statuses(&self, guid: u32) -> Result<()>;
    async fn delete_all_rewarded_quests(&self, guid: u32) -> Result<()>;
}

pub struct QuestRepository {
    pool: Arc<MySqlPool>,
}

impl QuestRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== INTERNAL DATABASE METHODS ==========

    /// Internal method to find all quest statuses
    async fn find_statuses_internal(&self, guid: u32) -> Result<Vec<QuestStatusRow>> {
        sqlx::query_as::<_, QuestStatusRow>(
            r#"SELECT guid, quest, status, rewarded, explored, timer,
                      mob_count1, mob_count2, mob_count3, mob_count4,
                      item_count1, item_count2, item_count3, item_count4,
                      reward_choice
               FROM character_queststatus WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch quest statuses")
    }

    /// Internal method to find all rewarded quests
    async fn find_rewarded_internal(&self, guid: u32) -> Result<Vec<QuestStatusRewardedRow>> {
        // Query from main table using rewarded flag (no separate table in this schema)
        sqlx::query_as::<_, QuestStatusRewardedRow>(
            r#"SELECT guid, quest, reward_choice FROM character_queststatus WHERE guid = ? AND rewarded = 1"#,
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch rewarded quests")
    }

    /// Internal method to find a specific quest status
    async fn find_status_internal(
        &self,
        guid: u32,
        quest_id: u32,
    ) -> Result<Option<QuestStatusRow>> {
        sqlx::query_as::<_, QuestStatusRow>(
            r#"SELECT guid, quest, status, rewarded, explored, timer,
                      mob_count1, mob_count2, mob_count3, mob_count4,
                      item_count1, item_count2, item_count3, item_count4,
                      reward_choice
               FROM character_queststatus WHERE guid = ? AND quest = ?"#,
        )
        .bind(guid)
        .bind(quest_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch quest status")
    }

    /// Internal method to check if quest is completed
    async fn check_completed_internal(&self, guid: u32, quest_id: u32) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM character_queststatus WHERE guid = ? AND quest = ? AND rewarded = 1"#,
        )
        .bind(guid)
        .bind(quest_id)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to check quest completion")?;

        Ok(count > 0)
    }

    /// Internal method to save quest status
    async fn save_status_internal(&self, quest_status: &QuestStatusRow) -> Result<()> {
        sqlx::query(
            r#"REPLACE INTO character_queststatus
               (guid, quest, status, rewarded, explored, timer,
                mob_count1, mob_count2, mob_count3, mob_count4,
                item_count1, item_count2, item_count3, item_count4,
                reward_choice)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(quest_status.guid)
        .bind(quest_status.quest)
        .bind(quest_status.status)
        .bind(quest_status.rewarded)
        .bind(quest_status.explored)
        .bind(quest_status.timer)
        .bind(quest_status.mob_count1)
        .bind(quest_status.mob_count2)
        .bind(quest_status.mob_count3)
        .bind(quest_status.mob_count4)
        .bind(quest_status.item_count1)
        .bind(quest_status.item_count2)
        .bind(quest_status.item_count3)
        .bind(quest_status.item_count4)
        .bind(quest_status.reward_choice)
        .execute(&*self.pool)
        .await
        .context("Failed to save quest status")?;

        Ok(())
    }

    /// Internal method to save rewarded quest
    async fn save_rewarded_internal(&self, quest_rewarded: &QuestStatusRewardedRow) -> Result<()> {
        // Save to main table with rewarded flag (no separate table in this schema)
        // Insert minimal record with just quest ID and rewarded=1
        sqlx::query(
            r#"INSERT INTO character_queststatus
               (guid, quest, status, rewarded, explored, timer,
                mob_count1, mob_count2, mob_count3, mob_count4,
                item_count1, item_count2, item_count3, item_count4,
                reward_choice)
               VALUES (?, ?, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ?)
               ON DUPLICATE KEY UPDATE
                   status = 1,
                   rewarded = 1,
                   reward_choice = IF(VALUES(reward_choice) <> 0, VALUES(reward_choice), reward_choice)"#,
        )
        .bind(quest_rewarded.guid)
        .bind(quest_rewarded.quest)
        .bind(quest_rewarded.reward_choice)
        .execute(&*self.pool)
        .await
        .context("Failed to save rewarded quest")?;

        Ok(())
    }

    /// Internal method to delete quest status
    async fn delete_status_internal(&self, guid: u32, quest_id: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM character_queststatus WHERE guid = ? AND quest = ?"#)
            .bind(guid)
            .bind(quest_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete quest status")?;

        Ok(())
    }

    /// Internal method to delete all quest statuses
    async fn delete_all_statuses_internal(&self, guid: u32) -> Result<()> {
        sqlx::query(r#"DELETE FROM character_queststatus WHERE guid = ?"#)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete all quest statuses")?;

        Ok(())
    }

    /// Internal method to delete all rewarded quests
    async fn delete_all_rewarded_internal(&self, guid: u32) -> Result<()> {
        // Delete from main table using rewarded flag (no separate table in this schema)
        sqlx::query(r#"DELETE FROM character_queststatus WHERE guid = ? AND rewarded = 1"#)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete all rewarded quests")?;

        Ok(())
    }
}

#[async_trait]
impl QuestRepositoryTrait for QuestRepository {
    async fn find_quest_statuses(&self, guid: u32) -> Result<Vec<QuestStatusRow>> {
        self.find_statuses_internal(guid).await
    }

    async fn find_rewarded_quests(&self, guid: u32) -> Result<Vec<QuestStatusRewardedRow>> {
        self.find_rewarded_internal(guid).await
    }

    async fn find_quest_status(&self, guid: u32, quest_id: u32) -> Result<Option<QuestStatusRow>> {
        self.find_status_internal(guid, quest_id).await
    }

    async fn has_completed_quest(&self, guid: u32, quest_id: u32) -> Result<bool> {
        self.check_completed_internal(guid, quest_id).await
    }

    async fn save_quest_status(&self, quest_status: &QuestStatusRow) -> Result<()> {
        self.save_status_internal(quest_status).await
    }

    async fn save_rewarded_quest(&self, quest_rewarded: &QuestStatusRewardedRow) -> Result<()> {
        self.save_rewarded_internal(quest_rewarded).await
    }

    async fn delete_quest_status(&self, guid: u32, quest_id: u32) -> Result<()> {
        self.delete_status_internal(guid, quest_id).await
    }

    async fn delete_all_quest_statuses(&self, guid: u32) -> Result<()> {
        self.delete_all_statuses_internal(guid).await
    }

    async fn delete_all_rewarded_quests(&self, guid: u32) -> Result<()> {
        self.delete_all_rewarded_internal(guid).await
    }
}
