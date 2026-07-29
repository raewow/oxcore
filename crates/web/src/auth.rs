use anyhow::{Context, Result};
use axum::extract::Form;
use axum::response::{Html, IntoResponse, Redirect, Response};
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

#[derive(Debug, Clone, Copy)]
pub struct Session {
    pub account_id: u32,
}

pub async fn login(
    Extension(state): Extension<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Response {
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
    (jar.add(cookie), Redirect::to("/account")).into_response()
}

pub async fn register(
    Extension(state): Extension<AppState>,
    jar: CookieJar,
    Form(form): Form<RegistrationForm>,
) -> Response {
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
    (jar.add(cookie), Redirect::to("/account")).into_response()
}

pub async fn logout(Extension(state): Extension<AppState>, jar: CookieJar) -> Response {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        if let Err(error) = delete_session(&state.web, cookie.value()).await {
            tracing::error!(target: "oxcore_web", %error, "web session deletion failed");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    (jar.remove(Cookie::from(SESSION_COOKIE)), Redirect::to("/")).into_response()
}

pub async fn account(Extension(state): Extension<AppState>, jar: CookieJar) -> Response {
    let Some(session) = current_session(&state.web, &jar).await else {
        return Redirect::to("/").into_response();
    };

    Html(format!(
        r#"<main><h1>Account portal</h1><p>Account {}</p><form action="/auth/logout" method="post"><button type="submit">Sign out</button></form></main>"#,
        session.account_id
    ))
    .into_response()
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

    Some(Session { account_id })
}

async fn has_gm_access(pool: &MySqlPool, account_id: u32) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(SELECT 1 FROM `account` WHERE `id` = ? AND `gmlevel` > 0) \
         OR EXISTS(SELECT 1 FROM `account_access` WHERE `id` = ? AND `gmlevel` > 0)",
    )
    .bind(account_id)
    .bind(account_id)
    .fetch_one(pool)
    .await
    .context("failed to query GM access")?;
    Ok(exists != 0)
}

fn token_hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
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
