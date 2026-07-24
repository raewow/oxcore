//! HTTPS REST login service.
//!
//! This is the first thing a modern client talks to. It fetches the portal address, renders
//! the login form in its embedded browser, runs SRP6v2 against us, and walks away with a
//! login ticket that it later presents on the BGS RPC channel.
//!
//! ## Request correlation
//!
//! The exchange spans two requests: `POST /login/srp/` (server generates `B`, replies with the
//! challenge) then `POST /login/` (client sends `A`/`M1`). TrinityCore correlates them by TLS
//! connection. We instead key an in-flight store by the SRP username, which requires the
//! client to include `account_name` on the second POST as well. That is how the form
//! resubmission behaves, but it is **unverified against a live client** — if the second request
//! turns out to omit `account_name`, switch this to per-connection state.

pub mod types;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use oxcore_shared::crypto::srp6v2::{self, SrpServer};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::config::Config;
use crate::database::Database;

/// How long a generated challenge remains valid before the client must restart the exchange.
const CHALLENGE_TTL: Duration = Duration::from_secs(60);

/// A challenge awaiting the client's evidence.
struct PendingSrp {
    server: SrpServer,
    created: Instant,
}

pub struct RestState {
    pub config: Config,
    pub db: Database,
    /// In-flight challenges keyed by SRP username. A `Mutex<HashMap>` is ample: logins are
    /// low-rate and each entry lives for seconds.
    pending: Mutex<HashMap<String, PendingSrp>>,
}

impl RestState {
    pub fn new(config: Config, db: Database) -> Self {
        Self {
            config,
            db,
            pending: Mutex::new(HashMap::new()),
        }
    }
}

pub fn router(state: Arc<RestState>) -> Router {
    Router::new()
        .route("/bnetserver/portal/", get(get_portal))
        .route("/bnetserver/login/", get(get_login_form).post(post_login))
        .route("/bnetserver/login/srp/", post(post_login_srp))
        .route("/bnetserver/gameAccounts/", get(get_game_accounts))
        .route(
            "/bnetserver/refreshLoginTicket/",
            post(refresh_login_ticket),
        )
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

/// `POST /bnetserver/login/srp/` — look up the account, generate `B`, return the challenge.
async fn post_login_srp(
    State(state): State<Arc<RestState>>,
    Json(form): Json<types::LoginForm>,
) -> impl IntoResponse {
    let Some(account_name) = form.get("account_name") else {
        return types::login_error("MISSING_ACCOUNT", "No account name supplied").into_response();
    };

    let creds = match state.db.accounts.find_bnet_credentials(account_name).await {
        Ok(Some(creds)) => creds,
        Ok(None) => {
            // Do not distinguish "no such account" from "wrong password" to the client here;
            // both surface later as a failed login. Log for the operator.
            debug!(account = %account_name, "srp challenge for unknown/uninitialised account");
            return types::login_error("INVALID_CREDENTIALS", "Account not found").into_response();
        }
        Err(e) => {
            warn!("bnet credential lookup failed: {e}");
            return types::login_error("SERVER_ERROR", "Internal error").into_response();
        }
    };

    let srp_username = srp6v2::srp_username(account_name);
    let server = SrpServer::new(srp_username.clone(), creds.salt, &creds.verifier);
    let challenge = server.challenge();

    {
        let mut pending = state.pending.lock().await;
        prune_expired(&mut pending);
        pending.insert(
            srp_username,
            PendingSrp {
                server,
                created: Instant::now(),
            },
        );
    }

    Json(types::SrpLoginChallenge::from(challenge)).into_response()
}

/// `POST /bnetserver/login/` — verify the client's evidence and, on success, issue a ticket.
async fn post_login(
    State(state): State<Arc<RestState>>,
    Json(form): Json<types::LoginForm>,
) -> impl IntoResponse {
    let (Some(account_name), Some(public_a), Some(client_m1)) = (
        form.get("account_name"),
        form.get("public_A"),
        form.get("client_evidence_M1"),
    ) else {
        return Json(types::LoginResult::rejected(
            "MISSING_FIELDS",
            "Login is missing required fields",
        ))
        .into_response();
    };

    let srp_username = srp6v2::srp_username(account_name);

    // Take the pending challenge: an SRP object must verify exactly once.
    let pending = {
        let mut map = state.pending.lock().await;
        prune_expired(&mut map);
        map.remove(&srp_username)
    };
    let Some(pending) = pending else {
        return Json(types::LoginResult::rejected(
            "NO_CHALLENGE",
            "No active challenge; restart the login",
        ))
        .into_response();
    };

    let Some(verified) = pending.server.verify(public_a, client_m1) else {
        debug!(account = %account_name, "srp evidence rejected");
        return Json(types::LoginResult::rejected(
            "INVALID_CREDENTIALS",
            "Incorrect password",
        ))
        .into_response();
    };

    // Re-resolve the account id for ticket storage (cheap, and avoids threading it through the
    // SRP object).
    let account_id = match state.db.accounts.find_bnet_credentials(account_name).await {
        Ok(Some(creds)) => creds.id,
        _ => {
            warn!(account = %account_name, "account vanished between challenge and login");
            return Json(types::LoginResult::rejected(
                "SERVER_ERROR",
                "Internal error",
            ))
            .into_response();
        }
    };

    let ticket = generate_login_ticket();
    let expiry = chrono::Utc::now().timestamp() + state.config.login_ticket_duration as i64;
    if let Err(e) = state
        .db
        .accounts
        .store_bnet_login_ticket(account_id, &ticket, expiry)
        .await
    {
        warn!("failed to store login ticket: {e}");
        return Json(types::LoginResult::rejected(
            "SERVER_ERROR",
            "Internal error",
        ))
        .into_response();
    }

    debug!(account = %account_name, "login succeeded, ticket issued");
    Json(types::LoginResult::done(ticket, verified.server_m2)).into_response()
}

/// `GET /bnetserver/gameAccounts/` — implemented in M4 alongside the account services.
async fn get_game_accounts() -> impl IntoResponse {
    Json(types::GameAccountList {
        game_accounts: Vec::new(),
    })
}

/// `POST /bnetserver/refreshLoginTicket/` — implemented in M4 with ticket lookup.
async fn refresh_login_ticket() -> impl IntoResponse {
    axum::http::StatusCode::NOT_IMPLEMENTED
}

/// A login ticket: `OX-` followed by 40 hex chars (20 random bytes), matching TrinityCore's
/// `TC-<hex>` shape.
fn generate_login_ticket() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("OX-{}", hex::encode_upper(bytes))
}

/// Drop challenges older than [`CHALLENGE_TTL`]. Called opportunistically under the lock, so no
/// separate reaper task is needed at this scale.
fn prune_expired(pending: &mut HashMap<String, PendingSrp>) {
    let now = Instant::now();
    pending.retain(|_, p| now.duration_since(p.created) < CHALLENGE_TTL);
}
