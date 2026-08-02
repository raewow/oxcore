use super::super::models::group::*;
use super::group_repository_trait::GroupRepositoryTrait;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct GroupRepository {
    pool: Arc<MySqlPool>,
}

impl GroupRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum group ID from the database (for generating next ID).
    pub async fn get_max_group_id(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(group_id) FROM `groups`")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max group_id")
    }

    /// Find a group by ID.
    pub async fn find_by_id(&self, group_id: u32) -> Result<Option<GroupRow>> {
        sqlx::query_as::<_, GroupRow>(
            r#"SELECT group_id, leader_guid, main_tank_guid, main_assistant_guid,
                      loot_method, loot_threshold, looter_guid,
                      icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, is_raid
               FROM `groups`
               WHERE group_id = ?"#,
        )
        .bind(group_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch group by ID")
    }

    /// Load all groups from the database.
    pub async fn find_all(&self) -> Result<Vec<GroupRow>> {
        sqlx::query_as::<_, GroupRow>(
            r#"SELECT group_id, leader_guid, main_tank_guid, main_assistant_guid,
                      loot_method, loot_threshold, looter_guid,
                      icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, is_raid
               FROM `groups`"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch all groups")
    }

    /// Find all members for a group.
    pub async fn find_members(&self, group_id: u32) -> Result<Vec<GroupMemberRow>> {
        sqlx::query_as::<_, GroupMemberRow>(
            r#"SELECT group_id, member_guid, assistant, subgroup
               FROM group_member
               WHERE group_id = ?"#,
        )
        .bind(group_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch group members")
    }

    /// Load all group members (for all groups).
    pub async fn find_all_members(&self) -> Result<Vec<GroupMemberRow>> {
        sqlx::query_as::<_, GroupMemberRow>(
            r#"SELECT group_id, member_guid, assistant, subgroup
               FROM group_member"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch all group members")
    }

    /// Find group ID for a member.
    pub async fn find_group_for_member(&self, member_guid: u32) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, u32>("SELECT group_id FROM group_member WHERE member_guid = ?")
            .bind(member_guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find group for member")
    }

    /// Find members with character data (LEFT JOIN).
    pub async fn find_members_with_character_data(
        &self,
        group_id: u32,
    ) -> Result<Vec<GroupMemberWithCharacterDataRow>> {
        sqlx::query_as::<_, GroupMemberWithCharacterDataRow>(
            r#"SELECT gm.member_guid, gm.assistant, gm.subgroup,
                      c.name, c.level, c.class, c.zone, c.online
               FROM group_member gm
               LEFT JOIN characters c ON gm.member_guid = c.guid
               WHERE gm.group_id = ?"#,
        )
        .bind(group_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch group members with character data")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create or update a group (uses REPLACE INTO).
    pub async fn save(&self, group: &GroupRow) -> Result<()> {
        sqlx::query(
            r#"REPLACE INTO `groups`
               (group_id, leader_guid, main_tank_guid, main_assistant_guid,
                loot_method, loot_threshold, looter_guid,
                icon1, icon2, icon3, icon4, icon5, icon6, icon7, icon8, is_raid)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(group.group_id)
        .bind(group.leader_guid)
        .bind(group.main_tank_guid)
        .bind(group.main_assistant_guid)
        .bind(group.loot_method)
        .bind(group.loot_threshold)
        .bind(group.looter_guid)
        .bind(group.icon1)
        .bind(group.icon2)
        .bind(group.icon3)
        .bind(group.icon4)
        .bind(group.icon5)
        .bind(group.icon6)
        .bind(group.icon7)
        .bind(group.icon8)
        .bind(group.is_raid)
        .execute(&*self.pool)
        .await
        .context("Failed to save group")?;

        Ok(())
    }

    /// Add a member to a group.
    pub async fn add_member(&self, group_id: u32, member_guid: u32, subgroup: u16) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO group_member (group_id, member_guid, assistant, subgroup)
               VALUES (?, ?, 0, ?)"#,
        )
        .bind(group_id)
        .bind(member_guid)
        .bind(subgroup)
        .execute(&*self.pool)
        .await
        .context("Failed to add group member")?;

        Ok(())
    }

    /// Update member subgroup or assistant status.
    pub async fn update_member(
        &self,
        group_id: u32,
        member_guid: u32,
        assistant: bool,
        subgroup: u16,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE group_member
               SET assistant = ?, subgroup = ?
               WHERE group_id = ? AND member_guid = ?"#,
        )
        .bind(if assistant { 1u8 } else { 0u8 })
        .bind(subgroup)
        .bind(group_id)
        .bind(member_guid)
        .execute(&*self.pool)
        .await
        .context("Failed to update group member")?;

        Ok(())
    }

    /// Remove a member from a group.
    pub async fn remove_member(&self, group_id: u32, member_guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM group_member WHERE group_id = ? AND member_guid = ?")
            .bind(group_id)
            .bind(member_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to remove group member")?;

        Ok(())
    }

    // ========== DELETE OPERATIONS ==========

    /// Delete a group and all its members (transactional).
    pub async fn delete(&self, group_id: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete members first
        sqlx::query("DELETE FROM group_member WHERE group_id = ?")
            .bind(group_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete group members")?;

        // Delete group
        sqlx::query("DELETE FROM `groups` WHERE group_id = ?")
            .bind(group_id)
            .execute(&mut *tx)
            .await
            .context("Failed to commit group deletion")?;

        tx.commit()
            .await
            .context("Failed to commit group deletion")?;
        Ok(())
    }
}

/// Trait implementation for GroupRepository.
/// Delegates all methods to the concrete implementation.
#[async_trait]
impl GroupRepositoryTrait for GroupRepository {
    async fn get_max_group_id(&self) -> Result<Option<u32>> {
        self.get_max_group_id().await
    }

    async fn find_by_id(&self, group_id: u32) -> Result<Option<GroupRow>> {
        self.find_by_id(group_id).await
    }

    async fn find_all(&self) -> Result<Vec<GroupRow>> {
        self.find_all().await
    }

    async fn find_members(&self, group_id: u32) -> Result<Vec<GroupMemberRow>> {
        self.find_members(group_id).await
    }

    async fn find_group_for_member(&self, member_guid: u32) -> Result<Option<u32>> {
        self.find_group_for_member(member_guid).await
    }

    async fn find_members_with_character_data(
        &self,
        group_id: u32,
    ) -> Result<Vec<GroupMemberWithCharacterDataRow>> {
        self.find_members_with_character_data(group_id).await
    }

    async fn save_group(&self, group: &GroupRow) -> Result<()> {
        self.save(group).await
    }

    async fn add_member(&self, group_id: u32, member_guid: u32, subgroup: u16) -> Result<()> {
        self.add_member(group_id, member_guid, subgroup).await
    }

    async fn update_member(
        &self,
        group_id: u32,
        member_guid: u32,
        assistant: bool,
        subgroup: u16,
    ) -> Result<()> {
        self.update_member(group_id, member_guid, assistant, subgroup)
            .await
    }

    async fn remove_member(&self, group_id: u32, member_guid: u32) -> Result<()> {
        self.remove_member(group_id, member_guid).await
    }

    async fn delete_group(&self, group_id: u32) -> Result<()> {
        self.delete(group_id).await
    }
}
