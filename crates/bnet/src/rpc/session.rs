//! Per-connection driver for the BGS RPC channel.
//!
//! Reads length-prefixed frames off the TLS stream, dispatches each to a service handler, and
//! writes any response back. One task per connection.

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, trace};

use super::framing::{self};
use super::services::{self, Action};

/// Cap on the buffered-but-undecoded bytes, so a malicious client cannot make us buffer without
/// bound while withholding the end of a frame.
const MAX_BUFFER: usize = 1024 * 1024;

/// Run the RPC loop over one connection until it closes or errors.
pub async fn run<S>(mut stream: S) -> Result<()>
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

            let action = services::dispatch(&frame)?;
            buf.drain(..consumed);

            match action {
                Action::Reply(bytes) => stream.write_all(&bytes).await?,
                Action::None => {}
                Action::Disconnect(maybe) => {
                    if let Some(bytes) = maybe {
                        stream.write_all(&bytes).await?;
                    }
                    stream.flush().await?;
                    debug!("closing connection at handler request");
                    return Ok(());
                }
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
        buf.extend_from_slice(&chunk[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::proto::{ConnectRequest, ConnectResponse, Header, ProcessId};
    use prost::Message;
    use tokio::io::duplex;

    /// Drive the session loop over an in-memory duplex, sending a Connect and reading the reply.
    #[tokio::test]
    async fn session_answers_a_connect_over_a_pipe() {
        let (mut client, server) = duplex(8192);

        let server_task = tokio::spawn(async move { run(server).await });

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
