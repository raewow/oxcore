//! End-to-end test of the REST SRP login, driving the axum router in-process (no TLS, no
//! socket) with the shared [`SrpClient`] as the peer.
//!
//! Requires the dev MySQL from docker-compose, so it is `#[ignore]` by default. Run with:
//!   cargo test -p oxcore-bnet --test login_flow -- --ignored --nocapture

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use oxcore_bnet::config::Config;
use oxcore_bnet::database::Database;
use oxcore_bnet::rest::{router, RestState};
use oxcore_shared::crypto::srp6v2::{Challenge, SrpClient};
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

const DB_URL: &str = "mysql://root:root@127.0.0.1:3306/auth";
const USER: &str = "LOGINFLOW";
const PASSWORD: &str = "correct horse";

fn test_config() -> Config {
    toml::from_str(&format!("login_database_url = \"{DB_URL}\"")).unwrap()
}

fn login_form(inputs: &[(&str, &str)]) -> Body {
    let body = json!({
        "platform_id": "Win",
        "program_id": "WoW",
        "version": "1.14.0",
        "inputs": inputs.iter().map(|(id, v)| json!({"input_id": id, "value": v})).collect::<Vec<_>>(),
    });
    Body::from(serde_json::to_vec(&body).unwrap())
}

async fn post(app: &axum::Router, uri: &str, body: Body) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(body)
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

fn challenge_from_json(v: &Value) -> Challenge {
    Challenge {
        version: v["version"].as_u64().unwrap() as u32,
        iterations: v["iterations"].as_u64().unwrap() as u32,
        modulus: v["modulus"].as_str().unwrap().to_string(),
        generator: v["generator"].as_str().unwrap().to_string(),
        hash_function: v["hash_function"].as_str().unwrap().to_string(),
        username: v["username"].as_str().unwrap().to_string(),
        salt: v["salt"].as_str().unwrap().to_string(),
        public_b: v["public_B"].as_str().unwrap().to_string(),
    }
}

#[tokio::test]
#[ignore = "needs the dev MySQL container"]
async fn full_rest_srp_login() {
    let db = Database::connect(DB_URL).await.expect("connect to auth db");

    // Fresh account with both verifiers.
    sqlx::query("DELETE FROM account WHERE username = ?")
        .bind(USER)
        .execute(db_pool(&db))
        .await
        .unwrap();
    let account_id = db.accounts.create_account(USER, PASSWORD).await.unwrap();

    let app = router(Arc::new(RestState::new(test_config(), db.clone())));

    // Step 1: challenge.
    let (status, challenge_json) =
        post(&app, "/bnetserver/login/srp/", login_form(&[("account_name", USER)])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(challenge_json["version"], 2);
    let challenge = challenge_from_json(&challenge_json);

    // Step 2: client proof.
    let proof = SrpClient::new().prove(PASSWORD, &challenge);
    let (status, result) = post(
        &app,
        "/bnetserver/login/",
        login_form(&[
            ("account_name", USER),
            ("public_A", &proof.public_a),
            ("client_evidence_M1", &proof.client_m1),
        ]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let ticket = result["login_ticket"].as_str().expect("a login ticket");
    assert_eq!(result["server_evidence_M2"].as_str().unwrap(), proof.expected_m2);
    assert!(ticket.starts_with("OX-") && ticket.len() == 43, "ticket shape: {ticket}");

    // Ticket persisted.
    let stored: (Option<String>, i64) = sqlx::query_as(
        "SELECT bnet_login_ticket, bnet_login_ticket_expiry FROM account WHERE id = ?",
    )
    .bind(account_id)
    .fetch_one(db_pool(&db))
    .await
    .unwrap();
    assert_eq!(stored.0.as_deref(), Some(ticket));
    assert!(stored.1 > chrono::Utc::now().timestamp());

    // Wrong password is rejected, and no new ticket is issued.
    let (_, challenge2) =
        post(&app, "/bnetserver/login/srp/", login_form(&[("account_name", USER)])).await;
    let bad = SrpClient::new().prove("wrong", &challenge_from_json(&challenge2));
    let (_, rejected) = post(
        &app,
        "/bnetserver/login/",
        login_form(&[
            ("account_name", USER),
            ("public_A", &bad.public_a),
            ("client_evidence_M1", &bad.client_m1),
        ]),
    )
    .await;
    assert!(rejected["login_ticket"].is_null());
    assert_eq!(rejected["authentication_state"], "LOGIN");

    // A login POST with no prior challenge is refused.
    let (_, no_challenge) = post(
        &app,
        "/bnetserver/login/",
        login_form(&[
            ("account_name", "NOBODY"),
            ("public_A", "01"),
            ("client_evidence_M1", "01"),
        ]),
    )
    .await;
    assert_eq!(no_challenge["error_code"], "NO_CHALLENGE");

    sqlx::query("DELETE FROM account WHERE username = ?")
        .bind(USER)
        .execute(db_pool(&db))
        .await
        .unwrap();
}

/// The `Database` holds a repository, not the pool directly; grab a pool for raw assertions via
/// a fresh short-lived query helper. Simplest is a second connection, but reusing the account
/// repository's pool keeps it to one. Exposed for the test via a tiny accessor.
fn db_pool(db: &Database) -> &sqlx::MySqlPool {
    db.accounts.pool()
}
