//! Chat logging to the `logs.chat_log` table and GM message delivery from the
//! `logs.chat_outbox` table.
//!
//! The world server never blocks the gameplay loop on database I/O for chat:
//! [`ChatLogger`] accepts log entries over an unbounded channel and a background
//! task performs the inserts, mirroring the fire-and-forget style of vmangos's
//! `Log::Player(LOG_CHAT)` path (see `reference/core/src/game/Objects/Player.cpp`).

use anyhow::Result;
use oxcore_db::database::logs::models::{ChatLogInsert, ChatOutboxRow};
use oxcore_db::database::logs::repositories::ChatLogRepository;
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{error, warn};

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
                let repository = ChatLogRepository::new(Arc::new(pool.clone()));
                if let Err(err) = repository.insert(&to_insert(&entry)).await {
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

fn to_insert(entry: &ChatLogEntry) -> ChatLogInsert {
    ChatLogInsert {
        channel_type: entry.channel_type.to_string(),
        channel_name: entry.channel_name.clone(),
        sender_guid: entry.sender_guid,
        sender_name: entry.sender_name.clone(),
        sender_account: entry.sender_account,
        target_guid: entry.target_guid,
        target_name: entry.target_name.clone(),
        message: entry.message.clone(),
        map: entry.map,
        pos_x: entry.pos_x,
        pos_y: entry.pos_y,
        pos_z: entry.pos_z,
    }
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
async fn process_outbox_row(world: &World, row: ChatOutboxRow) -> Result<()> {
    let repository = ChatLogRepository::new(Arc::new(world.databases.logs.clone()));
    let gm_guid = match outbox_sender(world, row.sender_guid, row.sender_account) {
        Some(guid) => guid,
        None => {
            repository
                .mark_outbox(
                    row.id,
                    "failed",
                    Some("sender character offline or not on account"),
                )
                .await?;
            return Ok(());
        }
    };

    let result = match row.channel_type.as_str() {
        "Whisper" => match row.target_name {
            Some(target) => {
                world
                    .systems
                    .chat
                    .send_whisper_as_gm(gm_guid, &target, &row.message, &world.systems.social)
                    .await
            }
            None => Err(ChatError::TargetNotFound),
        },
        other => Err(ChatError::NoPermission),
    };

    match result {
        Ok(()) => repository.mark_outbox(row.id, "sent", None).await?,
        Err(err) => {
            repository
                .mark_outbox(row.id, "failed", Some(&err.to_string()))
                .await?
        }
    }
    Ok(())
}

/// Poll `chat_outbox` once for pending rows and deliver them.
/// Returns whether work was found so the caller can back off while the queue is idle.
async fn poll_outbox(world: Arc<World>) -> Result<bool> {
    let rows = ChatLogRepository::new(Arc::new(world.databases.logs.clone()))
        .pending_outbox()
        .await?;

    let has_rows = !rows.is_empty();
    for row in rows {
        let id = row.id;
        if let Err(err) = process_outbox_row(&world, row).await {
            warn!(target: "world_chat_outbox", %err, outbox_id = id, "failed to process chat outbox row");
        }
    }
    Ok(has_rows)
}

/// Spawn the background task that drains `chat_outbox` until shutdown.
///
/// The web and world services use separate database pools, so a newly inserted outbox row has no
/// in-process notification to wake the world. Back off exponentially while idle instead of issuing
/// a permanent once-per-second `SELECT`; once work is found, the queue drains at a one-second cadence.
pub fn spawn_chat_outbox_poller(world: Arc<World>, mut shutdown_rx: broadcast::Receiver<()>) {
    tokio::spawn(async move {
        let mut idle_delay = std::time::Duration::from_secs(1);
        loop {
            tokio::select! {
                _ = tokio::time::sleep(idle_delay) => {
                    match poll_outbox(world.clone()).await {
                        Ok(true) => idle_delay = std::time::Duration::from_secs(1),
                        Ok(false) => idle_delay = (idle_delay * 2).min(std::time::Duration::from_secs(60)),
                        Err(err) => {
                            warn!(target: "world_chat_outbox", %err, "chat outbox poll failed");
                            idle_delay = (idle_delay * 2).min(std::time::Duration::from_secs(60));
                        }
                    }
                },
                _ = shutdown_rx.recv() => break,
            }
        }
    });
}
