//! BGS service constants and request handlers.
//!
//! Requests are dispatched by `(service_hash, method_id)` (CypherCore `Session.cs`). The hashes
//! are the FNV-1a-32 values of the service names, taken verbatim from CypherCore's `OriginalHash`
//! enum.
//!
//! ## Server-initiated frames
//!
//! Unlike a plain request/response service, authentication drives the client with *callbacks*:
//! the client's `Logon` prompts us to push a `ChallengeListener.OnExternalChallenge` (pointing its
//! browser at our web-auth URL), and a successful `VerifyWebCredentials` pushes an
//! `AuthenticationListener.OnLogonComplete`. Those callbacks are server-initiated requests
//! (`service_id = 0`, our own incrementing token), sent *in addition to* the ordinary OK response
//! for the method the client called. A handler therefore returns a list of frames, not one.

use anyhow::Result;
use prost::Message;
use tracing::{debug, warn};

use super::framing::{self, Frame};
use super::proto::{
    ChallengeExternalRequest, ConnectRequest, ConnectResponse, EntityId, Header, LogonRequest,
    LogonResult, ProcessId, VerifyWebCredentialsRequest,
};
use crate::database::Database;

// Service hashes (dispatch keys + callback targets), from CypherCore `OriginalHash`.
pub const CONNECTION_SERVICE: u32 = 0x6544_6991;
pub const AUTHENTICATION_SERVICE: u32 = 0x0DEC_FC01;
pub const AUTHENTICATION_LISTENER: u32 = 0x7124_0E35;
pub const CHALLENGE_LISTENER: u32 = 0xBBDA_171F;
pub const ACCOUNT_SERVICE: u32 = 0x62DA_0891;
pub const GAME_UTILITIES_SERVICE: u32 = 0x3FC1_274D;

// ConnectionService method ids.
const METHOD_CONNECT: u32 = 1;
const METHOD_KEEP_ALIVE: u32 = 5;
const METHOD_REQUEST_DISCONNECT: u32 = 7;

// AuthenticationService method ids.
const METHOD_LOGON: u32 = 1;
const METHOD_VERIFY_WEB_CREDENTIALS: u32 = 7;

// Callback method ids (server -> client).
const METHOD_ON_EXTERNAL_CHALLENGE: u32 = 3; // on ChallengeListener
const METHOD_ON_LOGON_COMPLETE: u32 = 5; // on AuthenticationListener

// A subset of BattlenetRpcErrorCode (src/server/proto/BattlenetRpcErrorCodes.h).
const ERROR_OK: u32 = 0x0000_0000;
const ERROR_DENIED: u32 = 0x0000_0003;
const ERROR_BAD_PROGRAM: u32 = 0x0000_004D;
const ERROR_LOGON_INVALID_AUTH_TOKEN: u32 = 0x0000_020A;
const ERROR_RPC_INVALID_SERVICE: u32 = 0x0000_0BC2;
const ERROR_RPC_INVALID_METHOD: u32 = 0x0000_0BC3;

// EntityId high bits, verbatim from CypherCore's LogonResult construction. The low half is the
// account id; the high half encodes the entity kind ("WoW" = 0x57_6F_57 shows up in the game
// account high bits).
const ENTITY_HIGH_ACCOUNT: u64 = 0x0100_0000_0000_0000;
const ENTITY_HIGH_GAME_ACCOUNT: u64 = 0x0200_0002_0057_6F57;

/// Length of the BGS session key handed to the client in `LogonResult`, and later validated by
/// the world server.
const SESSION_KEY_LEN: usize = 64;

/// What a handler decided to do with a request: a list of already-framed bytes to write (in
/// order), and whether to close the connection afterwards.
pub struct Outcome {
    pub frames: Vec<Vec<u8>>,
    pub disconnect: bool,
}

impl Outcome {
    fn send(frames: Vec<Vec<u8>>) -> Self {
        Self {
            frames,
            disconnect: false,
        }
    }
    fn disconnect(frames: Vec<Vec<u8>>) -> Self {
        Self {
            frames,
            disconnect: true,
        }
    }
}

/// Per-connection service state: shared database access plus the mutable bits of one client's
/// session (the server-request token counter, whether it has authenticated, and the session key
/// issued at logon for the world server to later validate).
pub struct Services {
    db: Database,
    /// The URL we point the client's embedded browser at during `Logon`.
    web_auth_url: String,
    /// Monotonic token for server-initiated requests (callbacks).
    request_token: u32,
    /// Set once `VerifyWebCredentials` succeeds.
    authed: bool,
    /// The BGS session key issued in `OnLogonComplete`, kept for the realm-join step (M5).
    session_key: Option<Vec<u8>>,
}

impl Services {
    pub fn new(db: Database, web_auth_url: String) -> Self {
        Self {
            db,
            web_auth_url,
            request_token: 0,
            authed: false,
            session_key: None,
        }
    }

    /// Whether the client has completed `VerifyWebCredentials`.
    pub fn is_authed(&self) -> bool {
        self.authed
    }

    fn next_token(&mut self) -> u32 {
        let t = self.request_token;
        self.request_token = self.request_token.wrapping_add(1);
        t
    }

    /// Route one request frame to its handler.
    pub async fn dispatch(&mut self, frame: &Frame) -> Result<Outcome> {
        let service_hash = frame.header.service_hash.unwrap_or(0);
        let method_id = frame.header.method_id.unwrap_or(0);
        let token = frame.header.token.unwrap_or(0);

        let result = match service_hash {
            CONNECTION_SERVICE => self.dispatch_connection(method_id, frame),
            AUTHENTICATION_SERVICE => self.dispatch_authentication(method_id, frame).await,
            other => {
                debug!(
                    service_hash = format!("0x{other:08X}"),
                    method_id, "unimplemented service"
                );
                Ok(Outcome::send(vec![reply(
                    &frame.header,
                    ERROR_RPC_INVALID_SERVICE,
                )?]))
            }
        };

        result.map_err(|e| {
            anyhow::anyhow!(
                "handling service 0x{service_hash:08X} method {method_id} (token {token}): {e}"
            )
        })
    }

    fn dispatch_connection(&mut self, method_id: u32, frame: &Frame) -> Result<Outcome> {
        match method_id {
            METHOD_CONNECT => handle_connect(frame),
            METHOD_KEEP_ALIVE => {
                debug!("keep-alive");
                Ok(Outcome::send(vec![reply(&frame.header, ERROR_OK)?]))
            }
            METHOD_REQUEST_DISCONNECT => {
                debug!("client requested disconnect");
                Ok(Outcome::disconnect(vec![reply(&frame.header, ERROR_OK)?]))
            }
            other => {
                debug!(method_id = other, "unimplemented connection method");
                Ok(Outcome::send(vec![reply(
                    &frame.header,
                    ERROR_RPC_INVALID_METHOD,
                )?]))
            }
        }
    }

    async fn dispatch_authentication(&mut self, method_id: u32, frame: &Frame) -> Result<Outcome> {
        match method_id {
            METHOD_LOGON => self.handle_logon(frame),
            METHOD_VERIFY_WEB_CREDENTIALS => self.handle_verify_web_credentials(frame).await,
            other => {
                debug!(method_id = other, "unimplemented authentication method");
                Ok(Outcome::send(vec![reply(
                    &frame.header,
                    ERROR_RPC_INVALID_METHOD,
                )?]))
            }
        }
    }

    /// AuthenticationService.Logon — the client opens authentication here. We validate the program
    /// and then push a `ChallengeListener.OnExternalChallenge` pointing its embedded browser at our
    /// web-auth URL; the ordinary OK response follows. The client does the REST SRP login there and
    /// comes back with `VerifyWebCredentials`.
    fn handle_logon(&mut self, frame: &Frame) -> Result<Outcome> {
        let request = LogonRequest::decode(frame.payload.as_slice())
            .map_err(|e| anyhow::anyhow!("bad LogonRequest: {e}"))?;

        let program = request.program.as_deref().unwrap_or_default();
        if program != "WoW" {
            warn!(%program, "rejecting logon for unexpected program");
            return Ok(Outcome::send(vec![reply(&frame.header, ERROR_BAD_PROGRAM)?]));
        }
        debug!(
            platform = request.platform.as_deref().unwrap_or_default(),
            locale = request.locale.as_deref().unwrap_or_default(),
            "logon: sending external web-auth challenge"
        );

        let challenge = ChallengeExternalRequest {
            request_token: None,
            payload_type: Some("web_auth_url".to_string()),
            payload: Some(self.web_auth_url.clone().into_bytes()),
        };
        let callback = self.server_request(
            CHALLENGE_LISTENER,
            METHOD_ON_EXTERNAL_CHALLENGE,
            &challenge.encode_to_vec(),
        )?;

        // Callback first (mirrors CypherCore, which sends the request inside the handler body and
        // the OK response only after the handler returns).
        Ok(Outcome::send(vec![callback, reply(&frame.header, ERROR_OK)?]))
    }

    /// AuthenticationService.VerifyWebCredentials — the client presents the login ticket it earned
    /// from the web-auth flow. We look it up, and on success push
    /// `AuthenticationListener.OnLogonComplete` carrying the account/game-account ids and a fresh
    /// session key.
    async fn handle_verify_web_credentials(&mut self, frame: &Frame) -> Result<Outcome> {
        let request = VerifyWebCredentialsRequest::decode(frame.payload.as_slice())
            .map_err(|e| anyhow::anyhow!("bad VerifyWebCredentialsRequest: {e}"))?;

        // The web credentials are the login ticket string, carried as bytes.
        let ticket = request
            .web_credentials
            .as_deref()
            .map(|b| String::from_utf8_lossy(b).trim().to_string())
            .unwrap_or_default();

        if ticket.is_empty() {
            debug!("verify web credentials: empty ticket");
            return Ok(Outcome::send(vec![reply(
                &frame.header,
                ERROR_LOGON_INVALID_AUTH_TOKEN,
            )?]));
        }

        let account = match self.db.accounts.find_by_login_ticket(&ticket).await {
            Ok(Some(account)) => account,
            Ok(None) => {
                debug!("verify web credentials: unknown or expired ticket");
                return Ok(Outcome::send(vec![reply(
                    &frame.header,
                    ERROR_LOGON_INVALID_AUTH_TOKEN,
                )?]));
            }
            Err(e) => {
                warn!("login ticket lookup failed: {e}");
                return Ok(Outcome::send(vec![reply(&frame.header, ERROR_DENIED)?]));
            }
        };

        let session_key = random_session_key();
        let result = LogonResult {
            error_code: Some(ERROR_OK),
            account_id: Some(EntityId {
                high: Some(ENTITY_HIGH_ACCOUNT),
                low: Some(account.id as u64),
            }),
            game_account_id: vec![EntityId {
                high: Some(ENTITY_HIGH_GAME_ACCOUNT),
                low: Some(account.id as u64),
            }],
            session_key: Some(session_key.clone()),
            connected_region: Some(1),
            ..Default::default()
        };

        let callback = self.server_request(
            AUTHENTICATION_LISTENER,
            METHOD_ON_LOGON_COMPLETE,
            &result.encode_to_vec(),
        )?;

        self.authed = true;
        self.session_key = Some(session_key);
        debug!(account = %account.username, "logon complete");

        Ok(Outcome::send(vec![callback, reply(&frame.header, ERROR_OK)?]))
    }

    /// Frame a server-initiated request (callback) with a fresh token.
    fn server_request(
        &mut self,
        service_hash: u32,
        method_id: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>> {
        let token = self.next_token();
        framing::encode(
            framing::request_header(service_hash, method_id, token),
            payload,
        )
    }
}

/// ConnectionService.Connect — the first call the client makes. Echoes its client id, reports a
/// server id/time, and mirrors the bindless-RPC preference.
fn handle_connect(frame: &Frame) -> Result<Outcome> {
    let request = ConnectRequest::decode(frame.payload.as_slice())
        .map_err(|e| anyhow::anyhow!("bad ConnectRequest: {e}"))?;

    let now = chrono::Utc::now();
    let response = ConnectResponse {
        server_id: Some(ProcessId {
            // A stable-ish server identity; the client only uses it for routing bookkeeping.
            label: Some(std::process::id()),
            epoch: Some(now.timestamp() as u32),
        }),
        client_id: request.client_id,
        bind_result: None,
        server_time: Some(now.timestamp_millis() as u64),
        use_bindless_rpc: request.use_bindless_rpc,
    };

    debug!(
        bindless = ?request.use_bindless_rpc,
        "connect: replying with server id and time"
    );
    Ok(Outcome::send(vec![reply_with(
        &frame.header,
        ERROR_OK,
        &response.encode_to_vec(),
    )?]))
}

/// Frame an OK/error response with an empty payload.
fn reply(request: &Header, status: u32) -> Result<Vec<u8>> {
    reply_with(request, status, &[])
}

/// Frame a response carrying `payload`.
fn reply_with(request: &Header, status: u32, payload: &[u8]) -> Result<Vec<u8>> {
    framing::encode(framing::response_header(request, status), payload)
}

/// 64 random bytes: the BGS session key.
fn random_session_key() -> Vec<u8> {
    use rand::RngCore;
    let mut key = vec![0u8; SESSION_KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Services` with a lazily-connected pool: fine for handlers that never touch the DB.
    fn services() -> Services {
        let db =
            Database::connect_lazy("mysql://user:pass@127.0.0.1/oxcore_auth").expect("lazy pool");
        Services::new(db, "https://localhost:8081/bnetserver/login/".to_string())
    }

    fn frame(service_hash: u32, method_id: u32, token: u32, payload: Vec<u8>) -> Frame {
        Frame {
            header: Header {
                service_id: Some(0),
                method_id: Some(method_id),
                token: Some(token),
                service_hash: Some(service_hash),
                size: Some(payload.len() as u32),
                ..Default::default()
            },
            payload,
        }
    }

    fn connect_frame(token: u32, bindless: bool) -> Frame {
        let payload = ConnectRequest {
            client_id: Some(ProcessId {
                label: Some(123),
                epoch: Some(456),
            }),
            bind_request: None,
            use_bindless_rpc: Some(bindless),
        }
        .encode_to_vec();
        frame(CONNECTION_SERVICE, METHOD_CONNECT, token, payload)
    }

    #[tokio::test]
    async fn connect_produces_a_well_formed_response() {
        let mut svc = services();
        let out = svc.dispatch(&connect_frame(7, true)).await.unwrap();
        assert!(!out.disconnect);
        assert_eq!(out.frames.len(), 1);

        let (resp_frame, _) = framing::decode(&out.frames[0]).unwrap().unwrap();
        assert_eq!(
            resp_frame.header.service_id,
            Some(framing::RESPONSE_SERVICE_ID)
        );
        assert_eq!(resp_frame.header.token, Some(7));
        assert_eq!(resp_frame.header.status, Some(ERROR_OK));

        let resp = ConnectResponse::decode(resp_frame.payload.as_slice()).unwrap();
        assert!(resp.server_id.is_some());
        assert!(resp.server_time.unwrap() > 0);
        assert_eq!(resp.use_bindless_rpc, Some(true));
        assert_eq!(resp.client_id.unwrap().label, Some(123));
    }

    #[tokio::test]
    async fn keep_alive_replies_ok_with_empty_payload() {
        let mut svc = services();
        let out = svc
            .dispatch(&frame(CONNECTION_SERVICE, METHOD_KEEP_ALIVE, 3, vec![]))
            .await
            .unwrap();
        let (resp, _) = framing::decode(&out.frames[0]).unwrap().unwrap();
        assert_eq!(resp.header.status, Some(ERROR_OK));
        assert!(resp.payload.is_empty());
    }

    #[tokio::test]
    async fn request_disconnect_asks_to_close() {
        let mut svc = services();
        let out = svc
            .dispatch(&frame(CONNECTION_SERVICE, METHOD_REQUEST_DISCONNECT, 1, vec![]))
            .await
            .unwrap();
        assert!(out.disconnect);
    }

    #[tokio::test]
    async fn unknown_service_replies_with_invalid_service() {
        let mut svc = services();
        let out = svc.dispatch(&frame(0xDEAD_BEEF, 1, 1, vec![])).await.unwrap();
        let (resp, _) = framing::decode(&out.frames[0]).unwrap().unwrap();
        assert_eq!(resp.header.status, Some(ERROR_RPC_INVALID_SERVICE));
    }

    #[tokio::test]
    async fn logon_with_wow_program_sends_external_challenge_then_ok() {
        let mut svc = services();
        let payload = LogonRequest {
            program: Some("WoW".to_string()),
            platform: Some("Wn64".to_string()),
            locale: Some("enUS".to_string()),
            ..Default::default()
        }
        .encode_to_vec();
        let out = svc
            .dispatch(&frame(AUTHENTICATION_SERVICE, METHOD_LOGON, 11, payload))
            .await
            .unwrap();

        // Two frames: the OnExternalChallenge callback, then the OK response.
        assert_eq!(out.frames.len(), 2);

        let (callback, _) = framing::decode(&out.frames[0]).unwrap().unwrap();
        assert_eq!(callback.header.service_id, Some(0));
        assert_eq!(callback.header.service_hash, Some(CHALLENGE_LISTENER));
        assert_eq!(callback.header.method_id, Some(METHOD_ON_EXTERNAL_CHALLENGE));
        let ext = ChallengeExternalRequest::decode(callback.payload.as_slice()).unwrap();
        assert_eq!(ext.payload_type.as_deref(), Some("web_auth_url"));
        assert_eq!(
            String::from_utf8(ext.payload.unwrap()).unwrap(),
            "https://localhost:8081/bnetserver/login/"
        );

        let (resp, _) = framing::decode(&out.frames[1]).unwrap().unwrap();
        assert_eq!(resp.header.service_id, Some(framing::RESPONSE_SERVICE_ID));
        assert_eq!(resp.header.token, Some(11));
        assert_eq!(resp.header.status, Some(ERROR_OK));
        assert!(!svc.is_authed());
    }

    #[tokio::test]
    async fn logon_with_wrong_program_is_rejected_without_a_callback() {
        let mut svc = services();
        let payload = LogonRequest {
            program: Some("D3".to_string()),
            ..Default::default()
        }
        .encode_to_vec();
        let out = svc
            .dispatch(&frame(AUTHENTICATION_SERVICE, METHOD_LOGON, 1, payload))
            .await
            .unwrap();
        assert_eq!(out.frames.len(), 1);
        let (resp, _) = framing::decode(&out.frames[0]).unwrap().unwrap();
        assert_eq!(resp.header.status, Some(ERROR_BAD_PROGRAM));
    }

    #[tokio::test]
    async fn server_request_uses_incrementing_tokens_and_service_id_zero() {
        let mut svc = services();
        let a = svc.server_request(CHALLENGE_LISTENER, 3, b"x").unwrap();
        let b = svc.server_request(AUTHENTICATION_LISTENER, 5, b"y").unwrap();
        let (fa, _) = framing::decode(&a).unwrap().unwrap();
        let (fb, _) = framing::decode(&b).unwrap().unwrap();
        assert_eq!(fa.header.service_id, Some(0));
        assert_eq!(fa.header.token, Some(0));
        assert_eq!(fb.header.token, Some(1));
    }
}
