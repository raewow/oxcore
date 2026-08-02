use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use oxcore_db::database::logs::{
    models::{ChatLogRow, ChatOutboxInsert},
    repositories::ChatLogRepository,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalOverview {
    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub characters: Vec<PortalCharacter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct PortalCharacter {
    pub guid: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub online: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDetail {
    pub guid: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub online: u8,
    pub zone: u32,
    pub money: u32,
    pub played_time_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub expires_at: i64,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub action: String,
    pub target_type: String,
    pub occurred_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: u64,
    pub subject: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmStatus {
    pub name: String,
    pub population: f32,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminOverview {
    pub realms: Vec<RealmStatus>,
    pub open_support_tickets: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAccount {
    pub id: u32,
    pub username: String,
    pub email: Option<String>,
    pub gmlevel: u8,
    pub locked: u8,
    pub banned: u8,
    pub last_login: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAccountDetail {
    pub id: u32,
    pub username: String,
    pub email: Option<String>,
    pub gmlevel: u8,
    pub locked: u8,
    pub banned: u8,
    pub expansion: u8,
    pub last_ip: String,
    pub muted_until: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmAccess {
    pub realm_id: i32,
    pub gmlevel: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAccountPage {
    pub accounts: Vec<AdminAccount>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminCharacter {
    pub guid: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub online: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminCharacterPage {
    pub characters: Vec<AdminCharacter>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

// ========== Chat monitoring ==========

/// One chat message as stored in `logs.chat_log`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: u64,
    pub time: i64,
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub sender_guid: Option<u32>,
    pub sender_name: Option<String>,
    pub sender_account: Option<u32>,
    pub target_guid: Option<u32>,
    pub target_name: Option<String>,
    pub message: String,
    pub map: Option<u32>,
}

/// Aggregated view of one `(channel_type, channel_name)` conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChannelSummary {
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub message_count: u64,
    pub participants: u64,
    pub last_message_at: i64,
}

/// A player that has spoken in `chat_log`, for the participant lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParticipant {
    pub guid: Option<u32>,
    pub name: String,
    pub account: Option<u32>,
    pub message_count: u64,
    pub last_seen: i64,
}

/// Overview for the channels tab: total volume plus per-conversation summaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOverview {
    pub total_messages: u64,
    pub channels: Vec<ChatChannelSummary>,
}

/// Everything the chat page needs to render a player dive-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPlayerDetail {
    pub name: String,
    pub account_id: Option<u32>,
    pub messages: Vec<ChatMessage>,
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendChatResult {
    pub accepted: bool,
    pub note: String,
}

#[cfg(feature = "ssr")]
fn chat_row(row: ChatLogRow) -> ChatMessage {
    ChatMessage {
        id: row.id,
        time: row.time,
        channel_type: row.channel_type,
        channel_name: row.channel_name,
        sender_guid: row.sender_guid,
        sender_name: row.sender_name,
        sender_account: row.sender_account,
        target_guid: row.target_guid,
        target_name: row.target_name,
        message: row.message,
        map: row.map,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminCharacterDetail {
    pub guid: u32,
    pub name: String,
    pub account: u32,
    pub account_username: Option<String>,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub level: u8,
    pub online: u8,
    pub zone: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub money: u32,
    pub played_time_total: u32,
    pub create_time: i64,
    pub logout_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSession {
    pub account_id: u32,
    pub id: String,
    pub created_at: i64,
    pub last_seen_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMapPlayer {
    pub guid: u32,
    pub name: String,
    pub level: u8,
    pub class: u8,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub zone: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: u64,
    pub occurred_at: i64,
    pub actor: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub reason: Option<String>,
}

pub const SECURITY_LEVELS: &[(u8, &str)] = &[
    (0, "Player"),
    (1, "Moderator"),
    (2, "Ticketmaster"),
    (3, "Gamemaster"),
    (4, "Basic Admin"),
    (5, "Developer"),
    (6, "Administrator"),
    (7, "Console"),
];

pub fn class_name(class: u8) -> &'static str {
    match class {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        11 => "Druid",
        _ => "Unknown class",
    }
}

pub fn race_name(race: u8) -> &'static str {
    match race {
        1 => "Human",
        2 => "Orc",
        3 => "Dwarf",
        4 => "Night Elf",
        5 => "Undead",
        6 => "Tauren",
        7 => "Gnome",
        8 => "Troll",
        _ => "Unknown race",
    }
}

pub fn gender_name(gender: u8) -> &'static str {
    match gender {
        0 => "Male",
        1 => "Female",
        _ => "Unknown",
    }
}

pub fn format_played_time(total_seconds: u32) -> String {
    let hours = total_seconds / 3_600;
    let days = hours / 24;
    format!("{days}d {}h", hours % 24)
}

pub fn format_money(copper: u32) -> String {
    let gold = copper / 10_000;
    let silver = (copper % 10_000) / 100;
    let copper = copper % 100;
    format!("{gold}g {silver}s {copper}c")
}

pub fn format_timestamp(unix_seconds: i64) -> String {
    if unix_seconds <= 0 {
        "Never".to_string()
    } else {
        unix_seconds.to_string()
    }
}

pub const PORTAL_CAPABILITIES: &[(&str, u8)] = &[
    ("View accounts", 1),
    ("Edit account profile fields", 3),
    ("Manage account locks and bans", 3),
    ("Assign roles", 6),
    ("View chat", 1),
    ("Moderate chat", 1),
    ("View moderation records", 1),
    ("Manage moderation actions", 3),
    ("View live map", 1),
    ("View audit logs", 4),
];

#[server]
pub async fn get_portal_overview() -> Result<PortalOverview, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use axum_extra::extract::cookie::CookieJar;
        use leptos_axum::extract;

        let state = expect_context::<crate::state::AppState>();
        let jar: CookieJar = extract().await?;
        let Some(cookie) = jar.get("oxcore_session") else {
            return Err(ServerFnError::ServerError("Not authenticated".to_string()));
        };
        let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await
        else {
            return Err(ServerFnError::ServerError("Not authenticated".to_string()));
        };

        return load_overview(&state, session.account_id)
            .await
            .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn has_admin_access() -> Result<bool, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        return crate::auth::has_gm_access(&state.auth, account_id)
            .await
            .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_active_sessions() -> Result<Vec<SessionSummary>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use axum_extra::extract::cookie::CookieJar;
        use leptos_axum::extract;

        let state = expect_context::<crate::state::AppState>();
        let jar: CookieJar = extract().await?;
        let Some(cookie) = jar.get("oxcore_session") else {
            return Err(ServerFnError::ServerError("Not authenticated".to_string()));
        };
        let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await
        else {
            return Err(ServerFnError::ServerError("Not authenticated".to_string()));
        };

        return load_sessions(
            &state,
            session.account_id,
            &crate::auth::session_token_hash_hex(cookie.value()),
        )
        .await
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_account_activity() -> Result<Vec<ActivityEvent>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        return sqlx::query_as::<_, (String, String, i64)>(
            "SELECT `action`, `target_type`, UNIX_TIMESTAMP(`occurred_at`) \
             FROM `web_audit_log` WHERE `actor_account_id` = ? \
             ORDER BY `occurred_at` DESC LIMIT 50",
        )
        .bind(account_id)
        .fetch_all(&*state.web)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(action, target_type, occurred_at)| ActivityEvent {
                    action,
                    target_type,
                    occurred_at,
                })
                .collect()
        })
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_character_detail(guid: u32) -> Result<Option<CharacterDetail>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        return sqlx::query_as::<_, (u32, String, u8, u8, u8, u8, u32, u32, u32)>(
            "SELECT `guid`, `name`, `race`, `class`, `level`, `online`, `zone`, `money`, \
             `played_time_total` FROM `characters` WHERE `guid` = ? AND `account` = ?",
        )
        .bind(guid)
        .bind(account_id)
        .fetch_optional(&*state.characters)
        .await
        .map(|row| {
            row.map(
                |(guid, name, race, class, level, online, zone, money, played_time_total)| {
                    CharacterDetail {
                        guid,
                        name,
                        race,
                        class,
                        level,
                        online,
                        zone,
                        money,
                        played_time_total,
                    }
                },
            )
        })
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_support_tickets() -> Result<Vec<SupportTicket>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        return sqlx::query_as::<_, (u64, String, String, i64, i64)>(
            "SELECT id, `subject`, `status`, UNIX_TIMESTAMP(`created_at`), \
             UNIX_TIMESTAMP(`updated_at`) FROM `web_support_tickets` \
             WHERE `account_id` = ? ORDER BY `updated_at` DESC",
        )
        .bind(account_id)
        .fetch_all(&*state.web)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(
                    |(id, subject, status, created_at, updated_at)| SupportTicket {
                        id,
                        subject,
                        status,
                        created_at,
                        updated_at,
                    },
                )
                .collect()
        })
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_realm_status() -> Result<Vec<RealmStatus>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let state = expect_context::<crate::state::AppState>();
        return sqlx::query_as::<_, (String, f32, i64)>(
            "SELECT `name`, `population`, IF(`last_seen` > DATE_SUB(UTC_TIMESTAMP(), INTERVAL 60 SECOND), 1, 0) \
             FROM `realmlist` ORDER BY id",
        )
        .fetch_all(&*state.auth)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(name, population, flag)| RealmStatus {
                    name,
                    population,
                    online: flag != 0,
                })
                .collect()
        })
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_overview() -> Result<AdminOverview, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        let is_gm = match crate::auth::has_gm_access(&state.auth, account_id).await {
            Ok(is_gm) => is_gm,
            Err(error) => {
                tracing::error!(target: "oxcore_web", %error, account_id, "portal GM authorization lookup failed");
                return Err(ServerFnError::ServerError(error.to_string()));
            }
        };
        if !is_gm {
            tracing::warn!(target: "oxcore_web", account_id, "portal GM authorization denied");
            return Err(ServerFnError::ServerError("Not authorized".to_string()));
        }
        let realms = get_realm_status().await?;
        let open_support_tickets = match sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM `web_support_tickets` WHERE `status` IN ('open', 'awaiting_player')",
        )
        .fetch_one(&*state.web)
        .await
        {
            Ok(count) => count as u64,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(AdminOverview {
            realms,
            open_support_tickets,
        });
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

// ========== Chat monitoring server functions ==========

/// Per-conversation volume for the chat page's channel browser.
#[server]
pub async fn get_chat_overview() -> Result<ChatOverview, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        require_gm_level(&state, account_id, 1).await?;
        let repository = ChatLogRepository::new(std::sync::Arc::clone(&state.logs));
        let total_messages = repository.message_count().await.unwrap_or(0) as u64;
        let channels = match repository.channel_summaries().await {
            Ok(rows) => rows
                .into_iter()
                .map(|row| ChatChannelSummary {
                    channel_type: row.channel_type,
                    channel_name: row.channel_name,
                    message_count: row.message_count as u64,
                    participants: row.participants as u64,
                    last_message_at: row.last_message_at,
                })
                .collect(),
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(ChatOverview {
            total_messages,
            channels,
        });
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

/// Message history for one conversation. `before_id` gives older-than pagination.
#[server]
pub async fn get_chat_channel(
    channel_type: String,
    channel_name: Option<String>,
    before_id: u64,
    limit: u32,
) -> Result<Vec<ChatMessage>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        require_gm_level(&state, account_id, 1).await?;
        let limit = limit.clamp(1, 500);
        let rows = match ChatLogRepository::new(std::sync::Arc::clone(&state.logs))
            .messages_for_channel(&channel_type, channel_name.as_deref(), before_id, limit)
            .await
        {
            Ok(rows) => rows,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(rows.into_iter().map(chat_row).collect());
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

/// Participant lookup: everyone who has spoken, newest activity first.
#[server]
pub async fn get_chat_participants(
    search: Option<String>,
) -> Result<Vec<ChatParticipant>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        require_gm_level(&state, account_id, 1).await?;
        let pattern = format!("%{}%", search.unwrap_or_default().trim());
        let rows = match ChatLogRepository::new(std::sync::Arc::clone(&state.logs))
            .participants(&pattern)
            .await
        {
            Ok(rows) => rows,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(rows
            .into_iter()
            .map(|row| ChatParticipant {
                guid: row.guid,
                name: row.name,
                account: row.account,
                message_count: row.message_count as u64,
                last_seen: row.last_seen,
            })
            .collect());
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

/// Full history for one player, across every channel/group/guild they spoke in.
#[server]
pub async fn get_player_chat(name: String) -> Result<ChatPlayerDetail, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        require_gm_level(&state, account_id, 1).await?;
        let name = name.trim();
        if name.is_empty() {
            return Err(ServerFnError::ServerError(
                "Enter a player name".to_string(),
            ));
        }
        let rows = match ChatLogRepository::new(std::sync::Arc::clone(&state.logs))
            .messages_for_player(name)
            .await
        {
            Ok(rows) => rows,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        let messages: Vec<ChatMessage> = rows.into_iter().map(chat_row).collect();

        let account_from_characters =
            sqlx::query_scalar::<_, i64>("SELECT `account` FROM `characters` WHERE `name` = ?")
                .bind(name)
                .fetch_optional(&*state.characters)
                .await
                .ok()
                .flatten()
                .map(|account| account as u32);
        let account_id = messages
            .iter()
            .find_map(|message| message.sender_account)
            .or(account_from_characters);

        let mut channels: Vec<String> = messages
            .iter()
            .filter_map(|message| message.channel_name.clone())
            .collect();
        channels.sort();
        channels.dedup();

        return Ok(ChatPlayerDetail {
            name: name.to_string(),
            account_id,
            messages,
            channels,
        });
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

/// Every message touching any character on an account (all their groups/guilds).
#[server]
pub async fn get_account_chat(account_id: u32) -> Result<Vec<ChatMessage>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        require_gm_level(&state, actor_id, 1).await?;
        let guids = match sqlx::query_scalar::<_, u32>(
            "SELECT `guid` FROM `characters` WHERE `account` = ?",
        )
        .bind(account_id)
        .fetch_all(&*state.characters)
        .await
        {
            Ok(guids) => guids,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };

        let rows = match ChatLogRepository::new(std::sync::Arc::clone(&state.logs))
            .messages_for_account(account_id, &guids)
            .await
        {
            Ok(rows) => rows,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(rows.into_iter().map(chat_row).collect());
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

/// Online characters of the acting GM account, available as in-game senders.
#[server]
pub async fn get_gm_characters() -> Result<Vec<PortalCharacter>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        require_gm_level(&state, account_id, 1).await?;
        let rows = match sqlx::query_as::<_, PortalCharacter>(
            "SELECT `guid`, `name`, `race`, `class`, `level`, `online` FROM `characters` \
             WHERE `account` = ? AND `online` <> 0 ORDER BY `guid`",
        )
        .bind(account_id)
        .fetch_all(&*state.characters)
        .await
        {
            Ok(rows) => rows,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(rows);
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

/// Queue a GM chat message for the world server to deliver as the chosen character.
#[server]
pub async fn send_chat_message(
    sender_guid: u32,
    channel_type: String,
    channel_name: Option<String>,
    target_name: Option<String>,
    message: String,
) -> Result<SendChatResult, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        require_gm_level(&state, account_id, 1).await?;
        let message = message.trim();
        if message.is_empty() || message.len() > 512 {
            return Ok(SendChatResult {
                accepted: false,
                note: "Message must be 1-512 characters".to_string(),
            });
        }

        let sender_online = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM `characters` WHERE `guid` = ? AND `account` = ? AND `online` <> 0",
        )
        .bind(sender_guid)
        .bind(account_id)
        .fetch_one(&*state.characters)
        .await
        .unwrap_or(0)
            > 0;
        if !sender_online {
            return Ok(SendChatResult {
                accepted: false,
                note: "The selected sender character is not online. Log that character in first."
                    .to_string(),
            });
        }

        let entry = match channel_type.as_str() {
            "Whisper" => {
                let target = target_name.unwrap_or_default().trim().to_string();
                if target.is_empty() {
                    return Ok(SendChatResult {
                        accepted: false,
                        note: "A whisper target is required".to_string(),
                    });
                }
                ChatOutboxInsert {
                    sender_account: account_id,
                    sender_guid,
                    channel_type,
                    channel_name: None,
                    target_name: Some(target),
                    message: message.to_string(),
                }
            }
            "Channel" => {
                let channel = channel_name.unwrap_or_default().trim().to_string();
                if channel.is_empty() {
                    return Ok(SendChatResult {
                        accepted: false,
                        note: "A channel name is required".to_string(),
                    });
                }
                ChatOutboxInsert {
                    sender_account: account_id,
                    sender_guid,
                    channel_type,
                    channel_name: Some(channel),
                    target_name: None,
                    message: message.to_string(),
                }
            }
            "Say" => ChatOutboxInsert {
                sender_account: account_id,
                sender_guid,
                channel_type,
                channel_name: None,
                target_name: None,
                message: message.to_string(),
            },
            _ => {
                return Ok(SendChatResult {
                    accepted: false,
                    note: "Unsupported chat type".to_string(),
                });
            }
        };
        if let Err(error) = ChatLogRepository::new(std::sync::Arc::clone(&state.logs))
            .enqueue_outbox(&entry)
            .await
        {
            return Err(ServerFnError::ServerError(error.to_string()));
        }
        return Ok(SendChatResult {
            accepted: true,
            note: "Queued for delivery".to_string(),
        });
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_accounts(
    search: Option<String>,
    page: u32,
) -> Result<AdminAccountPage, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, account_id, _) = authenticated_request().await?;
        let level = match crate::auth::highest_gm_level(&state.auth, account_id).await {
            Ok(level) => level,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        if level == 0 {
            return Err(ServerFnError::ServerError("Not authorized".to_string()));
        }
        const PAGE_SIZE: u32 = 50;
        let page = page.max(1);
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", search.trim());
        let total = match sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM `account` WHERE `username` LIKE ? OR CAST(id AS CHAR) LIKE ? OR `email` LIKE ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&*state.auth)
        .await {
            Ok(total) => total as u64,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        let accounts = match sqlx::query_as::<_, (u32, String, Option<String>, u8, u8, u8, i64)>(
            "SELECT id, `username`, `email`, `gmlevel`, `locked`, `banned`, UNIX_TIMESTAMP(`last_login`) \
              FROM `account` WHERE `username` LIKE ? OR CAST(id AS CHAR) LIKE ? OR `email` LIKE ? \
              ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(PAGE_SIZE)
        .bind((page - 1) * PAGE_SIZE)
        .fetch_all(&*state.auth)
        .await {
            Ok(accounts) => accounts,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        }
        .into_iter()
        .map(|(id, username, email, gmlevel, locked, banned, last_login)| AdminAccount { id, username, email, gmlevel, locked, banned, last_login })
        .collect();
        return Ok(AdminAccountPage {
            accounts,
            total,
            page,
            page_size: PAGE_SIZE,
        });
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_account(
    account_id: u32,
) -> Result<Option<AdminAccountDetail>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        if crate::auth::highest_gm_level(&state.auth, actor_id)
            .await
            .unwrap_or(0)
            == 0
        {
            return Err(ServerFnError::ServerError("Not authorized".to_string()));
        }
        return sqlx::query_as::<_, (u32, String, Option<String>, u8, u8, u8, u8, String, i64)>(
            "SELECT id, `username`, `email`, `gmlevel`, `locked`, `banned`, `expansion`, `last_ip`, `mutetime` FROM `account` WHERE id = ?",
        )
        .bind(account_id)
        .fetch_optional(&*state.auth)
        .await
        .map(|row| row.map(|(id, username, email, gmlevel, locked, banned, expansion, last_ip, muted_until)| AdminAccountDetail { id, username, email, gmlevel, locked, banned, expansion, last_ip, muted_until }))
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_account_characters(
    account_id: u32,
) -> Result<Vec<AdminCharacter>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        require_gm_level(&state, actor_id, 1).await?;
        return sqlx::query_as::<_, (u32, String, u8, u8, u8, u8)>(
            "SELECT `guid`, `name`, `race`, `class`, `level`, `online` FROM `characters` WHERE `account` = ? ORDER BY `guid`",
        )
        .bind(account_id)
        .fetch_all(&*state.characters)
        .await
        .map(|rows| rows.into_iter().map(|(guid, name, race, class, level, online)| AdminCharacter { guid, name, race, class, level, online }).collect())
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_characters(
    search: Option<String>,
    page: u32,
) -> Result<AdminCharacterPage, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        require_gm_level(&state, actor_id, 1).await?;
        const PAGE_SIZE: u32 = 50;
        let page = page.max(1);
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", search.trim());
        let total = match sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM `characters` WHERE `name` LIKE ? OR CAST(`guid` AS CHAR) LIKE ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&*state.characters)
        .await
        {
            Ok(total) => total as u64,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        let characters = match sqlx::query_as::<_, (u32, String, u8, u8, u8, u8)>(
            "SELECT `guid`, `name`, `race`, `class`, `level`, `online` FROM `characters` \
             WHERE `name` LIKE ? OR CAST(`guid` AS CHAR) LIKE ? ORDER BY `guid` LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(PAGE_SIZE)
        .bind((page - 1) * PAGE_SIZE)
        .fetch_all(&*state.characters)
        .await
        {
            Ok(characters) => characters,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        }
        .into_iter()
        .map(|(guid, name, race, class, level, online)| AdminCharacter {
            guid,
            name,
            race,
            class,
            level,
            online,
        })
        .collect();
        return Ok(AdminCharacterPage {
            characters,
            total,
            page,
            page_size: PAGE_SIZE,
        });
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_character_detail(
    guid: u32,
) -> Result<Option<AdminCharacterDetail>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;

        let (state, actor_id, _) = authenticated_request().await?;
        require_gm_level(&state, actor_id, 1).await?;
        let row = match sqlx::query(
            "SELECT `guid`, `name`, `account`, `race`, `class`, `gender`, `level`, `online`, \
             `zone`, `map`, `position_x`, `position_y`, `position_z`, `money`, `played_time_total`, \
             `create_time`, `logout_time` FROM `characters` WHERE `guid` = ?",
        )
        .bind(guid)
        .fetch_optional(&*state.characters)
        .await
        {
            Ok(row) => row,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        let Some(row) = row else {
            return Ok(None);
        };
        let create_time = row.get::<u64, _>("create_time") as i64;
        let logout_time = row.get::<u64, _>("logout_time") as i64;
        let account = row.get::<u32, _>("account");
        let account_username =
            match sqlx::query_scalar::<_, String>("SELECT `username` FROM `account` WHERE id = ?")
                .bind(account)
                .fetch_optional(&*state.auth)
                .await
            {
                Ok(username) => username,
                Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
            };
        return Ok(Some(AdminCharacterDetail {
            guid: row.get::<u32, _>("guid"),
            name: row.get::<String, _>("name"),
            account,
            account_username,
            race: row.get::<u8, _>("race"),
            class: row.get::<u8, _>("class"),
            gender: row.get::<u8, _>("gender"),
            level: row.get::<u8, _>("level"),
            online: row.get::<u8, _>("online"),
            zone: row.get::<u32, _>("zone"),
            map: row.get::<u32, _>("map"),
            position_x: row.get::<f32, _>("position_x"),
            position_y: row.get::<f32, _>("position_y"),
            position_z: row.get::<f32, _>("position_z"),
            money: row.get::<u32, _>("money"),
            played_time_total: row.get::<u32, _>("played_time_total"),
            create_time,
            logout_time,
        }));
    }

    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_account_sessions(
    account_id: u32,
) -> Result<Vec<AdminSession>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        require_gm_level(&state, actor_id, 3).await?;
        return sqlx::query_as::<_, (String, i64, i64, i64)>(
            "SELECT HEX(`token_hash`), UNIX_TIMESTAMP(`created_at`), UNIX_TIMESTAMP(`last_seen_at`), UNIX_TIMESTAMP(`expires_at`) FROM `web_sessions` WHERE `account_id` = ? ORDER BY `last_seen_at` DESC",
        )
        .bind(account_id)
        .fetch_all(&*state.web)
        .await
        .map(|rows| rows.into_iter().map(|(id, created_at, last_seen_at, expires_at)| AdminSession { account_id, id, created_at, last_seen_at, expires_at }).collect())
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_audit_log(
    account_id: Option<u32>,
) -> Result<Vec<AuditEntry>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        require_gm_level(&state, actor_id, if account_id.is_some() { 1 } else { 4 }).await?;
        let rows = match if let Some(account_id) = account_id {
            sqlx::query_as::<_, (u64, i64, Option<String>, String, String, Option<String>, Option<String>)>(
                "SELECT l.id, UNIX_TIMESTAMP(l.`occurred_at`), a.`username`, l.`action`, l.`target_type`, l.`target_id`, l.`reason` FROM `web_audit_log` l LEFT JOIN `auth`.`account` a ON a.id = l.`actor_account_id` WHERE l.`actor_account_id` = ? OR (l.`target_type` = 'account' AND l.`target_id` = CAST(? AS CHAR) COLLATE utf8mb4_unicode_ci) OR (l.`target_type` = 'realm_role' AND l.`target_id` LIKE CONCAT(CAST(? AS CHAR) COLLATE utf8mb4_unicode_ci, ':%')) ORDER BY l.`occurred_at` DESC LIMIT 100",
            ).bind(account_id).bind(account_id).bind(account_id).fetch_all(&*state.web).await
        } else {
            sqlx::query_as::<_, (u64, i64, Option<String>, String, String, Option<String>, Option<String>)>(
                "SELECT l.id, UNIX_TIMESTAMP(l.`occurred_at`), a.`username`, l.`action`, l.`target_type`, l.`target_id`, l.`reason` FROM `web_audit_log` l LEFT JOIN `auth`.`account` a ON a.id = l.`actor_account_id` ORDER BY l.`occurred_at` DESC LIMIT 200",
            ).fetch_all(&*state.web).await
        } {
            Ok(rows) => rows,
            Err(error) => return Err(ServerFnError::ServerError(error.to_string())),
        };
        return Ok(rows
            .into_iter()
            .map(
                |(id, occurred_at, actor, action, target_type, target_id, reason)| AuditEntry {
                    id,
                    occurred_at,
                    actor,
                    action,
                    target_type,
                    target_id,
                    reason,
                },
            )
            .collect());
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[server]
pub async fn get_admin_account_realm_access(
    account_id: u32,
) -> Result<Vec<RealmAccess>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let (state, actor_id, _) = authenticated_request().await?;
        if crate::auth::highest_gm_level(&state.auth, actor_id)
            .await
            .unwrap_or(0)
            == 0
        {
            return Err(ServerFnError::ServerError("Not authorized".to_string()));
        }
        return sqlx::query_as::<_, (i32, u8)>(
            "SELECT `RealmID`, `gmlevel` FROM `account_access` WHERE id = ? ORDER BY `RealmID`",
        )
        .bind(account_id)
        .fetch_all(&*state.auth)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(realm_id, gmlevel)| RealmAccess { realm_id, gmlevel })
                .collect()
        })
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!("server function body only runs on the server")
}

#[cfg(feature = "ssr")]
pub async fn overview(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(cookie) = jar.get("oxcore_session") else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };

    match load_overview(&state, session.account_id).await {
        Ok(overview) => axum::Json(overview).into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "portal overview query failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn live_map_players(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(cookie) = jar.get("oxcore_session") else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };
    if crate::auth::highest_gm_level(&state.auth, session.account_id)
        .await
        .unwrap_or(0)
        < 1
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }

    match sqlx::query_as::<_, (u32, String, u8, u8, u32, f32, f32, u32)>(
        "SELECT `guid`, `name`, `level`, `class`, `map`, `position_x`, `position_y`, `zone` \
         FROM `characters` WHERE `online` <> 0 ORDER BY `name`",
    )
    .fetch_all(&*state.characters)
    .await
    {
        Ok(rows) => axum::Json(
            rows.into_iter()
                .map(
                    |(guid, name, level, class, map, position_x, position_y, zone)| LiveMapPlayer {
                        guid,
                        name,
                        level,
                        class,
                        map,
                        position_x,
                        position_y,
                        zone,
                    },
                )
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "live map player query failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Query params for the live chat feed endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct ChatLiveParams {
    /// Only messages with `id > since`. Omit/0 to fetch the most recent `limit`.
    pub since: Option<u64>,
    pub channel_type: Option<String>,
    pub channel_name: Option<String>,
    pub player: Option<String>,
    pub limit: Option<u32>,
}

/// Polled every ~1.5s by `chat-live.js`. Returns new messages (or, on first load,
/// the most recent batch in descending order so the client can render them reversed).
#[cfg(feature = "ssr")]
pub async fn chat_live_feed(
    axum::Extension(state): axum::Extension<crate::state::AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
    axum::extract::Query(params): axum::extract::Query<ChatLiveParams>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let Some(cookie) = jar.get("oxcore_session") else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };
    if crate::auth::highest_gm_level(&state.auth, session.account_id)
        .await
        .unwrap_or(0)
        < 1
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }

    let since = params.since.unwrap_or(0);
    let limit = params.limit.unwrap_or(300).clamp(1, 1000);
    match ChatLogRepository::new(std::sync::Arc::clone(&state.logs))
        .live_messages(
            since,
            params.channel_type.as_deref(),
            params.channel_name.as_deref(),
            params.player.as_deref(),
            limit,
        )
        .await
    {
        Ok(rows) => axum::Json(rows.into_iter().map(chat_row).collect::<Vec<_>>()).into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "chat live feed query failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(feature = "ssr")]
async fn load_overview(
    state: &crate::state::AppState,
    account_id: u32,
) -> anyhow::Result<PortalOverview> {
    use anyhow::Context;

    let (username, email, email_verified) = sqlx::query_as::<_, (String, Option<String>, bool)>(
        "SELECT `username`, `email`, `email_verif` FROM `account` WHERE id = ?",
    )
    .bind(account_id)
    .fetch_optional(&*state.auth)
    .await
    .context("failed to load portal account")?
    .context("portal account no longer exists")?;

    let characters = sqlx::query_as::<_, PortalCharacter>(
        "SELECT `guid`, `name`, `race`, `class`, `level`, `online` \
         FROM `characters` WHERE `account` = ? ORDER BY `guid`",
    )
    .bind(account_id)
    .fetch_all(&*state.characters)
    .await
    .context("failed to load portal characters")?;

    Ok(PortalOverview {
        username,
        email,
        email_verified,
        characters,
    })
}

#[cfg(feature = "ssr")]
async fn load_sessions(
    state: &crate::state::AppState,
    account_id: u32,
    current_session_id: &str,
) -> anyhow::Result<Vec<SessionSummary>> {
    use anyhow::Context;

    let rows = sqlx::query_as::<_, (String, i64, i64, i64)>(
        "SELECT HEX(`token_hash`), UNIX_TIMESTAMP(`created_at`), UNIX_TIMESTAMP(`last_seen_at`), \
         UNIX_TIMESTAMP(`expires_at`) FROM `web_sessions` WHERE `account_id` = ? \
         AND `expires_at` > UTC_TIMESTAMP() ORDER BY `last_seen_at` DESC",
    )
    .bind(account_id)
    .fetch_all(&*state.web)
    .await
    .context("failed to load active sessions")?;

    Ok(rows
        .into_iter()
        .map(
            |(id, created_at, last_seen_at, expires_at)| SessionSummary {
                current: id.eq_ignore_ascii_case(current_session_id),
                id,
                created_at,
                last_seen_at,
                expires_at,
            },
        )
        .collect())
}

#[cfg(feature = "ssr")]
async fn authenticated_request() -> Result<(crate::state::AppState, u32, String), ServerFnError> {
    use axum_extra::extract::cookie::CookieJar;
    use leptos_axum::extract;

    let state = expect_context::<crate::state::AppState>();
    let jar: CookieJar = extract().await?;
    let Some(cookie) = jar.get("oxcore_session") else {
        return Err(ServerFnError::ServerError("Not authenticated".to_string()));
    };
    let Some(session) = crate::auth::session_from_token(&state.web, cookie.value()).await else {
        return Err(ServerFnError::ServerError("Not authenticated".to_string()));
    };
    Ok((state, session.account_id, cookie.value().to_string()))
}

#[cfg(feature = "ssr")]
async fn require_gm_level(
    state: &crate::state::AppState,
    account_id: u32,
    required: u8,
) -> Result<(), ServerFnError> {
    match crate::auth::highest_gm_level(&state.auth, account_id).await {
        Ok(level) if level >= required => Ok(()),
        Ok(_) => Err(ServerFnError::ServerError("Not authorized".to_string())),
        Err(error) => Err(ServerFnError::ServerError(error.to_string())),
    }
}
