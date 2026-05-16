//! Mail command handlers for world
//!
//! Commands for sending and managing mail.

use anyhow::Result;
use std::sync::Arc;

use crate::shared::common::AccountType;
use crate::shared::database::characters::models::mail::MailRow;
use crate::shared::database::characters::repositories::mail_repository::MailRepository;
use crate::shared::database::characters::repositories::mail_repository_trait::MailRepositoryTrait;
use crate::shared::game::mail::{MailMessageType, MailStationery};
use crate::shared::messages::mail::SmsgReceivedMail;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};

const EXPIRE_DAYS: i64 = 30; // Mail expires after 30 days

/// Send mail command - sends mail to any player
/// This spawns the actual work in a background task to avoid blocking packet handling
pub async fn cmd_sendmail(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok("Usage: .sendmail <player> <subject> [body]".to_string());
    }

    // Parse arguments: first word is player name, second is subject, rest is body
    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Ok("Usage: .sendmail <player> <subject> [body]".to_string());
    }

    let player_name = parts[0].to_string();
    let subject = parts[1].trim_matches('"').to_string();
    let body = parts
        .get(2)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default();

    // Validate subject length
    if subject.len() > 64 {
        return Ok("Subject too long. Maximum 64 characters.".to_string());
    }

    // Get required data before spawning
    let sender_guid = ctx.player_guid.low();
    let character_pool = Arc::new(ctx.world.databases.character.clone());
    let world = Arc::new(MailWorldContext {
        player_mgr: Arc::clone(&ctx.world.managers.player_mgr),
        session_mgr: Arc::clone(&ctx.world.session_mgr),
    });

    // Clone for the response message
    let player_name_resp = player_name.clone();
    let subject_resp = subject.clone();

    // Spawn the mail sending as a background task so we don't block packet handling
    tokio::spawn(async move {
        tracing::info!(
            "[SENDMAIL] Background task started for '{}' - '{}'",
            player_name,
            subject
        );

        let mail_repo = MailRepository::new(character_pool);

        // Find recipient by name
        let receiver_guid = match mail_repo.find_player_guid_by_name(&player_name).await {
            Ok(Some(guid)) => guid,
            Ok(None) => {
                tracing::warn!("[SENDMAIL] Player '{}' not found", player_name);
                return;
            }
            Err(e) => {
                tracing::error!(
                    "[SENDMAIL] Database error looking up player '{}': {}",
                    player_name,
                    e
                );
                return;
            }
        };

        tracing::info!(
            "[SENDMAIL] Found player '{}' with GUID {}",
            player_name,
            receiver_guid
        );

        // Create item text for body if present
        let item_text_id = if !body.is_empty() {
            if body.len() > 500 {
                tracing::warn!("[SENDMAIL] Body too long");
                0
            } else {
                match mail_repo.create_item_text(&body).await {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::error!("[SENDMAIL] Failed to create item text: {}", e);
                        0
                    }
                }
            }
        } else {
            0
        };

        // Calculate expiration time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expire_time = now + (EXPIRE_DAYS * 24 * 60 * 60);

        // Create mail row
        let mail_row = MailRow {
            id: 0,
            message_type: MailMessageType::Normal as u8,
            stationery: MailStationery::Gm as i8,
            mail_template_id: 0,
            sender_guid,
            receiver_guid,
            subject: Some(subject.clone()),
            item_text_id,
            has_items: 0,
            expire_time,
            deliver_time: 0,
            money: 0,
            cod: 0,
            checked: 0,
        };

        // Insert mail into database
        match mail_repo.create(&mail_row).await {
            Ok(mail_id) => {
                tracing::info!(
                    "[SENDMAIL] Mail created with ID {} for '{}'",
                    mail_id,
                    player_name
                );

                // Notify recipient if online
                if let Some(receiver_guid_obj) = world.player_mgr.find_player_by_name(&player_name)
                {
                    let notification = SmsgReceivedMail {};
                    if let Some(session) =
                        world.session_mgr.get_session_by_player(receiver_guid_obj)
                    {
                        let _ = session.send_packet(notification.to_world_packet());
                        tracing::info!(
                            "[SENDMAIL] Notification sent to online player '{}'",
                            player_name
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("[SENDMAIL] Failed to create mail: {}", e);
            }
        }
    });

    // Return immediately so we don't block packet handling
    Ok(format!(
        "Sending mail to {} with subject '{}'... (check logs for result)",
        player_name_resp, subject_resp
    ))
}

/// Lightweight context for mail operations
struct MailWorldContext {
    player_mgr: Arc<crate::world::game::player::PlayerManager>,
    session_mgr: Arc<crate::world::core::session::SessionManager>,
}

pub fn sendmail_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "sendmail",
        help: "Send mail to any player. Usage: .sendmail <player> <subject> [body]",
        min_security: AccountType::GameMaster,
    }
}
