//! Per-connection driver for the BGS RPC channel.
//!
//! Reads length-prefixed frames off the TLS stream, dispatches each to a service handler, and
//! writes any response back. One task per connection.

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, trace};

use super::framing;
use super::services::Services;

/// Cap on the buffered-but-undecoded bytes, so a malicious client cannot make us buffer without
/// bound while withholding the end of a frame.
const MAX_BUFFER: usize = 1024 * 1024;

/// Run the RPC loop over one connection until it closes or errors. `services` carries this
/// connection's per-session state (auth status, callback tokens) and the shared database handle.
pub async fn run<S>(mut services: Services, mut stream: S) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];

    loop {
        // Drain every whole frame currently buffered before reading more.
        loop {
            let Some((frame, consumed)) = framing::decode(&buf)? else {
                break;
            };
            trace!(
                service_hash = frame.header.service_hash.map(|h| format!("0x{h:08X}")),
                method_id = frame.header.method_id,
                "request"
            );
            debug!(
                service_id = frame.header.service_id,
                service_hash = frame
                    .header
                    .service_hash
                    .map(|hash| format!("0x{hash:08X}")),
                method_id = frame.header.method_id,
                token = frame.header.token,
                payload_len = frame.payload.len(),
                "BGS request decoded"
            );

            let outcome = services.dispatch(&frame).await?;
            buf.drain(..consumed);

            for bytes in &outcome.frames {
                if let Some((response, _)) = framing::decode(bytes)? {
                    debug!(
                        service_id = response.header.service_id,
                        service_hash = response.header.service_hash.map(|hash| format!("0x{hash:08X}")),
                        method_id = response.header.method_id,
                        token = response.header.token,
                        status = response.header.status,
                        ciid = response.header.ciid.as_deref(),
                        payload = %hex::encode(&response.payload),
                        "bgs tx"
                    );
                }
                stream.write_all(bytes).await?;
            }
            if outcome.disconnect {
                stream.flush().await?;
                debug!("closing connection at handler request");
                return Ok(());
            }
        }
        stream.flush().await?;

        if buf.len() > MAX_BUFFER {
            anyhow::bail!("frame exceeded {MAX_BUFFER} bytes without completing");
        }

        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            debug!("client closed the connection");
            return Ok(());
        }
        // Temporary live-client diagnostic: log the first bytes of every read so we can see the
        // exact BGS frame the client sends, even if our framing fails to decode it.
        debug!(
            n,
            rx = %hex::encode(&chunk[..n.min(64)]),
            "bgs rx"
        );
        buf.extend_from_slice(&chunk[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::rpc::proto::{ConnectRequest, ConnectResponse, Header, ProcessId};
    use crate::rpc::services;
    use prost::Message;
    use tokio::io::duplex;

    /// Drive the session loop over an in-memory duplex, sending a Connect and reading the reply.
    #[tokio::test]
    async fn session_answers_a_connect_over_a_pipe() {
        let (mut client, server) = duplex(8192);

        // Connect never touches the database, so a lazy pool is enough.
        let db = Database::connect_lazy("mysql://user:pass@127.0.0.1/oxcore_auth").unwrap();
        let svc = Services::new(db, "https://localhost:8081/bnetserver/login/".to_string());
        let server_task = tokio::spawn(async move { run(svc, server).await });

        // Send a Connect request frame.
        let payload = ConnectRequest {
            client_id: Some(ProcessId {
                label: Some(1),
                epoch: Some(2),
            }),
            bind_request: None,
            use_bindless_rpc: Some(true),
        }
        .encode_to_vec();
        let header = Header {
            service_id: Some(0),
            method_id: Some(1),
            token: Some(99),
            service_hash: Some(services::CONNECTION_SERVICE),
            ..Default::default()
        };
        let request = framing::encode(header, &payload).unwrap();
        client.write_all(&request).await.unwrap();

        // Read the response frame back.
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        let resp_frame = loop {
            let n = client.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed without responding");
            buf.extend_from_slice(&chunk[..n]);
            if let Some((frame, _)) = framing::decode(&buf).unwrap() {
                break frame;
            }
        };

        assert_eq!(resp_frame.header.token, Some(99));
        assert_eq!(
            resp_frame.header.service_id,
            Some(framing::RESPONSE_SERVICE_ID)
        );
        let resp = ConnectResponse::decode(resp_frame.payload.as_slice()).unwrap();
        assert_eq!(resp.use_bindless_rpc, Some(true));

        // Closing the client ends the loop cleanly.
        drop(client);
        let _ = server_task.await;
    }
}
