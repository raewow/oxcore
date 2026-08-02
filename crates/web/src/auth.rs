use anyhow::{Context, Result};
use axum::extract::Form;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Extension;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use oxcore_db::database::{web::repositories::WebRepository, AccountRepository};
use oxcore_shared::crypto::srp6v2;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{MySqlPool, PgPool};

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
}

#[derive(Debug, Deserialize)]
pub struct AdminBanForm {
    #[serde(default)]
    pub active: bool,
    pub reason: String,
    #[serde(default)]
    pub duration_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct AdminMuteForm {
    #[serde(default)]
    pub active: bool,
    pub reason: String,
    #[serde(default)]
    pub duration_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct AdminSessionRevokeForm {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminRealmRoleForm {
    pub realm_id: i32,
    pub gmlevel: u8,
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
    let result = WebRepository::new(std::sync::Arc::clone(&state.web))
        .delete_account_session_by_hex(session.account_id, &form.session_id)
        .await;
    let Ok(result) = result else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if result != 1 {
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

    let result = WebRepository::new(std::sync::Arc::clone(&state.web))
        .create_support_ticket(session.account_id, subject, message)
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
    let result =
        sqlx::query("UPDATE `account` SET `email` = ?, `locked` = ?, `gmlevel` = ? WHERE `id` = ?")
            .bind((!email.is_empty()).then_some(email))
            .bind(form.locked)
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
    let _ = record_audit_for(
        &state.web,
        session.account_id,
        "account.updated",
        "account",
        Some(&account_id.to_string()),
        None,
    )
    .await;
    Redirect::to(&format!("/admin/accounts/{account_id}")).into_response()
}

pub async fn update_admin_ban(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    axum::extract::Path(account_id): axum::extract::Path<u32>,
    Form(form): Form<AdminBanForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/login").into_response();
    };
    let Ok(actor_level) = highest_gm_level(&state.auth, session.account_id).await else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if actor_level < 3
        || !can_manage_target(&state.auth, session.account_id, account_id, actor_level).await
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let reason = form.reason.trim();
    if (form.active && reason.is_empty()) || reason.len() > 255 {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let mut transaction = match state.auth.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let result = if form.active {
        sqlx::query("UPDATE `account` SET `banned` = 1 WHERE `id` = ?")
            .bind(account_id)
            .execute(&mut *transaction)
            .await
    } else {
        sqlx::query("UPDATE `account` SET `banned` = 0 WHERE `id` = ?")
            .bind(account_id)
            .execute(&mut *transaction)
            .await
    };
    if !matches!(result, Ok(result) if result.rows_affected() == 1) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    if form.active {
        let expires_at = form.duration_seconds.min(i64::MAX as u64) as i64;
        if sqlx::query("INSERT INTO `account_banned` (`id`, `bandate`, `unbandate`, `bannedby`, `banreason`, `active`, `realm`, `gmlevel`) VALUES (?, UNIX_TIMESTAMP(), IF(? = 0, 0, UNIX_TIMESTAMP() + ?), ?, ?, 1, -1, ?)")
            .bind(account_id).bind(expires_at).bind(expires_at).bind(session.account_id.to_string()).bind(reason).bind(actor_level).execute(&mut *transaction).await.is_err() { return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(); }
    } else if sqlx::query(
        "UPDATE `account_banned` SET `active` = 0 WHERE `id` = ? AND `active` = 1",
    )
    .bind(account_id)
    .execute(&mut *transaction)
    .await
    .is_err()
    {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    if transaction.commit().await.is_err() {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let action = if form.active {
        "account.banned"
    } else {
        "account.unbanned"
    };
    let _ = record_audit_for(
        &state.web,
        session.account_id,
        action,
        "account",
        Some(&account_id.to_string()),
        (!reason.is_empty()).then_some(reason),
    )
    .await;
    Redirect::to(&format!("/admin/accounts/{account_id}")).into_response()
}

pub async fn update_admin_mute(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    axum::extract::Path(account_id): axum::extract::Path<u32>,
    Form(form): Form<AdminMuteForm>,
) -> Response {
    if !has_same_origin(&headers, &state) {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/login").into_response();
    };
    let Ok(actor_level) = highest_gm_level(&state.auth, session.account_id).await else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if actor_level < 3
        || !can_manage_target(&state.auth, session.account_id, account_id, actor_level).await
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let reason = form.reason.trim();
    if (form.active && (reason.is_empty() || form.duration_seconds == 0)) || reason.len() > 255 {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let expires_at = form.duration_seconds.min(i64::MAX as u64) as i64;
    let result = sqlx::query("UPDATE `account` SET `mutetime` = IF(? = 1, UNIX_TIMESTAMP() + ?, 0), `mutereason` = IF(? = 1, ?, ''), `muteby` = IF(? = 1, ?, '') WHERE `id` = ?")
        .bind(form.active).bind(expires_at).bind(form.active).bind(reason).bind(form.active).bind(session.account_id.to_string()).bind(account_id).execute(&*state.auth).await;
    if !matches!(result, Ok(result) if result.rows_affected() == 1) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    let action = if form.active {
        "account.muted"
    } else {
        "account.unmuted"
    };
    let _ = record_audit_for(
        &state.web,
        session.account_id,
        action,
        "account",
        Some(&account_id.to_string()),
        (!reason.is_empty()).then_some(reason),
    )
    .await;
    Redirect::to(&format!("/admin/accounts/{account_id}")).into_response()
}

pub async fn revoke_admin_session(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    axum::extract::Path(account_id): axum::extract::Path<u32>,
    Form(form): Form<AdminSessionRevokeForm>,
) -> Response {
    if !has_same_origin(&headers, &state) || !valid_session_id(&form.session_id) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/login").into_response();
    };
    let Ok(actor_level) = highest_gm_level(&state.auth, session.account_id).await else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if actor_level < 3
        || !can_manage_target(&state.auth, session.account_id, account_id, actor_level).await
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let result = WebRepository::new(std::sync::Arc::clone(&state.web))
        .delete_account_session_by_hex(account_id, &form.session_id)
        .await;
    if !matches!(result, Ok(1)) {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }
    let _ = record_audit_for(
        &state.web,
        session.account_id,
        "session.revoked_by_admin",
        "account",
        Some(&account_id.to_string()),
        None,
    )
    .await;
    Redirect::to(&format!("/admin/accounts/{account_id}")).into_response()
}

pub async fn update_admin_realm_role(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    axum::extract::Path(account_id): axum::extract::Path<u32>,
    Form(form): Form<AdminRealmRoleForm>,
) -> Response {
    if !has_same_origin(&headers, &state) || form.gmlevel > 7 {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/login").into_response();
    };
    let Ok(actor_level) = highest_gm_level(&state.auth, session.account_id).await else {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if actor_level < 6
        || form.gmlevel >= actor_level
        || !can_manage_target(&state.auth, session.account_id, account_id, actor_level).await
    {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let result = if form.gmlevel == 0 {
        sqlx::query("DELETE FROM `account_access` WHERE `id` = ? AND `RealmID` = ?")
            .bind(account_id)
            .bind(form.realm_id)
            .execute(&*state.auth)
            .await
    } else {
        sqlx::query("INSERT INTO `account_access` (`id`, `RealmID`, `gmlevel`) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE `gmlevel` = VALUES(`gmlevel`)").bind(account_id).bind(form.realm_id).bind(form.gmlevel).execute(&*state.auth).await
    };
    if result.is_err() {
        return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let action = if form.gmlevel == 0 {
        "realm_role.removed"
    } else {
        "realm_role.updated"
    };
    let target = format!("{account_id}:{}", form.realm_id);
    let _ = record_audit_for(
        &state.web,
        session.account_id,
        action,
        "realm_role",
        Some(&target),
        None,
    )
    .await;
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

async fn create_session(pool: &PgPool, account_id: u32) -> Result<String> {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = token_hash(&token);

    WebRepository::new(std::sync::Arc::new(pool.clone()))
        .create_session(hash.as_slice(), account_id)
        .await
        .context("failed to store web session")?;

    Ok(token)
}

async fn delete_session(pool: &PgPool, token: &str) -> Result<()> {
    let hash = token_hash(token);
    WebRepository::new(std::sync::Arc::new(pool.clone()))
        .delete_session(hash.as_slice())
        .await
        .context("failed to delete web session")?;
    Ok(())
}

async fn delete_account_sessions(pool: &PgPool, account_id: u32) -> Result<()> {
    WebRepository::new(std::sync::Arc::new(pool.clone()))
        .delete_account_sessions(account_id)
        .await
        .context("failed to revoke account web sessions")?;
    Ok(())
}

async fn record_audit(
    pool: &PgPool,
    account_id: u32,
    action: &str,
    target_type: &str,
) -> Result<()> {
    record_audit_for(pool, account_id, action, target_type, None, None).await
}

async fn record_audit_for(
    pool: &PgPool,
    account_id: u32,
    action: &str,
    target_type: &str,
    target_id: Option<&str>,
    reason: Option<&str>,
) -> Result<()> {
    WebRepository::new(std::sync::Arc::new(pool.clone()))
        .record_audit(account_id, action, target_type, target_id, reason)
        .await
        .context("failed to write web audit event")?;
    Ok(())
}

async fn can_manage_target(
    pool: &MySqlPool,
    actor_id: u32,
    target_id: u32,
    actor_level: u8,
) -> bool {
    if actor_id == target_id {
        return false;
    }
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM `account` WHERE `id` = ?")
        .bind(target_id)
        .fetch_one(pool)
        .await;
    matches!(exists, Ok(1))
        && matches!(highest_gm_level(pool, target_id).await, Ok(target_level) if target_level < actor_level)
}

fn valid_session_id(session_id: &str) -> bool {
    session_id.len() == 64 && session_id.bytes().all(|byte| byte.is_ascii_hexdigit())
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

async fn current_session(pool: &PgPool, jar: &CookieJar) -> Option<Session> {
    let token = jar.get(SESSION_COOKIE)?.value();
    session_from_token(pool, token).await
}

pub async fn session_from_token(pool: &PgPool, token: &str) -> Option<Session> {
    let hash = token_hash(token);
    let account_id = WebRepository::new(std::sync::Arc::new(pool.clone()))
        .session_account(hash.as_slice())
        .await
        .ok()??;

    // Session activity is advisory metadata; an update failure must not turn a valid session into
    // an authentication failure.
    let _ = WebRepository::new(std::sync::Arc::new(pool.clone()))
        .touch_session(hash.as_slice())
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
