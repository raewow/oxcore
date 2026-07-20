//! HTTPS REST login service.
//!
//! This is the first thing a modern client talks to. It fetches the portal address, renders
//! the login form in its embedded browser, runs SRP6v2 against us, and walks away with a
//! login ticket that it later presents on the BGS RPC channel.

pub mod types;

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::debug;

use crate::config::Config;

pub struct RestState {
    pub config: Config,
}

pub fn router(state: Arc<RestState>) -> Router {
    Router::new()
        .route("/bnetserver/portal/", get(get_portal))
        .route("/bnetserver/login/", get(get_login_form).post(post_login))
        .route("/bnetserver/login/srp/", post(post_login_srp))
        .route("/bnetserver/gameAccounts/", get(get_game_accounts))
        .route("/bnetserver/refreshLoginTicket/", post(refresh_login_ticket))
        .with_state(state)
}

/// `GET /bnetserver/portal/` — plain `host:port` of the BGS RPC channel, not JSON.
async fn get_portal(State(state): State<Arc<RestState>>) -> impl IntoResponse {
    let address = state.config.portal_address();
    debug!(%address, "serving portal address");
    address
}

/// `GET /bnetserver/login/` — describes the form the client should render.
async fn get_login_form(State(state): State<Arc<RestState>>) -> impl IntoResponse {
    Json(types::login_form(Some(format!(
        "{}srp/",
        state.config.login_base_url()
    ))))
}

/// `POST /bnetserver/login/srp/` — SRP6v2 challenge. Implemented in M2.
async fn post_login_srp() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

/// `POST /bnetserver/login/` — verify SRP evidence, issue a ticket. Implemented in M2.
async fn post_login() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

/// `GET /bnetserver/gameAccounts/` — implemented in M4 alongside the account services.
async fn get_game_accounts() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

/// `POST /bnetserver/refreshLoginTicket/` — implemented in M2 with ticket storage.
async fn refresh_login_ticket() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}
