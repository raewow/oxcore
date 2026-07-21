//! BGS service constants and request handlers.
//!
//! Requests are dispatched by `(service_hash, method_id)` (CypherCore `Session.cs`). The hashes
//! are the FNV-1a-32 values of the service names, taken verbatim from CypherCore's
//! `ServiceHash` enum.

use anyhow::Result;
use prost::Message;
use tracing::debug;

use super::framing::{self, Frame};
use super::proto::{ConnectRequest, ConnectResponse, Header, ProcessId};

// Service hashes (dispatch keys).
pub const CONNECTION_SERVICE: u32 = 0x6544_6991;
pub const AUTHENTICATION_SERVICE: u32 = 0x0DEC_FC01;
pub const AUTHENTICATION_LISTENER: u32 = 0x7124_0E35;
pub const ACCOUNT_SERVICE: u32 = 0x62DA_0891;
pub const GAME_UTILITIES_SERVICE: u32 = 0x3FC1_274D;

// ConnectionService method ids.
const METHOD_CONNECT: u32 = 1;
const METHOD_KEEP_ALIVE: u32 = 5;
const METHOD_REQUEST_DISCONNECT: u32 = 7;

// A subset of BattlenetRpcErrorCode.
const ERROR_OK: u32 = 0;
const ERROR_RPC_NOT_IMPLEMENTED: u32 = 0x0000_000B; // ERROR_RPC_METHOD_NOT_FOUND-ish placeholder

/// What a handler decided to do with a request.
pub enum Action {
    /// Send this already-framed response back to the client.
    Reply(Vec<u8>),
    /// Nothing to send (e.g. a fire-and-forget notification was handled).
    None,
    /// Close the connection after optionally sending the given frame.
    Disconnect(Option<Vec<u8>>),
}

/// Route one request frame to its handler.
pub fn dispatch(frame: &Frame) -> Result<Action> {
    let service_hash = frame.header.service_hash.unwrap_or(0);
    let method_id = frame.header.method_id.unwrap_or(0);
    let token = frame.header.token.unwrap_or(0);

    match service_hash {
        CONNECTION_SERVICE => dispatch_connection(method_id, frame),
        other => {
            debug!(
                service_hash = format!("0x{other:08X}"),
                method_id, "unimplemented service"
            );
            // Reply with an error status so the client fails the call rather than hanging.
            Ok(Action::Reply(framing::encode(
                framing::response_header(&frame.header, ERROR_RPC_NOT_IMPLEMENTED),
                &[],
            )?))
        }
    }
    .map_err(|e| {
        anyhow::anyhow!("handling service 0x{service_hash:08X} method {method_id} (token {token}): {e}")
    })
}

fn dispatch_connection(method_id: u32, frame: &Frame) -> Result<Action> {
    match method_id {
        METHOD_CONNECT => handle_connect(frame),
        METHOD_KEEP_ALIVE => {
            debug!("keep-alive");
            Ok(Action::Reply(ok_response(&frame.header, &[])?))
        }
        METHOD_REQUEST_DISCONNECT => {
            debug!("client requested disconnect");
            Ok(Action::Disconnect(Some(ok_response(&frame.header, &[])?)))
        }
        other => {
            debug!(method_id = other, "unimplemented connection method");
            Ok(Action::Reply(framing::encode(
                framing::response_header(&frame.header, ERROR_RPC_NOT_IMPLEMENTED),
                &[],
            )?))
        }
    }
}

/// ConnectionService.Connect — the first call the client makes. Echoes its client id, reports a
/// server id/time, and mirrors the bindless-RPC preference.
fn handle_connect(frame: &Frame) -> Result<Action> {
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
    Ok(Action::Reply(ok_response(&frame.header, &response.encode_to_vec())?))
}

/// Frame an OK response carrying `payload`.
fn ok_response(request: &Header, payload: &[u8]) -> Result<Vec<u8>> {
    framing::encode(framing::response_header(request, ERROR_OK), payload)
}

#[cfg(test)]
mod tests {
    use super::*;

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

        Frame {
            header: Header {
                service_id: Some(0),
                method_id: Some(METHOD_CONNECT),
                token: Some(token),
                service_hash: Some(CONNECTION_SERVICE),
                size: Some(payload.len() as u32),
                ..Default::default()
            },
            payload,
        }
    }

    #[test]
    fn connect_produces_a_well_formed_response() {
        let action = dispatch(&connect_frame(7, true)).unwrap();
        let bytes = match action {
            Action::Reply(b) => b,
            _ => panic!("expected a reply"),
        };

        let (resp_frame, _) = framing::decode(&bytes).unwrap().unwrap();
        // Response framing: service_id 0xFE, token echoed, status OK.
        assert_eq!(resp_frame.header.service_id, Some(framing::RESPONSE_SERVICE_ID));
        assert_eq!(resp_frame.header.token, Some(7));
        assert_eq!(resp_frame.header.status, Some(ERROR_OK));

        let resp = ConnectResponse::decode(resp_frame.payload.as_slice()).unwrap();
        assert!(resp.server_id.is_some());
        assert!(resp.server_time.unwrap() > 0);
        assert_eq!(resp.use_bindless_rpc, Some(true));
        // Client id is mirrored back.
        assert_eq!(resp.client_id.unwrap().label, Some(123));
    }

    #[test]
    fn keep_alive_replies_ok_with_empty_payload() {
        let frame = Frame {
            header: Header {
                method_id: Some(METHOD_KEEP_ALIVE),
                token: Some(3),
                service_hash: Some(CONNECTION_SERVICE),
                ..Default::default()
            },
            payload: vec![],
        };
        let bytes = match dispatch(&frame).unwrap() {
            Action::Reply(b) => b,
            _ => panic!("expected a reply"),
        };
        let (resp, _) = framing::decode(&bytes).unwrap().unwrap();
        assert_eq!(resp.header.status, Some(ERROR_OK));
        assert!(resp.payload.is_empty());
    }

    #[test]
    fn request_disconnect_asks_to_close() {
        let frame = Frame {
            header: Header {
                method_id: Some(METHOD_REQUEST_DISCONNECT),
                token: Some(1),
                service_hash: Some(CONNECTION_SERVICE),
                ..Default::default()
            },
            payload: vec![],
        };
        assert!(matches!(dispatch(&frame).unwrap(), Action::Disconnect(_)));
    }

    #[test]
    fn unknown_service_replies_with_an_error_status() {
        let frame = Frame {
            header: Header {
                method_id: Some(1),
                token: Some(1),
                service_hash: Some(0xDEAD_BEEF),
                ..Default::default()
            },
            payload: vec![],
        };
        let bytes = match dispatch(&frame).unwrap() {
            Action::Reply(b) => b,
            _ => panic!("expected a reply"),
        };
        let (resp, _) = framing::decode(&bytes).unwrap().unwrap();
        assert_ne!(resp.header.status, Some(ERROR_OK));
    }
}
