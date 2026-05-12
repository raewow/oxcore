use super::super::models::guild::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

/// Trait abstraction for guild database operations
/// Enables mocking for tests via mockall
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait GuildRepositoryTrait: Send + Sync {
    // ========== QUERY METHODS ==========

    /// Find guild by ID
    async fn find_by_id(&self, guild_id: u32) -> Result<Option<GuildRow>>;

    /// Find guild by name
    async fn find_by_name(&self, name: &str) -> Result<Option<GuildRow>>;

    /// Check if guild name exists
    async fn exists_by_name(&self, name: &str) -> Result<bool>;

    /// Get maximum guild ID (for ID generation)
    async fn get_max_guild_id(&self) -> Result<Option<u32>>;

    /// Find all members of a guild
    async fn find_members(&self, guild_id: u32) -> Result<Vec<GuildMemberRow>>;

    /// Find all members with character data (LEFT JOIN with characters table)
    async fn find_members_with_character_data(
        &self,
        guild_id: u32,
    ) -> Result<Vec<GuildMemberWithCharacterDataRow>>;

    /// Find all ranks for a guild
    async fn find_ranks(&self, guild_id: u32) -> Result<Vec<GuildRankRow>>;

    /// Find all bank tabs for a guild
    async fn find_bank_tabs(&self, guild_id: u32) -> Result<Vec<GuildBankTabRow>>;

    /// Find event logs for a guild
    async fn find_event_logs(&self, guild_id: u32, limit: u32) -> Result<Vec<GuildEventLogRow>>;

    /// Find character data by GUID (level, class, zone, account, logout_time)
    async fn find_character_data(&self, guid: u32) -> Result<Option<(u8, u8, u32, u32, i64)>>;

    // ========== COMMAND METHODS ==========

    /// Create a new guild (transactional: guild, ranks, leader member, bank tabs)
    async fn create(
        &self,
        guild: &GuildRow,
        ranks: &[GuildRankRow],
        leader_member: &GuildMemberRow,
        bank_tabs: &[GuildBankTabRow],
    ) -> Result<()>;

    /// Update guild data
    async fn update(&self, guild: &GuildRow) -> Result<()>;

    /// Update guild MOTD
    async fn update_motd(&self, guild_id: u32, motd: &str) -> Result<()>;

    /// Update guild info text
    async fn update_info(&self, guild_id: u32, info: &str) -> Result<()>;

    /// Update guild name
    async fn update_guild_name(&self, guild_id: u32, name: &str) -> Result<()>;

    /// Update guild emblem
    async fn update_emblem(
        &self,
        guild_id: u32,
        emblem_style: i32,
        emblem_color: i32,
        border_style: i32,
        border_color: i32,
        background_color: i32,
    ) -> Result<()>;

    /// Update guild bank money
    async fn update_bank_money(&self, guild_id: u32, amount: u32) -> Result<()>;

    /// Update guild leader (transactional: update leader, update member ranks)
    async fn update_leader(
        &self,
        guild_id: u32,
        old_leader_guid: u32,
        new_leader_guid: u32,
    ) -> Result<()>;

    // ========== MEMBER OPERATIONS ==========

    /// Add a member to the guild
    async fn add_member(&self, member: &GuildMemberRow) -> Result<()>;

    /// Remove a member from the guild
    async fn remove_member(&self, guild_id: u32, guid: u32) -> Result<()>;

    /// Update member rank
    async fn update_member_rank(&self, guild_id: u32, guid: u32, rank: u8) -> Result<()>;

    /// Update member public note
    async fn update_member_public_note(&self, guild_id: u32, guid: u32, note: &str) -> Result<()>;

    /// Update member officer note
    async fn update_member_officer_note(&self, guild_id: u32, guid: u32, note: &str) -> Result<()>;

    // ========== RANK OPERATIONS ==========

    /// Create a new rank
    async fn create_rank(&self, rank: &GuildRankRow) -> Result<()>;

    /// Update rank name and rights
    async fn update_rank(&self, guild_id: u32, rank_id: u32, name: &str, rights: u32)
        -> Result<()>;

    /// Delete a rank
    async fn delete_rank(&self, guild_id: u32, rank_id: u32) -> Result<()>;

    // ========== BANK OPERATIONS ==========

    /// Update bank tab configuration
    async fn update_bank_tab(
        &self,
        guild_id: u32,
        tab_id: u8,
        name: &str,
        icon: &str,
        view_rank: u8,
        withdraw_rank: u8,
        deposit_rank: u8,
    ) -> Result<()>;

    // ========== EVENT LOG OPERATIONS ==========

    /// Insert an event log entry
    async fn insert_event_log(&self, log: &GuildEventLogRow) -> Result<()>;

    /// Get maximum event log GUID for a guild
    async fn get_max_event_log_guid(&self, guild_id: u32) -> Result<Option<i32>>;

    /// Delete old event logs, keeping only the most recent entries
    async fn delete_old_event_logs(&self, guild_id: u32, keep_count: u32) -> Result<()>;

    // ========== DELETE OPERATIONS ==========

    /// Delete a guild and all related data (cascading, transactional)
    async fn delete(&self, guild_id: u32) -> Result<()>;
}

pub struct GuildRepository {
    pool: Arc<MySqlPool>,
}

impl GuildRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    pub async fn find_by_id(&self, guild_id: u32) -> Result<Option<GuildRow>> {
        sqlx::query_as::<_, GuildRow>(
            r#"SELECT guild_id, name, leader_guid, emblem_style, emblem_color,
                      border_style, border_color, background_color, info, motd,
                      create_date, bank_money
               FROM guild WHERE guild_id = ?"#,
        )
        .bind(guild_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch guild by ID")
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<GuildRow>> {
        sqlx::query_as::<_, GuildRow>(
            r#"SELECT guild_id, name, leader_guid, emblem_style, emblem_color,
                      border_style, border_color, background_color, info, motd,
                      create_date, bank_money
               FROM guild WHERE name = ?"#,
        )
        .bind(name)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch guild by name")
    }

    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guild WHERE name = ?")
            .bind(name)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to check guild name existence")?;

        Ok(count > 0)
    }

    pub async fn get_max_guild_id(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(guild_id) FROM guild")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max guild_id")
    }

    pub async fn find_members(&self, guild_id: u32) -> Result<Vec<GuildMemberRow>> {
        sqlx::query_as::<_, GuildMemberRow>(
            r#"SELECT guild_id, guid, `rank`, player_note, officer_note
               FROM guild_member
               WHERE guild_id = ?"#,
        )
        .bind(guild_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch guild members")
    }

    /// LEFT JOIN with characters table. Character data fields may be None if character was deleted.
    pub async fn find_members_with_character_data(
        &self,
        guild_id: u32,
    ) -> Result<Vec<GuildMemberWithCharacterDataRow>> {
        sqlx::query_as::<_, GuildMemberWithCharacterDataRow>(
            r#"SELECT gm.guid, gm.`rank`, gm.player_note, gm.officer_note,
                      c.name, c.level, c.class, c.zone, c.account, c.logout_time
               FROM guild_member gm
               LEFT JOIN characters c ON gm.guid = c.guid
               WHERE gm.guild_id = ?"#,
        )
        .bind(guild_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch guild members with character data")
    }

    pub async fn find_character_data(&self, guid: u32) -> Result<Option<(u8, u8, u32, u32, i64)>> {
        sqlx::query_as::<_, (u8, u8, u32, u32, i64)>(
            r#"SELECT level, class, zone, account, logout_time
               FROM characters
               WHERE guid = ?"#,
        )
        .bind(guid)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch character data")
    }

    pub async fn find_ranks(&self, guild_id: u32) -> Result<Vec<GuildRankRow>> {
        sqlx::query_as::<_, GuildRankRow>(
            r#"SELECT guild_id, id, name, rights
               FROM guild_rank
               WHERE guild_id = ?
               ORDER BY id ASC"#,
        )
        .bind(guild_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch guild ranks")
    }

    pub async fn find_bank_tabs(&self, guild_id: u32) -> Result<Vec<GuildBankTabRow>> {
        sqlx::query_as::<_, GuildBankTabRow>(
            r#"SELECT guild_id, tab_id, name, icon, view_rank, withdraw_rank, deposit_rank
               FROM guild_bank_tab
               WHERE guild_id = ?
               ORDER BY tab_id ASC"#,
        )
        .bind(guild_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch guild bank tabs")
    }

    pub async fn find_event_logs(
        &self,
        guild_id: u32,
        limit: u32,
    ) -> Result<Vec<GuildEventLogRow>> {
        sqlx::query_as::<_, GuildEventLogRow>(
            r#"SELECT guild_id, log_guid, event_type, player_guid1, player_guid2, new_rank, timestamp
               FROM guild_eventlog
               WHERE guild_id = ?
               ORDER BY log_guid DESC
               LIMIT ?"#,
        )
        .bind(guild_id as i32)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch guild event logs")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Transactional: inserts guild, ranks, leader member, and bank tabs atomically.
    pub async fn create(
        &self,
        guild: &GuildRow,
        ranks: &[GuildRankRow],
        leader_member: &GuildMemberRow,
        bank_tabs: &[GuildBankTabRow],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Insert guild
        sqlx::query(
            r#"INSERT INTO guild (guild_id, name, leader_guid, emblem_style, emblem_color,
                                  border_style, border_color, background_color, info, motd,
                                  create_date, bank_money)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(guild.guild_id)
        .bind(&guild.name)
        .bind(guild.leader_guid)
        .bind(guild.emblem_style)
        .bind(guild.emblem_color)
        .bind(guild.border_style)
        .bind(guild.border_color)
        .bind(guild.background_color)
        .bind(&guild.info)
        .bind(&guild.motd)
        .bind(guild.create_date)
        .bind(guild.bank_money)
        .execute(&mut *tx)
        .await
        .context("Failed to insert guild")?;

        // Insert ranks
        for rank in ranks {
            sqlx::query(
                r#"INSERT INTO guild_rank (guild_id, id, name, rights)
                   VALUES (?, ?, ?, ?)"#,
            )
            .bind(guild.guild_id)
            .bind(rank.id)
            .bind(&rank.name)
            .bind(rank.rights)
            .execute(&mut *tx)
            .await
            .context("Failed to insert guild rank")?;
        }

        // Insert leader as first member
        sqlx::query(
            r#"INSERT INTO guild_member (guild_id, guid, `rank`, player_note, officer_note)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(guild.guild_id)
        .bind(leader_member.guid)
        .bind(leader_member.rank)
        .bind(&leader_member.player_note)
        .bind(&leader_member.officer_note)
        .execute(&mut *tx)
        .await
        .context("Failed to insert guild leader as member")?;

        // Insert bank tabs
        for tab in bank_tabs {
            sqlx::query(
                r#"INSERT INTO guild_bank_tab (guild_id, tab_id, name, icon, view_rank, withdraw_rank, deposit_rank)
                   VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(guild.guild_id)
            .bind(tab.tab_id)
            .bind(&tab.name)
            .bind(&tab.icon)
            .bind(tab.view_rank)
            .bind(tab.withdraw_rank)
            .bind(tab.deposit_rank)
            .execute(&mut *tx)
            .await
            .context("Failed to insert guild bank tab")?;
        }

        tx.commit()
            .await
            .context("Failed to commit guild creation")?;
        Ok(())
    }

    pub async fn update(&self, guild: &GuildRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE guild
               SET name = ?, leader_guid = ?, emblem_style = ?, emblem_color = ?,
                   border_style = ?, border_color = ?, background_color = ?,
                   info = ?, motd = ?, bank_money = ?
               WHERE guild_id = ?"#,
        )
        .bind(&guild.name)
        .bind(guild.leader_guid)
        .bind(guild.emblem_style)
        .bind(guild.emblem_color)
        .bind(guild.border_style)
        .bind(guild.border_color)
        .bind(guild.background_color)
        .bind(&guild.info)
        .bind(&guild.motd)
        .bind(guild.bank_money)
        .bind(guild.guild_id)
        .execute(&*self.pool)
        .await
        .context("Failed to update guild")?;

        Ok(())
    }

    pub async fn update_motd(&self, guild_id: u32, motd: &str) -> Result<()> {
        sqlx::query("UPDATE guild SET motd = ? WHERE guild_id = ?")
            .bind(motd)
            .bind(guild_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update guild MOTD")?;

        Ok(())
    }

    pub async fn update_info(&self, guild_id: u32, info: &str) -> Result<()> {
        sqlx::query("UPDATE guild SET info = ? WHERE guild_id = ?")
            .bind(info)
            .bind(guild_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update guild info")?;

        Ok(())
    }

    pub async fn update_guild_name(&self, guild_id: u32, name: &str) -> Result<()> {
        sqlx::query("UPDATE guild SET name = ? WHERE guild_id = ?")
            .bind(name)
            .bind(guild_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update guild name")?;

        Ok(())
    }

    pub async fn update_emblem(
        &self,
        guild_id: u32,
        emblem_style: i32,
        emblem_color: i32,
        border_style: i32,
        border_color: i32,
        background_color: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE guild
               SET emblem_style = ?, emblem_color = ?, border_style = ?,
                   border_color = ?, background_color = ?
               WHERE guild_id = ?"#,
        )
        .bind(emblem_style)
        .bind(emblem_color)
        .bind(border_style)
        .bind(border_color)
        .bind(background_color)
        .bind(guild_id)
        .execute(&*self.pool)
        .await
        .context("Failed to update guild emblem")?;

        Ok(())
    }

    pub async fn update_bank_money(&self, guild_id: u32, amount: u32) -> Result<()> {
        sqlx::query("UPDATE guild SET bank_money = ? WHERE guild_id = ?")
            .bind(amount)
            .bind(guild_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update guild bank money")?;

        Ok(())
    }

    /// Transactional: updates guild table and promotes/demotes members atomically.
    pub async fn update_leader(
        &self,
        guild_id: u32,
        old_leader_guid: u32,
        new_leader_guid: u32,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Update guild table
        sqlx::query("UPDATE guild SET leader_guid = ? WHERE guild_id = ?")
            .bind(new_leader_guid)
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .context("Failed to update guild leader")?;

        // Promote new leader to rank 0
        sqlx::query("UPDATE guild_member SET `rank` = 0 WHERE guild_id = ? AND guid = ?")
            .bind(guild_id)
            .bind(new_leader_guid)
            .execute(&mut *tx)
            .await
            .context("Failed to promote new leader to rank 0")?;

        // Demote old leader to rank 1 (if different from new leader)
        if old_leader_guid != new_leader_guid {
            sqlx::query("UPDATE guild_member SET `rank` = 1 WHERE guild_id = ? AND guid = ?")
                .bind(guild_id)
                .bind(old_leader_guid)
                .execute(&mut *tx)
                .await
                .context("Failed to demote old leader to rank 1")?;
        }

        tx.commit()
            .await
            .context("Failed to commit leader change")?;
        Ok(())
    }

    // ========== MEMBER OPERATIONS ==========

    pub async fn add_member(&self, member: &GuildMemberRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO guild_member (guild_id, guid, `rank`, player_note, officer_note)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(member.guild_id)
        .bind(member.guid)
        .bind(member.rank)
        .bind(&member.player_note)
        .bind(&member.officer_note)
        .execute(&*self.pool)
        .await
        .context("Failed to insert guild member")?;

        Ok(())
    }

    pub async fn remove_member(&self, guild_id: u32, guid: u32) -> Result<()> {
        sqlx::query("DELETE FROM guild_member WHERE guild_id = ? AND guid = ?")
            .bind(guild_id)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete guild member")?;

        Ok(())
    }

    pub async fn update_member_rank(&self, guild_id: u32, guid: u32, rank: u8) -> Result<()> {
        sqlx::query("UPDATE guild_member SET `rank` = ? WHERE guild_id = ? AND guid = ?")
            .bind(rank)
            .bind(guild_id)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update member rank")?;

        Ok(())
    }

    pub async fn update_member_public_note(
        &self,
        guild_id: u32,
        guid: u32,
        note: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE guild_member SET player_note = ? WHERE guild_id = ? AND guid = ?")
            .bind(note)
            .bind(guild_id)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update member public note")?;

        Ok(())
    }

    pub async fn update_member_officer_note(
        &self,
        guild_id: u32,
        guid: u32,
        note: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE guild_member SET officer_note = ? WHERE guild_id = ? AND guid = ?")
            .bind(note)
            .bind(guild_id)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update member officer note")?;

        Ok(())
    }

    // ========== RANK OPERATIONS ==========

    pub async fn create_rank(&self, rank: &GuildRankRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO guild_rank (guild_id, id, name, rights)
               VALUES (?, ?, ?, ?)"#,
        )
        .bind(rank.guild_id)
        .bind(rank.id)
        .bind(&rank.name)
        .bind(rank.rights)
        .execute(&*self.pool)
        .await
        .context("Failed to insert guild rank")?;

        Ok(())
    }

    pub async fn update_rank(
        &self,
        guild_id: u32,
        rank_id: u32,
        name: &str,
        rights: u32,
    ) -> Result<()> {
        sqlx::query("UPDATE guild_rank SET name = ?, rights = ? WHERE guild_id = ? AND id = ?")
            .bind(name)
            .bind(rights)
            .bind(guild_id)
            .bind(rank_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update guild rank")?;

        Ok(())
    }

    pub async fn delete_rank(&self, guild_id: u32, rank_id: u32) -> Result<()> {
        sqlx::query("DELETE FROM guild_rank WHERE guild_id = ? AND id = ?")
            .bind(guild_id)
            .bind(rank_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete guild rank")?;

        Ok(())
    }

    // ========== BANK TAB OPERATIONS ==========

    pub async fn update_bank_tab(
        &self,
        guild_id: u32,
        tab_id: u8,
        name: &str,
        icon: &str,
        view_rank: u8,
        withdraw_rank: u8,
        deposit_rank: u8,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE guild_bank_tab
               SET name = ?, icon = ?, view_rank = ?, withdraw_rank = ?, deposit_rank = ?
               WHERE guild_id = ? AND tab_id = ?"#,
        )
        .bind(name)
        .bind(icon)
        .bind(view_rank)
        .bind(withdraw_rank)
        .bind(deposit_rank)
        .bind(guild_id)
        .bind(tab_id)
        .execute(&*self.pool)
        .await
        .context("Failed to update bank tab")?;

        Ok(())
    }

    // ========== EVENT LOG OPERATIONS ==========

    pub async fn insert_event_log(&self, log: &GuildEventLogRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO guild_eventlog (guild_id, log_guid, event_type, player_guid1, player_guid2, new_rank, timestamp)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(log.guild_id)
        .bind(log.log_guid)
        .bind(log.event_type)
        .bind(log.player_guid1)
        .bind(log.player_guid2)
        .bind(log.new_rank)
        .bind(log.timestamp)
        .execute(&*self.pool)
        .await
        .context("Failed to insert guild event log")?;

        Ok(())
    }

    pub async fn get_max_event_log_guid(&self, guild_id: u32) -> Result<Option<i32>> {
        sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(log_guid) FROM guild_eventlog WHERE guild_id = ?",
        )
        .bind(guild_id as i32)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to query max event log guid")
    }

    pub async fn delete_old_event_logs(&self, guild_id: u32, keep_count: u32) -> Result<()> {
        sqlx::query(
            r#"DELETE FROM guild_eventlog
               WHERE guild_id = ? AND log_guid < (
                   SELECT log_guid FROM (
                       SELECT log_guid FROM guild_eventlog
                       WHERE guild_id = ?
                       ORDER BY log_guid DESC
                       LIMIT 1 OFFSET ?
                   ) tmp
               )"#,
        )
        .bind(guild_id as i32)
        .bind(guild_id as i32)
        .bind(keep_count)
        .execute(&*self.pool)
        .await
        .context("Failed to delete old event logs")?;

        Ok(())
    }

    // ========== DELETE OPERATIONS ==========

    /// Transactional: cascading delete across guild_member, guild_rank, guild_bank_item, guild_bank_tab, guild_eventlog, and guild tables.
    pub async fn delete(&self, guild_id: u32) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete members
        sqlx::query("DELETE FROM guild_member WHERE guild_id = ?")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete guild members")?;

        // Delete ranks
        sqlx::query("DELETE FROM guild_rank WHERE guild_id = ?")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete guild ranks")?;

        // Delete bank items (if table exists)
        sqlx::query("DELETE FROM guild_bank_item WHERE guild_id = ?")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete guild bank items")?;

        // Delete bank tabs
        sqlx::query("DELETE FROM guild_bank_tab WHERE guild_id = ?")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete guild bank tabs")?;

        // Delete event logs
        sqlx::query("DELETE FROM guild_eventlog WHERE guild_id = ?")
            .bind(guild_id as i32)
            .execute(&mut *tx)
            .await
            .context("Failed to delete guild event logs")?;

        // Delete guild
        sqlx::query("DELETE FROM guild WHERE guild_id = ?")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete guild")?;

        tx.commit()
            .await
            .context("Failed to commit guild deletion")?;
        Ok(())
    }
}

// Implement the trait for GuildRepository
// All methods delegate to the existing repository implementation
#[async_trait]
impl GuildRepositoryTrait for GuildRepository {
    async fn find_by_id(&self, guild_id: u32) -> Result<Option<GuildRow>> {
        self.find_by_id(guild_id).await
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<GuildRow>> {
        self.find_by_name(name).await
    }

    async fn exists_by_name(&self, name: &str) -> Result<bool> {
        self.exists_by_name(name).await
    }

    async fn get_max_guild_id(&self) -> Result<Option<u32>> {
        self.get_max_guild_id().await
    }

    async fn find_members(&self, guild_id: u32) -> Result<Vec<GuildMemberRow>> {
        self.find_members(guild_id).await
    }

    async fn find_members_with_character_data(
        &self,
        guild_id: u32,
    ) -> Result<Vec<GuildMemberWithCharacterDataRow>> {
        self.find_members_with_character_data(guild_id).await
    }

    async fn find_ranks(&self, guild_id: u32) -> Result<Vec<GuildRankRow>> {
        self.find_ranks(guild_id).await
    }

    async fn find_bank_tabs(&self, guild_id: u32) -> Result<Vec<GuildBankTabRow>> {
        self.find_bank_tabs(guild_id).await
    }

    async fn find_event_logs(&self, guild_id: u32, limit: u32) -> Result<Vec<GuildEventLogRow>> {
        self.find_event_logs(guild_id, limit).await
    }

    async fn find_character_data(&self, guid: u32) -> Result<Option<(u8, u8, u32, u32, i64)>> {
        self.find_character_data(guid).await
    }

    async fn create(
        &self,
        guild: &GuildRow,
        ranks: &[GuildRankRow],
        leader_member: &GuildMemberRow,
        bank_tabs: &[GuildBankTabRow],
    ) -> Result<()> {
        self.create(guild, ranks, leader_member, bank_tabs).await
    }

    async fn update(&self, guild: &GuildRow) -> Result<()> {
        self.update(guild).await
    }

    async fn update_motd(&self, guild_id: u32, motd: &str) -> Result<()> {
        self.update_motd(guild_id, motd).await
    }

    async fn update_info(&self, guild_id: u32, info: &str) -> Result<()> {
        self.update_info(guild_id, info).await
    }

    async fn update_emblem(
        &self,
        guild_id: u32,
        emblem_style: i32,
        emblem_color: i32,
        border_style: i32,
        border_color: i32,
        background_color: i32,
    ) -> Result<()> {
        self.update_emblem(
            guild_id,
            emblem_style,
            emblem_color,
            border_style,
            border_color,
            background_color,
        )
        .await
    }

    async fn update_bank_money(&self, guild_id: u32, amount: u32) -> Result<()> {
        self.update_bank_money(guild_id, amount).await
    }

    async fn update_leader(
        &self,
        guild_id: u32,
        old_leader_guid: u32,
        new_leader_guid: u32,
    ) -> Result<()> {
        self.update_leader(guild_id, old_leader_guid, new_leader_guid)
            .await
    }

    async fn add_member(&self, member: &GuildMemberRow) -> Result<()> {
        self.add_member(member).await
    }

    async fn remove_member(&self, guild_id: u32, guid: u32) -> Result<()> {
        self.remove_member(guild_id, guid).await
    }

    async fn update_member_rank(&self, guild_id: u32, guid: u32, rank: u8) -> Result<()> {
        self.update_member_rank(guild_id, guid, rank).await
    }

    async fn update_member_public_note(&self, guild_id: u32, guid: u32, note: &str) -> Result<()> {
        self.update_member_public_note(guild_id, guid, note).await
    }

    async fn update_member_officer_note(&self, guild_id: u32, guid: u32, note: &str) -> Result<()> {
        self.update_member_officer_note(guild_id, guid, note).await
    }

    async fn create_rank(&self, rank: &GuildRankRow) -> Result<()> {
        self.create_rank(rank).await
    }

    async fn update_rank(
        &self,
        guild_id: u32,
        rank_id: u32,
        name: &str,
        rights: u32,
    ) -> Result<()> {
        self.update_rank(guild_id, rank_id, name, rights).await
    }

    async fn delete_rank(&self, guild_id: u32, rank_id: u32) -> Result<()> {
        self.delete_rank(guild_id, rank_id).await
    }

    async fn update_bank_tab(
        &self,
        guild_id: u32,
        tab_id: u8,
        name: &str,
        icon: &str,
        view_rank: u8,
        withdraw_rank: u8,
        deposit_rank: u8,
    ) -> Result<()> {
        self.update_bank_tab(
            guild_id,
            tab_id,
            name,
            icon,
            view_rank,
            withdraw_rank,
            deposit_rank,
        )
        .await
    }

    async fn insert_event_log(&self, log: &GuildEventLogRow) -> Result<()> {
        self.insert_event_log(log).await
    }

    async fn get_max_event_log_guid(&self, guild_id: u32) -> Result<Option<i32>> {
        self.get_max_event_log_guid(guild_id).await
    }

    async fn delete_old_event_logs(&self, guild_id: u32, keep_count: u32) -> Result<()> {
        self.delete_old_event_logs(guild_id, keep_count).await
    }

    async fn delete(&self, guild_id: u32) -> Result<()> {
        self.delete(guild_id).await
    }

    async fn update_guild_name(&self, guild_id: u32, name: &str) -> Result<()> {
        self.update_guild_name(guild_id, name).await
    }
}
