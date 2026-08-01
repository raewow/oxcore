//! Chat logging to the `logs.chat_log` table and GM message delivery from the
//! `logs.chat_outbox` table.
//!
//! The world server never blocks the gameplay loop on database I/O for chat:
//! [`ChatLogger`] accepts log entries over an unbounded channel and a background
//! task performs the inserts, mirroring the fire-and-forget style of vmangos's
//! `Log::Player(LOG_CHAT)` path (see `reference/core/src/game/Objects/Player.cpp`).

use anyhow::Result;
use sqlx::MySqlPool;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use tracing::{error, warn};
use std::sync::Arc;

use crate::game::chat::ChatError;
use crate::game::player::PlayerManager;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// A single chat message queued for persistence.
#[derive(Debug, Clone)]
pub struct ChatLogEntry {
    /// `Say`, `Yell`, `Whisper`, `Emote`, `Party`, `Raid`, `RaidLeader`,
    /// `RaidWarning`, `Guild`, `Officer` or `Channel`.
    pub channel_type: &'static str,
    /// Channel name for `Channel`/`Guild`/`Officer`, `group:{id}` for party/raid,
    /// `None` for say/yell/emote/whisper.
    pub channel_name: Option<String>,
    pub sender_guid: u32,
    pub sender_name: String,
    pub sender_account: u32,
    pub target_guid: Option<u32>,
    pub target_name: Option<String>,
    pub message: String,
    pub map: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

/// Background chat-log writer. [`ChatSystem`] holds an `Option<Arc<ChatLogger>>`;
/// when unset (tests, or `chat_log_enabled = false`) no database writes happen.
pub struct ChatLogger {
    tx: mpsc::UnboundedSender<ChatLogEntry>,
}

impl ChatLogger {
    pub fn new(pool: MySqlPool) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<ChatLogEntry>();
        tokio::spawn(async move {
            while let Some(entry) = rx.recv().await {
                if let Err(err) = insert_entry(&pool, &entry).await {
                    error!(target: "world_chat_log", %err, sender_name = %entry.sender_name, "failed to write chat log entry");
                }
            }
        });
        Self { tx }
    }

    pub fn log(&self, entry: ChatLogEntry) {
        // A full queue means the writer is backed up; dropping the newest entry is
        // preferable to unbounded memory growth.
        let _ = self.tx.send(entry);
    }
}

async fn insert_entry(pool: &MySqlPool, entry: &ChatLogEntry) -> Result<()> {
    sqlx::query(
        "INSERT INTO `chat_log` (`channel_type`, `channel_name`, `sender_guid`, `sender_name`, \
         `sender_account`, `target_guid`, `target_name`, `message`, `map`, `pos_x`, `pos_y`, `pos_z`) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(entry.channel_type)
    .bind(&entry.channel_name)
    .bind(entry.sender_guid)
    .bind(&entry.sender_name)
    .bind(entry.sender_account)
    .bind(entry.target_guid)
    .bind(&entry.target_name)
    .bind(&entry.message)
    .bind(entry.map)
    .bind(entry.pos_x)
    .bind(entry.pos_y)
    .bind(entry.pos_z)
    .execute(pool)
    .await?;
    Ok(())
}

// ================= Outbox: GM messages from the web admin panel =================

/// Fetch the sender's live character context for the outbox: the character must be
/// online and must belong to the requesting account.
fn outbox_sender(world: &World, sender_guid: u32, sender_account: u32) -> Option<ObjectGuid> {
    let guid = ObjectGuid::new_player(sender_guid);
    world
        .managers
        .player_mgr
        .get_player(guid)
        .filter(|player| player.account_id == sender_account)
        .map(|_| guid)
}

/// Deliver one pending outbox row. Returns `Ok` once the row has been marked
/// `sent`/`failed` in the database.
async fn process_outbox_row(world: &World, row: OutboxRow) -> Result<()> {
    let gm_guid = match outbox_sender(world, row.sender_guid, row.sender_account) {
        Some(guid) => guid,
        None => {
            mark_outbox(&world.databases.logs, row.id, "failed", Some("sender character offline or not on account")).await?;
            return Ok(());
        }
    };

    let result = match row.channel_type.as_str() {
        "Whisper" => match row.target_name {
            Some(target) => world
                .systems
                .chat
                .send_whisper_as_gm(gm_guid, &target, &row.message, &world.systems.social)
                .await,
            None => Err(ChatError::TargetNotFound),
        },
        other => Err(ChatError::NoPermission),
    };

    match result {
        Ok(()) => mark_outbox(&world.databases.logs, row.id, "sent", None).await?,
        Err(err) => mark_outbox(&world.databases.logs, row.id, "failed", Some(&err.to_string())).await?,
    }
    Ok(())
}

struct OutboxRow {
    id: u64,
    sender_account: u32,
    sender_guid: u32,
    channel_type: String,
    target_name: Option<String>,
    message: String,
}

async fn mark_outbox(pool: &MySqlPool, id: u64, status: &str, error: Option<&str>) -> Result<()> {
    sqlx::query(
        "UPDATE `chat_outbox` SET `status` = ?, `processed_at` = NOW(), `error` = ? WHERE `id` = ?",
    )
    .bind(status)
    .bind(error)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Poll `chat_outbox` once for pending rows and deliver them.
async fn poll_outbox(world: Arc<World>) -> Result<()> {
    let rows = sqlx::query_as::<_, (u64, u32, u32, String, Option<String>, String)>(
        "SELECT `id`, `sender_account`, `sender_guid`, `channel_type`, `target_name`, `message` \
         FROM `chat_outbox` WHERE `status` = 'pending' ORDER BY `id` ASC LIMIT 20",
    )
    .fetch_all(&world.databases.logs)
    .await?;

    for (id, sender_account, sender_guid, channel_type, target_name, message) in rows {
        let row = OutboxRow {
            id,
            sender_account,
            sender_guid,
            channel_type,
            target_name,
            message,
        };
        if let Err(err) = process_outbox_row(&world, row).await {
            warn!(target: "world_chat_outbox", %err, outbox_id = id, "failed to process chat outbox row");
        }
    }
    Ok(())
}

/// Spawn the background task that drains `chat_outbox` every second until the
/// shutdown signal fires.
pub fn spawn_chat_outbox_poller(world: Arc<World>, mut shutdown_rx: broadcast::Receiver<()>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(1));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(err) = poll_outbox(world.clone()).await {
                        warn!(target: "world_chat_outbox", %err, "chat outbox poll failed");
                    }
                }
                _ = shutdown_rx.recv() => break,
            }
        }
    });
}
