use leptos::prelude::*;
use serde::{Deserialize, Serialize};

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
            "SELECT `id`, `subject`, `status`, UNIX_TIMESTAMP(`created_at`), \
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
             FROM `realmlist` ORDER BY `id`",
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

#[server]
pub async fn get_admin_accounts(
    search: Option<String>,
) -> Result<Vec<AdminAccount>, ServerFnError> {
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
        let search = search.unwrap_or_default();
        let pattern = format!("%{}%", search.trim());
        return sqlx::query_as::<_, (u32, String, Option<String>, u8, u8, u8, i64)>(
            "SELECT `id`, `username`, `email`, `gmlevel`, `locked`, `banned`, UNIX_TIMESTAMP(`last_login`) \
             FROM `account` WHERE `username` LIKE ? OR CAST(`id` AS CHAR) LIKE ? OR `email` LIKE ? \
             ORDER BY `id` DESC LIMIT 100",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&*state.auth)
        .await
        .map(|rows| rows.into_iter().map(|(id, username, email, gmlevel, locked, banned, last_login)| AdminAccount { id, username, email, gmlevel, locked, banned, last_login }).collect())
        .map_err(|error| ServerFnError::ServerError(error.to_string()));
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
        return sqlx::query_as::<_, (u32, String, Option<String>, u8, u8, u8, u8, String)>(
            "SELECT `id`, `username`, `email`, `gmlevel`, `locked`, `banned`, `expansion`, `last_ip` FROM `account` WHERE `id` = ?",
        )
        .bind(account_id)
        .fetch_optional(&*state.auth)
        .await
        .map(|row| row.map(|(id, username, email, gmlevel, locked, banned, expansion, last_ip)| AdminAccountDetail { id, username, email, gmlevel, locked, banned, expansion, last_ip }))
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
async fn load_overview(
    state: &crate::state::AppState,
    account_id: u32,
) -> anyhow::Result<PortalOverview> {
    use anyhow::Context;

    let (username, email, email_verified) = sqlx::query_as::<_, (String, Option<String>, bool)>(
        "SELECT `username`, `email`, `email_verif` FROM `account` WHERE `id` = ?",
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
