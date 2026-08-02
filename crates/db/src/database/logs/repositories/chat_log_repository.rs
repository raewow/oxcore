use anyhow::Result;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::sync::Arc;

use crate::database::logs::models::{
    ChatChannelSummaryRow, ChatLogInsert, ChatLogRow, ChatOutboxInsert, ChatOutboxRow,
    ChatParticipantRow,
};

type ChatLogTuple = (
    i64,
    i64,
    String,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    String,
    Option<i64>,
);

const CHAT_LOG_COLUMNS: &str =
    "id, EXTRACT(EPOCH FROM time)::BIGINT, channel_type, channel_name, sender_guid, sender_name, \
     sender_account, target_guid, target_name, message, map";

#[derive(Clone)]
pub struct ChatLogRepository {
    pool: Arc<PgPool>,
}

impl ChatLogRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, entry: &ChatLogInsert) -> Result<()> {
        sqlx::query("INSERT INTO logs.chat_log (channel_type, channel_name, sender_guid, sender_name, sender_account, target_guid, target_name, message, map, pos_x, pos_y, pos_z) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)")
            .bind(&entry.channel_type).bind(&entry.channel_name).bind(i64::from(entry.sender_guid))
            .bind(&entry.sender_name).bind(i64::from(entry.sender_account)).bind(entry.target_guid.map(i64::from))
            .bind(&entry.target_name).bind(&entry.message).bind(i64::from(entry.map)).bind(entry.pos_x)
            .bind(entry.pos_y).bind(entry.pos_z).execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn message_count(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM logs.chat_log")
            .fetch_one(&*self.pool)
            .await?)
    }

    pub async fn channel_summaries(&self) -> Result<Vec<ChatChannelSummaryRow>> {
        Ok(
            sqlx::query_as::<_, (String, Option<String>, i64, i64, i64)>(
                "SELECT channel_type, channel_name, COUNT(*), COUNT(DISTINCT sender_name), \
             EXTRACT(EPOCH FROM MAX(time))::BIGINT FROM logs.chat_log GROUP BY channel_type, channel_name \
              ORDER BY MAX(id) DESC LIMIT 150",
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
            query.push_bind(i64::try_from(before_id)?);
        }
        query.push(" ORDER BY id DESC LIMIT ");
        query.push_bind(i64::from(limit));
        self.fetch_messages(query).await
    }

    pub async fn participants(&self, pattern: &str) -> Result<Vec<ChatParticipantRow>> {
        Ok(
            sqlx::query_as::<_, (Option<i64>, String, Option<i64>, i64, i64)>(
                "SELECT sender_guid, sender_name, MAX(sender_account), COUNT(*), \
             EXTRACT(EPOCH FROM MAX(time))::BIGINT FROM logs.chat_log WHERE sender_name LIKE $1 \
              GROUP BY sender_guid, sender_name ORDER BY MAX(id) DESC LIMIT 200",
            )
            .bind(pattern)
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(
                |(guid, name, account, message_count, last_seen)| -> Result<ChatParticipantRow> {
                    Ok(ChatParticipantRow {
                        guid: guid.map(u32::try_from).transpose()?,
                        name,
                        account: account.map(u32::try_from).transpose()?,
                        message_count,
                        last_seen,
                    })
                },
            )
            .collect::<Result<Vec<_>, _>>()?,
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
        query.push_bind(i64::from(account_id));
        if !character_guids.is_empty() {
            query.push(" OR target_guid IN (");
            let mut separated = query.separated(", ");
            for guid in character_guids {
                separated.push_bind(i64::from(*guid));
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
            query.push_bind(i64::try_from(since)?);
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
        query.push_bind(i64::from(limit));
        self.fetch_messages(query).await
    }

    pub async fn enqueue_outbox(&self, entry: &ChatOutboxInsert) -> Result<()> {
        let mut query = QueryBuilder::new(
            "INSERT INTO logs.chat_outbox (sender_account, sender_guid, channel_type",
        );
        if entry.channel_name.is_some() {
            query.push(", channel_name");
        }
        if entry.target_name.is_some() {
            query.push(", target_name");
        }
        query.push(", message) VALUES (");
        query.push_bind(i64::from(entry.sender_account));
        query.push(", ");
        query.push_bind(i64::from(entry.sender_guid));
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
        Ok(sqlx::query_as::<_, (i64, i64, i64, String, Option<String>, String)>("SELECT id, sender_account, sender_guid, channel_type, target_name, message FROM logs.chat_outbox WHERE status = 'pending' ORDER BY id ASC LIMIT 20")
            .fetch_all(&*self.pool).await?.into_iter().map(|(id, sender_account, sender_guid, channel_type, target_name, message)| -> Result<ChatOutboxRow> { Ok(ChatOutboxRow { id: u64::try_from(id)?, sender_account: u32::try_from(sender_account)?, sender_guid: u32::try_from(sender_guid)?, channel_type, target_name, message }) }).collect::<Result<Vec<_>>>()?)
    }

    pub async fn mark_outbox(&self, id: u64, status: &str, error: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE logs.chat_outbox SET status = $1, processed_at = NOW(), error = $2 WHERE id = $3")
            .bind(status).bind(error).bind(i64::try_from(id)?).execute(&*self.pool).await?;
        Ok(())
    }

    fn message_query<'args>() -> QueryBuilder<'args, Postgres> {
        let mut query = QueryBuilder::new("SELECT ");
        query.push(CHAT_LOG_COLUMNS);
        query.push(" FROM logs.chat_log");
        query
    }

    async fn fetch_messages(
        &self,
        mut query: QueryBuilder<'_, Postgres>,
    ) -> Result<Vec<ChatLogRow>> {
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
                )|
                 -> Result<ChatLogRow> {
                    Ok(ChatLogRow {
                        id: u64::try_from(id)?,
                        time,
                        channel_type,
                        channel_name,
                        sender_guid: sender_guid.map(u32::try_from).transpose()?,
                        sender_name,
                        sender_account: sender_account.map(u32::try_from).transpose()?,
                        target_guid: target_guid.map(u32::try_from).transpose()?,
                        target_name,
                        message,
                        map: map.map(u32::try_from).transpose()?,
                    })
                },
            )
            .collect::<Result<Vec<_>, _>>()?)
    }
}
