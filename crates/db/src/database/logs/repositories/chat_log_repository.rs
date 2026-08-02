use anyhow::Result;
use sqlx::MySqlPool;
use std::sync::Arc;

use crate::database::logs::models::{ChatLogInsert, ChatOutboxRow};

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

    pub async fn pending_outbox(&self) -> Result<Vec<ChatOutboxRow>> {
        Ok(sqlx::query_as::<_, (u64, u32, u32, String, Option<String>, String)>("SELECT `id`, `sender_account`, `sender_guid`, `channel_type`, `target_name`, `message` FROM `chat_outbox` WHERE `status` = 'pending' ORDER BY `id` ASC LIMIT 20")
            .fetch_all(&*self.pool).await?.into_iter().map(|(id, sender_account, sender_guid, channel_type, target_name, message)| ChatOutboxRow { id, sender_account, sender_guid, channel_type, target_name, message }).collect())
    }

    pub async fn mark_outbox(&self, id: u64, status: &str, error: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE `chat_outbox` SET `status` = ?, `processed_at` = NOW(), `error` = ? WHERE `id` = ?")
            .bind(status).bind(error).bind(id).execute(&*self.pool).await?;
        Ok(())
    }
}
