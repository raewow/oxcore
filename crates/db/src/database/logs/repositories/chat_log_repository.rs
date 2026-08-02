use anyhow::Result;
use sqlx::{MySql, MySqlPool, QueryBuilder};
use std::sync::Arc;

use crate::database::logs::models::{
    ChatChannelSummaryRow, ChatLogInsert, ChatLogRow, ChatOutboxInsert, ChatOutboxRow,
    ChatParticipantRow,
};

type ChatLogTuple = (
    u64,
    i64,
    String,
    Option<String>,
    Option<u32>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<String>,
    String,
    Option<u32>,
);

const CHAT_LOG_COLUMNS: &str =
    "`id`, UNIX_TIMESTAMP(`time`), `channel_type`, `channel_name`, `sender_guid`, `sender_name`, \
     `sender_account`, `target_guid`, `target_name`, `message`, `map`";

#[derive(Clone)]
pub struct ChatLogRepository {
    pool: Arc<MySqlPool>,
}

impl ChatLogRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, entry: &ChatLogInsert) -> Result<()> {
        sqlx::query("INSERT INTO `chat_log` (`channel_type`, `channel_name`, `sender_guid`, `sender_name`, `sender_account`, `target_guid`, `target_name`, `message`, `map`, `pos_x`, `pos_y`, `pos_z`) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&entry.channel_type).bind(&entry.channel_name).bind(entry.sender_guid)
            .bind(&entry.sender_name).bind(entry.sender_account).bind(entry.target_guid)
            .bind(&entry.target_name).bind(&entry.message).bind(entry.map).bind(entry.pos_x)
            .bind(entry.pos_y).bind(entry.pos_z).execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn message_count(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM `chat_log`")
            .fetch_one(&*self.pool)
            .await?)
    }

    pub async fn channel_summaries(&self) -> Result<Vec<ChatChannelSummaryRow>> {
        Ok(
            sqlx::query_as::<_, (String, Option<String>, i64, i64, i64)>(
                "SELECT `channel_type`, `channel_name`, COUNT(*), COUNT(DISTINCT `sender_name`), \
             UNIX_TIMESTAMP(MAX(`time`)) FROM `chat_log` GROUP BY `channel_type`, `channel_name` \
             ORDER BY MAX(`id`) DESC LIMIT 150",
            )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(
                |(channel_type, channel_name, message_count, participants, last_message_at)| {
                    ChatChannelSummaryRow {
                        channel_type,
                        channel_name,
                        message_count,
                        participants,
                        last_message_at,
                    }
                },
            )
            .collect(),
        )
    }

    pub async fn messages_for_channel(
        &self,
        channel_type: &str,
        channel_name: Option<&str>,
        before_id: u64,
        limit: u32,
    ) -> Result<Vec<ChatLogRow>> {
        let mut query = Self::message_query();
        query.push(" WHERE channel_type = ");
        query.push_bind(channel_type);
        if let Some(channel_name) = channel_name.filter(|name| !name.is_empty()) {
            query.push(" AND channel_name = ");
            query.push_bind(channel_name);
        } else {
            query.push(" AND channel_name IS NULL");
        }
        if before_id > 0 {
            query.push(" AND id < ");
            query.push_bind(before_id);
        }
        query.push(" ORDER BY id DESC LIMIT ");
        query.push_bind(limit);
        self.fetch_messages(query).await
    }

    pub async fn participants(&self, pattern: &str) -> Result<Vec<ChatParticipantRow>> {
        Ok(
            sqlx::query_as::<_, (Option<u32>, String, Option<u32>, i64, i64)>(
                "SELECT `sender_guid`, sender_name, MAX(sender_account), COUNT(*), \
             UNIX_TIMESTAMP(MAX(time)) FROM `chat_log` WHERE sender_name LIKE ? \
             GROUP BY `sender_guid`, sender_name ORDER BY MAX(id) DESC LIMIT 200",
            )
            .bind(pattern)
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(
                |(guid, name, account, message_count, last_seen)| ChatParticipantRow {
                    guid,
                    name,
                    account,
                    message_count,
                    last_seen,
                },
            )
            .collect(),
        )
    }

    pub async fn messages_for_player(&self, name: &str) -> Result<Vec<ChatLogRow>> {
        let mut query = Self::message_query();
        query.push(" WHERE sender_name = ");
        query.push_bind(name);
        query.push(" OR target_name = ");
        query.push_bind(name);
        query.push(" ORDER BY id DESC LIMIT 300");
        self.fetch_messages(query).await
    }

    pub async fn messages_for_account(
        &self,
        account_id: u32,
        character_guids: &[u32],
    ) -> Result<Vec<ChatLogRow>> {
        let mut query = Self::message_query();
        query.push(" WHERE sender_account = ");
        query.push_bind(account_id);
        if !character_guids.is_empty() {
            query.push(" OR target_guid IN (");
            let mut separated = query.separated(", ");
            for guid in character_guids {
                separated.push_bind(guid);
            }
            query.push(")");
        }
        query.push(" ORDER BY id DESC LIMIT 500");
        self.fetch_messages(query).await
    }

    pub async fn live_messages(
        &self,
        since: u64,
        channel_type: Option<&str>,
        channel_name: Option<&str>,
        player: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ChatLogRow>> {
        let mut query = Self::message_query();
        query.push(" WHERE 1=1");
        if since > 0 {
            query.push(" AND id > ");
            query.push_bind(since);
        }
        if let Some(channel_type) = channel_type.filter(|value| !value.is_empty()) {
            query.push(" AND channel_type = ");
            query.push_bind(channel_type);
        }
        if let Some(channel_name) = channel_name.filter(|value| !value.is_empty()) {
            query.push(" AND channel_name = ");
            query.push_bind(channel_name);
        }
        if let Some(player) = player.filter(|value| !value.is_empty()) {
            query.push(" AND (sender_name = ");
            query.push_bind(player);
            query.push(" OR target_name = ");
            query.push_bind(player);
            query.push(")");
        }
        query.push(if since > 0 {
            " ORDER BY id ASC LIMIT "
        } else {
            " ORDER BY id DESC LIMIT "
        });
        query.push_bind(limit);
        self.fetch_messages(query).await
    }

    pub async fn enqueue_outbox(&self, entry: &ChatOutboxInsert) -> Result<()> {
        let mut query = QueryBuilder::new(
            "INSERT INTO `chat_outbox` (`sender_account`, `sender_guid`, `channel_type`",
        );
        if entry.channel_name.is_some() {
            query.push(", `channel_name`");
        }
        if entry.target_name.is_some() {
            query.push(", `target_name`");
        }
        query.push(", `message`) VALUES (");
        query.push_bind(entry.sender_account);
        query.push(", ");
        query.push_bind(entry.sender_guid);
        query.push(", ");
        query.push_bind(&entry.channel_type);
        if let Some(channel_name) = &entry.channel_name {
            query.push(", ");
            query.push_bind(channel_name);
        }
        if let Some(target_name) = &entry.target_name {
            query.push(", ");
            query.push_bind(target_name);
        }
        query.push(", ");
        query.push_bind(&entry.message);
        query.push(")");
        query.build().execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn pending_outbox(&self) -> Result<Vec<ChatOutboxRow>> {
        Ok(sqlx::query_as::<_, (u64, u32, u32, String, Option<String>, String)>("SELECT `id`, `sender_account`, `sender_guid`, `channel_type`, `target_name`, `message` FROM `chat_outbox` WHERE `status` = 'pending' ORDER BY `id` ASC LIMIT 20")
            .fetch_all(&*self.pool).await?.into_iter().map(|(id, sender_account, sender_guid, channel_type, target_name, message)| ChatOutboxRow { id, sender_account, sender_guid, channel_type, target_name, message }).collect())
    }

    pub async fn mark_outbox(&self, id: u64, status: &str, error: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE `chat_outbox` SET `status` = ?, `processed_at` = NOW(), `error` = ? WHERE `id` = ?")
            .bind(status).bind(error).bind(id).execute(&*self.pool).await?;
        Ok(())
    }

    fn message_query<'args>() -> QueryBuilder<'args, MySql> {
        let mut query = QueryBuilder::new("SELECT ");
        query.push(CHAT_LOG_COLUMNS);
        query.push(" FROM `chat_log`");
        query
    }

    async fn fetch_messages(&self, mut query: QueryBuilder<'_, MySql>) -> Result<Vec<ChatLogRow>> {
        Ok(query
            .build_query_as::<ChatLogTuple>()
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(
                |(
                    id,
                    time,
                    channel_type,
                    channel_name,
                    sender_guid,
                    sender_name,
                    sender_account,
                    target_guid,
                    target_name,
                    message,
                    map,
                )| {
                    ChatLogRow {
                        id,
                        time,
                        channel_type,
                        channel_name,
                        sender_guid,
                        sender_name,
                        sender_account,
                        target_guid,
                        target_name,
                        message,
                        map,
                    }
                },
            )
            .collect())
    }
}
