use anyhow::{Context, Result};
use axum::extract::Form;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Extension;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use oxcore_shared::crypto::srp6v2;
use oxcore_shared::database::AccountRepository;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;

use crate::state::AppState;

const SESSION_COOKIE: &str = "oxcore_session";

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordChangeForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionRevokeForm {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SupportTicketForm {
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminAccountForm {
    pub email: String,
    pub gmlevel: u8,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub banned: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Session {
    pub account_id: u32,
}

pub async fn login(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let repository = AccountRepository::new(state.auth.clone());
    let account_id = match repository.find_bnet_credentials(&form.username).await {
        Ok(Some(credentials)) => srp6v2::verify_password(
            &form.username,
            &form.password,
            credentials.salt,
            &credentials.verifier,
        )
        .then_some(credentials.id),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "web login credential lookup failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let Some(account_id) = account_id else {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    };

    let token = match create_session(&state.web, account_id).await {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "web session creation failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let cookie = session_cookie(token, state.secure_cookies);
    let _ = record_audit(&state.web, account_id, "session.created", "session").await;
    (jar.add(cookie), Redirect::to("/account")).into_response()
}

pub async fn register(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<RegistrationForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    if !is_valid_email(&form.email) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }

    let repository = AccountRepository::new(state.auth.clone());
    let account_id = match repository
        .create_account(&form.username, &form.password)
        .await
    {
        Ok(account_id) => account_id,
        Err(error) => {
            tracing::warn!(target: "oxcore_web", %error, "web account registration rejected");
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
    };

    if let Err(error) =
        sqlx::query("UPDATE `account` SET `email` = ?, `reg_mail` = ? WHERE `id` = ?")
            .bind(&form.email)
            .bind(&form.email)
            .bind(account_id)
            .execute(&*state.auth)
            .await
    {
        tracing::error!(target: "oxcore_web", %error, account_id, "web account email update failed");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let token = match create_session(&state.web, account_id).await {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, account_id, "web registration session creation failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let cookie = session_cookie(token, state.secure_cookies);
    let _ = record_audit(&state.web, account_id, "account.registered", "account").await;
    (jar.add(cookie), Redirect::to("/account")).into_response()
}

pub async fn logout(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let session = session_from_token(&state.web, cookie.value()).await;
        if let Err(error) = delete_session(&state.web, cookie.value()).await {
            tracing::error!(target: "oxcore_web", %error, "web session deletion failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        if let Some(session) = session {
            let _ = record_audit(&state.web, session.account_id, "session.ended", "session").await;
        }
    }

    (jar.remove(Cookie::from(SESSION_COOKIE)), Redirect::to("/")).into_response()
}

pub async fn change_password(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<PasswordChangeForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    if form.new_password != form.confirm_password {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Redirect::to("/login").into_response();
    };
    let Some(session) = session_from_token(&state.web, cookie.value()).await else {
        return Redirect::to("/login").into_response();
    };

    let username = match sqlx::query_scalar::<_, String>(
        "SELECT `username` FROM `account` WHERE `id` = ?",
    )
    .bind(session.account_id)
    .fetch_optional(&*state.auth)
    .await
    {
        Ok(Some(username)) => username,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "password change account lookup failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let repository = AccountRepository::new(state.auth.clone());
    let current_credentials = match repository.find_bnet_credentials(&username).await {
        Ok(Some(credentials)) => credentials,
        Ok(None) => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "password change credential lookup failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    if !srp6v2::verify_password(
        &username,
        &form.current_password,
        current_credentials.salt,
        &current_credentials.verifier,
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }

    if let Err(error) = repository
        .set_password(session.account_id, &username, &form.new_password)
        .await
    {
        tracing::warn!(target: "oxcore_web", %error, account_id = session.account_id, "password change rejected");
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    if let Err(error) = delete_account_sessions(&state.web, session.account_id).await {
        tracing::error!(target: "oxcore_web", %error, account_id = session.account_id, "password change session revocation failed");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let _ = record_audit(
        &state.web,
        session.account_id,
        "password.changed",
        "account",
    )
    .await;

    (
        jar.remove(Cookie::from(SESSION_COOKIE)),
        Redirect::to("/login"),
    )
        .into_response()
}

pub async fn revoke_session(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<SessionRevokeForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    if form.session_id.len() != 64 || !form.session_id.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Redirect::to("/login").into_response();
    };
    let Some(session) = session_from_token(&state.web, cookie.value()).await else {
        return Redirect::to("/login").into_response();
    };

    let current_session_id = session_token_hash_hex(cookie.value());
    let result = sqlx::query(
        "DELETE FROM `web_sessions` WHERE `account_id` = ? AND `token_hash` = UNHEX(?)",
    )
    .bind(session.account_id)
    .bind(&form.session_id)
    .execute(&*state.web)
    .await;
    let Ok(result) = result else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if result.rows_affected() != 1 {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    let _ = record_audit(&state.web, session.account_id, "session.revoked", "session").await;

    if form.session_id.eq_ignore_ascii_case(&current_session_id) {
        return (
            jar.remove(Cookie::from(SESSION_COOKIE)),
            Redirect::to("/login"),
        )
            .into_response();
    }
    Redirect::to("/security").into_response()
}

pub async fn create_support_ticket(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<SupportTicketForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let subject = form.subject.trim();
    let message = form.message.trim();
    if subject.is_empty() || subject.len() > 160 || message.is_empty() || message.len() > 8_000 {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Redirect::to("/login").into_response();
    };
    let Some(session) = session_from_token(&state.web, cookie.value()).await else {
        return Redirect::to("/login").into_response();
    };

    let result = sqlx::query(
        "INSERT INTO `web_support_tickets` (`account_id`, `subject`, `message`) VALUES (?, ?, ?)",
    )
    .bind(session.account_id)
    .bind(subject)
    .bind(message)
    .execute(&*state.web)
    .await;
    if let Err(error) = result {
        tracing::error!(target: "oxcore_web", %error, account_id = session.account_id, "support ticket creation failed");
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let _ = record_audit(
        &state.web,
        session.account_id,
        "support_ticket.created",
        "support_ticket",
    )
    .await;
    Redirect::to("/support").into_response()
}

pub async fn update_admin_account(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    axum::extract::Path(account_id): axum::extract::Path<u32>,
    Form(form): Form<AdminAccountForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/login").into_response();
    };
    let actor_level = match highest_gm_level(&state.auth, session.account_id).await {
        Ok(level) => level,
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if actor_level < 3 || form.gmlevel > 7 || account_id == session.account_id {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let target_level = match highest_gm_level(&state.auth, account_id).await {
        Ok(level) => level,
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if target_level >= actor_level
        || (form.gmlevel != target_level && (actor_level < 6 || form.gmlevel >= actor_level))
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let email = form.email.trim();
    if !email.is_empty() && !is_valid_email(email) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let result = sqlx::query(
        "UPDATE `account` SET `email` = ?, `locked` = ?, `banned` = ?, `gmlevel` = ? WHERE `id` = ?",
    )
    .bind((!email.is_empty()).then_some(email))
    .bind(form.locked)
    .bind(form.banned)
    .bind(form.gmlevel)
    .bind(account_id)
    .execute(&*state.auth)
    .await;
    match result {
        Ok(result) if result.rows_affected() == 1 => {}
        Ok(_) => return axum::http::StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, account_id, "admin account update failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    let _ = record_audit(&state.web, session.account_id, "account.updated", "account").await;
    Redirect::to(&format!("/admin/accounts/{account_id}")).into_response()
}

pub async fn admin(Extension(state): Extension<AppState>, jar: CookieJar) -> Response {
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/").into_response();
    };

    match has_gm_access(&state.auth, session.account_id).await {
        Ok(true) => "Admin console".into_response(),
        Ok(false) => axum::http::StatusCode::FORBIDDEN.into_response(),
        Err(error) => {
            tracing::error!(target: "oxcore_web", %error, "GM access lookup failed");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn create_session(pool: &MySqlPool, account_id: u32) -> Result<String> {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = token_hash(&token);

    sqlx::query(
        "INSERT INTO `web_sessions` (`token_hash`, `account_id`, `expires_at`) \
         VALUES (?, ?, DATE_ADD(UTC_TIMESTAMP(), INTERVAL 30 DAY))",
    )
    .bind(hash.as_slice())
    .bind(account_id)
    .execute(pool)
    .await
    .context("failed to store web session")?;

    Ok(token)
}

async fn delete_session(pool: &MySqlPool, token: &str) -> Result<()> {
    let hash = token_hash(token);
    sqlx::query("DELETE FROM `web_sessions` WHERE `token_hash` = ?")
        .bind(hash.as_slice())
        .execute(pool)
        .await
        .context("failed to delete web session")?;
    Ok(())
}

async fn delete_account_sessions(pool: &MySqlPool, account_id: u32) -> Result<()> {
    sqlx::query("DELETE FROM `web_sessions` WHERE `account_id` = ?")
        .bind(account_id)
        .execute(pool)
        .await
        .context("failed to revoke account web sessions")?;
    Ok(())
}

async fn record_audit(
    pool: &MySqlPool,
    account_id: u32,
    action: &str,
    target_type: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO `web_audit_log` (`actor_account_id`, `action`, `target_type`) VALUES (?, ?, ?)",
    )
    .bind(account_id)
    .bind(action)
    .bind(target_type)
    .execute(pool)
    .await
    .context("failed to write web audit event")?;
    Ok(())
}

fn has_same_origin(headers: &HeaderMap, state: &AppState) -> bool {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|value| value.to_str().ok());
    let expected_scheme = if state.secure_cookies {
        "https://"
    } else {
        "http://"
    };
    let allowed = origin
        .and_then(|origin| origin.strip_prefix(expected_scheme))
        .and_then(|authority| authority.split('/').next())
        .zip(host)
        .is_some_and(|(authority, host)| authority.eq_ignore_ascii_case(host));

    if !allowed {
        tracing::error!(
            target: "oxcore_web",
            ?origin,
            ?host,
            expected_scheme,
            configured_origin = %state.public_origin,
            "portal rejected cross-origin form request"
        );
    }
    allowed
}

async fn current_session(pool: &MySqlPool, jar: &CookieJar) -> Option<Session> {
    let token = jar.get(SESSION_COOKIE)?.value();
    session_from_token(pool, token).await
}

pub async fn session_from_token(pool: &MySqlPool, token: &str) -> Option<Session> {
    let hash = token_hash(token);
    let account_id = sqlx::query_scalar::<_, u32>(
        "SELECT `account_id` FROM `web_sessions` \
         WHERE `token_hash` = ? AND `expires_at` > UTC_TIMESTAMP()",
    )
    .bind(hash.as_slice())
    .fetch_optional(pool)
    .await
    .ok()??;

    // Session activity is advisory metadata; an update failure must not turn a valid session into
    // an authentication failure.
    let _ = sqlx::query(
        "UPDATE `web_sessions` SET `last_seen_at` = UTC_TIMESTAMP() WHERE `token_hash` = ?",
    )
    .bind(hash.as_slice())
    .execute(pool)
    .await;

    Some(Session { account_id })
}

pub(crate) async fn has_gm_access(pool: &MySqlPool, account_id: u32) -> Result<bool> {
    Ok(highest_gm_level(pool, account_id).await? > 0)
}

pub(crate) async fn highest_gm_level(pool: &MySqlPool, account_id: u32) -> Result<u8> {
    let account_level =
        sqlx::query_scalar::<_, u8>("SELECT `gmlevel` FROM `account` WHERE `id` = ?")
            .bind(account_id)
            .fetch_optional(pool)
            .await
            .context("failed to query GM access")?;
    let realm_level = sqlx::query_scalar::<_, u8>(
        "SELECT `gmlevel` FROM `account_access` WHERE `id` = ? ORDER BY `gmlevel` DESC LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .context("failed to query realm GM access")?;
    Ok(account_level.unwrap_or(0).max(realm_level.unwrap_or(0)))
}

fn token_hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub fn session_token_hash_hex(token: &str) -> String {
    hex::encode_upper(token_hash(token))
}

fn session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .build()
}

fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    email.len() <= 254
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}
