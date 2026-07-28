//! The modern (1.14.x) world auth driver: runs the whole pre-gameplay handshake over one stream.
//!
//! Sequence (transcribed from HermesProxy's modern `WorldSocket`):
//!
//! 1. server sends `"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"`,
//! 2. client replies `"…CLIENT TO SERVER - V2\n"`,
//! 3. server sends `SMSG_AUTH_CHALLENGE` (plaintext-framed),
//! 4. client sends `CMSG_AUTH_SESSION`; the driver looks the account up by its realm-join ticket,
//!    finds the per-(build, OS) seed, and verifies the digest against the persisted session key,
//! 5. server sends `SMSG_ENTER_ENCRYPTED_MODE` (RSA-signed, plaintext),
//! 6. client sends `CMSG_ENTER_ENCRYPTED_MODE_ACK` (plaintext — encryption is **not** on yet),
//! 7. the driver switches [`WorldCrypt`] on and sends `SMSG_AUTH_RESPONSE` **encrypted**.
//!
//! It returns an [`AuthedConnection`] carrying the live cipher and the account, from which the
//! gameplay opcode loop takes over (a later milestone). This is I/O-generic (`AsyncRead + AsyncWrite`)
//! so it drives a real TCP socket or an in-memory duplex identically.
//!
//! **Unverified against a live client.**

use std::future::Future;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::debug;

use super::auth_seeds;
use super::crypt::WorldCrypt;
use super::framing::{
    self, ModernPacket, CONNECTION_INITIALIZE_CLIENT, CONNECTION_INITIALIZE_SERVER,
};
use super::handshake::{auth_response_frame, HandshakeServer};
use super::opcodes::{CMSG_AUTH_SESSION, CMSG_ENTER_ENCRYPTED_MODE_ACK, CMSG_PING, SMSG_PONG};
use super::packets::{pong_body, AuthResponseSuccess, AuthSession, EnterEncryptedModeSigner, Ping};

/// Resolves a realm-join ticket (a game-account name) to that account's 64-byte bnet session key.
/// The real implementation reads `account.sessionkey`; tests use an in-memory map.
///
/// The returned future is `Send` so the driver can run inside a spawned task.
pub trait SessionKeyProvider {
    /// Return the raw session-key bytes for `realm_join_ticket`, or `None` if unknown.
    fn session_key(
        &self,
        realm_join_ticket: &str,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;
}

/// The production [`SessionKeyProvider`]: reads `account.sessionkey` (hex) via the shared
/// account repository. The realm-join ticket is the account's game-account name, which is the
/// `username` the bnet realm-join stored the session key under.
pub struct AccountSessionKeys<'a> {
    pub accounts: &'a oxcore_shared::database::AccountRepository,
}

impl SessionKeyProvider for AccountSessionKeys<'_> {
    async fn session_key(&self, realm_join_ticket: &str) -> Result<Option<Vec<u8>>> {
        match self.accounts.find_session_key(realm_join_ticket).await? {
            Some((hex_key, _id)) => Ok(Some(
                hex::decode(hex_key.trim()).context("stored session key is not valid hex")?,
            )),
            None => Ok(None),
        }
    }
}

/// What the driver needs beyond the stream: the RSA signer, the account lookup, the client's
/// build/OS (established during the bnet flow), and the realm/expansion values for the response.
pub struct ModernAuthContext<'a, P: SessionKeyProvider> {
    pub signer: &'a dyn EnterEncryptedModeSigner,
    pub sessions: &'a P,
    pub build: u32,
    pub os: &'a str,
    pub virtual_realm_address: u32,
    pub active_expansion: u8,
    pub account_expansion: u8,
}

/// A successfully authenticated modern connection, handed to the gameplay layer.
pub struct AuthedConnection {
    /// The packet cipher, encryption enabled.
    pub crypt: WorldCrypt,
    /// The game-account name from the realm-join ticket.
    pub account: String,
    /// The 40-byte continued-session key, for later instance (`SMSG_CONNECT_TO`) handshakes.
    pub session_key40: [u8; 40],
}

/// Run the handshake to completion over `stream`.
pub async fn run_auth<S, P>(
    stream: &mut S,
    ctx: &ModernAuthContext<'_, P>,
) -> Result<AuthedConnection>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
    P: SessionKeyProvider,
{
    let mut buf: Vec<u8> = Vec::with_capacity(1024);

    // 1-2. Plaintext connection-init exchange.
    let mut greeting = CONNECTION_INITIALIZE_SERVER.as_bytes().to_vec();
    greeting.push(b'\n');
    stream.write_all(&greeting).await?;
    stream.flush().await?;
    read_client_initialize(stream).await?;

    // 3. Auth challenge.
    let handshake = HandshakeServer::new();
    stream.write_all(&handshake.auth_challenge_frame()).await?;
    stream.flush().await?;

    // 4. Auth session -> verify.
    let session_packet = read_plaintext(stream, &mut buf).await?;
    if session_packet.opcode != CMSG_AUTH_SESSION {
        bail!(
            "expected CMSG_AUTH_SESSION, got opcode 0x{:04X}",
            session_packet.opcode
        );
    }
    let session = AuthSession::parse(&session_packet.body)?;
    debug!(account = %session.realm_join_ticket, "modern auth session");

    let session_key = ctx
        .sessions
        .session_key(&session.realm_join_ticket)
        .await?
        .with_context(|| format!("no session key for '{}'", session.realm_join_ticket))?;
    let seed = auth_seeds::lookup(ctx.build, ctx.os)
        .with_context(|| format!("no auth seed for build {} os '{}'", ctx.build, ctx.os))?;

    let keys = handshake
        .verify_session(&session, &session_key, &seed)
        .context("auth digest verification failed")?;

    // 5. Enter encrypted mode.
    stream
        .write_all(&handshake.enter_encrypted_mode_frame(&keys, ctx.signer))
        .await?;
    stream.flush().await?;

    // 6. The ack is still plaintext (encryption turns on only afterwards).
    let ack = read_plaintext(stream, &mut buf).await?;
    if ack.opcode != CMSG_ENTER_ENCRYPTED_MODE_ACK {
        bail!(
            "expected CMSG_ENTER_ENCRYPTED_MODE_ACK, got opcode 0x{:04X}",
            ack.opcode
        );
    }

    // 7. Encryption is now live. The client has already consumed nonce counters for both
    // plaintext handshake frames in each direction.
    let mut crypt = WorldCrypt::server_after_handshake(&keys.aes_key);
    let info = AuthResponseSuccess {
        virtual_realm_address: ctx.virtual_realm_address,
        active_expansion: ctx.active_expansion,
        account_expansion: ctx.account_expansion,
        time: chrono::Utc::now().timestamp(),
        available_classes: Vec::new(),
    };
    stream
        .write_all(&auth_response_frame(&mut crypt, &info))
        .await?;
    stream.flush().await?;

    debug!(account = %session.realm_join_ticket, "modern auth complete");
    Ok(AuthedConnection {
        crypt,
        account: session.realm_join_ticket,
        session_key40: keys.session_key,
    })
}

/// Run the auth handshake and then the post-auth (encrypted) packet loop over one stream.
pub async fn serve_connection<S, P>(mut stream: S, ctx: &ModernAuthContext<'_, P>) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
    P: SessionKeyProvider,
{
    let conn = run_auth(&mut stream, ctx).await?;
    run_connection(stream, conn).await
}

/// The post-auth loop: read **encrypted** frames off the stream and dispatch them. Only the
/// keep-alive `CMSG_PING` is answered so far (with `SMSG_PONG`); the gameplay opcodes — starting
/// with character enumeration — are the next milestone. Unknown opcodes are logged and skipped so
/// one unhandled packet does not drop the connection.
pub async fn run_connection<S>(mut stream: S, mut conn: AuthedConnection) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];

    loop {
        // Drain every complete, decryptable frame currently buffered.
        while let Some((packet, consumed)) = framing::decode(&mut conn.crypt, &buf)? {
            buf.drain(..consumed);
            match packet.opcode {
                CMSG_PING => {
                    let ping = Ping::parse(&packet.body)?;
                    let pong = framing::encode(&mut conn.crypt, SMSG_PONG, &pong_body(ping.serial));
                    stream.write_all(&pong).await?;
                }
                other => {
                    debug!(
                        account = %conn.account,
                        opcode = format!("0x{other:04X}"),
                        "unhandled modern opcode"
                    );
                }
            }
        }
        stream.flush().await?;

        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            debug!(account = %conn.account, "modern connection closed");
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// Read and validate the client's connection-init greeting (`<string>\n`).
async fn read_client_initialize<S>(stream: &mut S) -> Result<()>
where
    S: AsyncReadExt + Unpin,
{
    let expected = CONNECTION_INITIALIZE_CLIENT.as_bytes();
    let mut line = vec![0u8; expected.len() + 1];
    stream
        .read_exact(&mut line)
        .await
        .context("failed to read client connection-init greeting")?;

    if &line[..expected.len()] != expected {
        bail!("client sent an unexpected connection-init string");
    }
    if line[expected.len()] != b'\n' {
        bail!("client connection-init greeting was not newline-terminated");
    }
    Ok(())
}

/// Read exactly one plaintext (pre-encryption) frame, buffering leftover bytes in `buf` for the
/// next call.
async fn read_plaintext<S>(stream: &mut S, buf: &mut Vec<u8>) -> Result<ModernPacket>
where
    S: AsyncReadExt + Unpin,
{
    let mut chunk = [0u8; 512];
    loop {
        if let Some((packet, consumed)) = framing::decode_plaintext(buf)? {
            buf.drain(..consumed);
            return Ok(packet);
        }
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            bail!("connection closed mid-handshake");
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network::modern::auth_crypto::{derive_keys, expected_digest};
    use crate::core::network::modern::framing::{decode, encode_plaintext};
    use crate::core::network::modern::opcodes::{
        SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, SMSG_ENTER_ENCRYPTED_MODE,
    };
    use crate::core::network::modern::packets::AuthSession;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const BUILD: u32 = 41794; // 1.14.1 Windows x64 — a build with a known seed.
    const OS: &str = auth_seeds::os::WINDOWS_X64;
    const TICKET: &str = "PLAYER#1";

    fn session_key() -> Vec<u8> {
        (0..64u8).collect()
    }

    struct MapProvider(HashMap<String, Vec<u8>>);
    impl SessionKeyProvider for MapProvider {
        async fn session_key(&self, ticket: &str) -> Result<Option<Vec<u8>>> {
            Ok(self.0.get(ticket).cloned())
        }
    }

    struct StubSigner;
    impl EnterEncryptedModeSigner for StubSigner {
        fn sign(&self, _hash: &[u8; 32]) -> Vec<u8> {
            vec![0u8; 256]
        }
    }

    /// Read exactly one plaintext frame from an async stream (client-side test helper).
    async fn read_one<S: AsyncReadExt + Unpin>(stream: &mut S, buf: &mut Vec<u8>) -> ModernPacket {
        let mut chunk = [0u8; 512];
        loop {
            if let Some((p, consumed)) = framing::decode_plaintext(buf).unwrap() {
                buf.drain(..consumed);
                return p;
            }
            let n = stream.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed early");
            buf.extend_from_slice(&chunk[..n]);
        }
    }

    #[tokio::test]
    async fn a_correct_client_completes_the_handshake_and_gets_an_encrypted_auth_response() {
        let (mut client, mut server) = tokio::io::duplex(8192);

        let provider = MapProvider(HashMap::from([(TICKET.to_string(), session_key())]));
        let server_task = tokio::spawn(async move {
            let ctx = ModernAuthContext {
                signer: &StubSigner,
                sessions: &provider,
                build: BUILD,
                os: OS,
                virtual_realm_address: 0x0101_0001,
                active_expansion: 0,
                account_expansion: 0,
            };
            run_auth(&mut server, &ctx).await
        });

        // --- client side ---
        let seed = auth_seeds::lookup(BUILD, OS).unwrap();
        let local_challenge = [0x42u8; 16];
        let mut buf = Vec::new();

        // 1. read server greeting.
        let mut greeting = vec![0u8; CONNECTION_INITIALIZE_SERVER.len() + 1];
        client.read_exact(&mut greeting).await.unwrap();
        assert_eq!(
            &greeting[..greeting.len() - 1],
            CONNECTION_INITIALIZE_SERVER.as_bytes()
        );
        assert_eq!(*greeting.last().unwrap(), b'\n');

        // 2. send client greeting.
        let mut reply = CONNECTION_INITIALIZE_CLIENT.as_bytes().to_vec();
        reply.push(b'\n');
        client.write_all(&reply).await.unwrap();

        // 3. read the auth challenge, pull the 16-byte server challenge out of it.
        let challenge = read_one(&mut client, &mut buf).await;
        assert_eq!(challenge.opcode, SMSG_AUTH_CHALLENGE);
        let mut server_challenge = [0u8; 16];
        server_challenge.copy_from_slice(&challenge.body[32..48]); // DosChallenge[32] then Challenge[16]

        // 4. answer with a correct CMSG_AUTH_SESSION.
        let full = expected_digest(&session_key(), &seed, &local_challenge, &server_challenge);
        let mut digest = [0u8; 24];
        digest.copy_from_slice(&full[..24]);
        let session_body = AuthSession {
            dos_response: 0,
            region_id: 1,
            battlegroup_id: 1,
            realm_id: 1,
            local_challenge,
            digest,
            use_ipv6: false,
            realm_join_ticket: TICKET.to_string(),
        }
        .encode()
        .unwrap();
        client
            .write_all(&encode_plaintext(CMSG_AUTH_SESSION, &session_body))
            .await
            .unwrap();

        // 5. read enter-encrypted-mode.
        let eem = read_one(&mut client, &mut buf).await;
        assert_eq!(eem.opcode, SMSG_ENTER_ENCRYPTED_MODE);

        // 6. ack it (plaintext, empty body).
        client
            .write_all(&encode_plaintext(CMSG_ENTER_ENCRYPTED_MODE_ACK, &[]))
            .await
            .unwrap();

        // 7. the auth response is encrypted — decode it with a client crypt keyed by the same
        // derived AES key.
        let keys = derive_keys(&session_key(), &server_challenge, &local_challenge);
        let mut client_crypt = WorldCrypt::client_after_handshake(&keys.aes_key);
        let mut enc_buf = Vec::new();
        let mut chunk = [0u8; 512];
        let response = loop {
            if let Some((p, _)) = decode(&mut client_crypt, &enc_buf).unwrap() {
                break p;
            }
            let n = client.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed before the auth response");
            enc_buf.extend_from_slice(&chunk[..n]);
        };
        assert_eq!(response.opcode, SMSG_AUTH_RESPONSE);
        assert_eq!(&response.body[..4], &[0, 0, 0, 0]); // Result = Ok

        let outcome = server_task.await.unwrap().unwrap();
        assert_eq!(outcome.account, TICKET);
    }

    #[tokio::test]
    async fn the_encrypted_loop_answers_a_ping_with_a_pong() {
        use crate::core::network::modern::framing::encode;
        use crate::core::network::modern::opcodes::{CMSG_PING, SMSG_PONG};

        let key = [0x33u8; 16];
        let (mut client, server) = tokio::io::duplex(4096);

        let conn = AuthedConnection {
            crypt: WorldCrypt::server(&key),
            account: "PLAYER#1".to_string(),
            session_key40: [0u8; 40],
        };
        let task = tokio::spawn(run_connection(server, conn));

        let mut client_crypt = WorldCrypt::client(&key);
        let mut ping_body = 7u32.to_le_bytes().to_vec();
        ping_body.extend_from_slice(&123u32.to_le_bytes()); // latency
        let frame = encode(&mut client_crypt, CMSG_PING, &ping_body);
        client.write_all(&frame).await.unwrap();

        // Read the encrypted pong back.
        let mut buf = Vec::new();
        let mut chunk = [0u8; 256];
        let pong = loop {
            if let Some((p, _)) = framing::decode(&mut client_crypt, &buf).unwrap() {
                break p;
            }
            let n = client.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed without a pong");
            buf.extend_from_slice(&chunk[..n]);
        };
        assert_eq!(pong.opcode, SMSG_PONG);
        assert_eq!(&pong.body, &7u32.to_le_bytes()); // echoed serial

        drop(client);
        let _ = task.await;
    }

    #[tokio::test]
    async fn a_wrong_session_key_aborts_the_handshake() {
        let (mut client, mut server) = tokio::io::duplex(8192);

        // Provider hands back a key that does not match what the client used.
        let provider = MapProvider(HashMap::from([(TICKET.to_string(), vec![0xFFu8; 64])]));
        let server_task = tokio::spawn(async move {
            let ctx = ModernAuthContext {
                signer: &StubSigner,
                sessions: &provider,
                build: BUILD,
                os: OS,
                virtual_realm_address: 1,
                active_expansion: 0,
                account_expansion: 0,
            };
            run_auth(&mut server, &ctx).await
        });

        let seed = auth_seeds::lookup(BUILD, OS).unwrap();
        let mut buf = Vec::new();

        let mut greeting = vec![0u8; CONNECTION_INITIALIZE_SERVER.len() + 1];
        client.read_exact(&mut greeting).await.unwrap();
        let mut reply = CONNECTION_INITIALIZE_CLIENT.as_bytes().to_vec();
        reply.push(b'\n');
        client.write_all(&reply).await.unwrap();

        let challenge = read_one(&mut client, &mut buf).await;
        let mut server_challenge = [0u8; 16];
        server_challenge.copy_from_slice(&challenge.body[32..48]);

        // Digest computed with the *real* key (which the server does not have).
        let full = expected_digest(&session_key(), &seed, &[0x42; 16], &server_challenge);
        let mut digest = [0u8; 24];
        digest.copy_from_slice(&full[..24]);
        let body = AuthSession {
            dos_response: 0,
            region_id: 1,
            battlegroup_id: 1,
            realm_id: 1,
            local_challenge: [0x42; 16],
            digest,
            use_ipv6: false,
            realm_join_ticket: TICKET.to_string(),
        }
        .encode()
        .unwrap();
        client
            .write_all(&encode_plaintext(CMSG_AUTH_SESSION, &body))
            .await
            .unwrap();

        // The server must reject rather than proceed to encrypted mode.
        assert!(server_task.await.unwrap().is_err());
    }
}
