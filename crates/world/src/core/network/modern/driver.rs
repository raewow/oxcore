//! The modern (1.14.x) world auth driver: runs the whole pre-gameplay handshake over one stream.
//!
//! Sequence:
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
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, warn};

use crate::core::network::socket::{cleanup_disconnected_session, route_packet};
use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::protocol::{Opcode, Protocol, WorldPacket};

use super::auth_crypto::{derive_continued_session_aes_key, verify_continued_session, DerivedKeys};
use super::auth_seeds;
use super::connect_to::{
    connect_to_body, ConnectKeyStore, ConnectToKey, PendingInstance, CONNECTION_TYPE_INSTANCE,
    CONNECT_TO_SERIAL_WORLD_ATTEMPT_1,
};
use super::crypt::WorldCrypt;
use super::framing::{
    self, ModernPacket, CONNECTION_INITIALIZE_CLIENT, CONNECTION_INITIALIZE_SERVER,
};
use super::handshake::{auth_response_frame, HandshakeServer};
use super::packet_log;
use super::opcodes::{
    CMSG_AUTH_CONTINUED_SESSION, CMSG_AUTH_SESSION, CMSG_ENTER_ENCRYPTED_MODE_ACK, CMSG_PING,
    SMSG_PONG,
};
use super::packets::{
    classic_available_classes, pong_body, AuthContinuedSession, AuthResponseSuccess, AuthSession,
    EnterEncryptedModeSigner, Ping,
};

/// Resolves a realm-join ticket (a game-account name) to that account's 64-byte bnet session key.
/// The real implementation reads `account.sessionkey`; tests use an in-memory map.
///
/// The returned future is `Send` so the driver can run inside a spawned task.
pub trait SessionKeyProvider {
    /// Return the account ID and raw session-key bytes for `realm_join_ticket`, or `None` if unknown.
    fn session_key(
        &self,
        realm_join_ticket: &str,
    ) -> impl Future<Output = Result<Option<(u32, Vec<u8>)>>> + Send;
}

/// The production [`SessionKeyProvider`]: reads `account.sessionkey` (hex) via the shared
/// account repository. The realm-join ticket is the account's game-account name, which is the
/// `username` the bnet realm-join stored the session key under.
pub struct AccountSessionKeys<'a> {
    pub accounts: &'a oxcore_shared::database::AccountRepository,
}

impl SessionKeyProvider for AccountSessionKeys<'_> {
    async fn session_key(&self, realm_join_ticket: &str) -> Result<Option<(u32, Vec<u8>)>> {
        match self.accounts.find_session_key(realm_join_ticket).await? {
            Some((hex_key, id)) => Ok(Some((
                id,
                hex::decode(hex_key.trim()).context("stored session key is not valid hex")?,
            ))),
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
    /// Persistent account ID used by the shared world session manager.
    pub account_id: u32,
    /// The 40-byte continued-session key, for later instance (`SMSG_CONNECT_TO`) handshakes.
    pub session_key40: [u8; 40],
}

/// How long to wait for the client's encrypted-mode ack before giving up. A client that dislikes
/// the signature stalls silently instead of disconnecting, and an un-capped read leaves the
/// connection task parked with no log line explaining why.
const ACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

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
    let mut plaintext_received = 0u64;
    let session_packet = read_plaintext(stream, &mut buf).await?;
    plaintext_received += 1;
    if session_packet.opcode != CMSG_AUTH_SESSION {
        bail!(
            "expected CMSG_AUTH_SESSION, got opcode 0x{:04X}",
            session_packet.opcode
        );
    }
    let session = AuthSession::parse(&session_packet.body)?;
    debug!(account = %session.realm_join_ticket, "modern auth session");

    let (account_id, session_key) = ctx
        .sessions
        .session_key(&session.realm_join_ticket)
        .await?
        .with_context(|| format!("no session key for '{}'", session.realm_join_ticket))?;
    // The build's own seed first, then the static fallback a patched client may have been made to
    // use instead — the packet does not say which, so both are tried.
    let seeds = auth_seeds::candidates(ctx.build, ctx.os);
    let seed_refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();

    let keys = handshake
        .verify_session_any_seed(&session, &session_key, &seed_refs)
        .context("auth digest verification failed")?;
    debug!("modern handshake: digest verified");

    // 5. Enter encrypted mode. Signing happens here, so a failure in the RSA signer surfaces
    // between these two log lines rather than as an unexplained stall.
    let eem = handshake.enter_encrypted_mode_frame(&keys, ctx.signer);
    debug!(
        bytes = eem.len(),
        "modern handshake: signed enter-encrypted-mode"
    );
    stream.write_all(&eem).await?;
    stream.flush().await?;
    debug!("modern handshake: sent enter-encrypted-mode, awaiting ack");

    // 6. The ack is still plaintext (encryption turns on only afterwards). A client that rejects
    // the signature simply stops talking without closing, so cap the wait rather than parking the
    // connection task forever with nothing in the log.
    tokio::time::timeout(
        ACK_TIMEOUT,
        read_plaintext_expecting(
            stream,
            &mut buf,
            CMSG_ENTER_ENCRYPTED_MODE_ACK,
            "CMSG_ENTER_ENCRYPTED_MODE_ACK",
            &mut plaintext_received,
        ),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "timed out after {:?} waiting for CMSG_ENTER_ENCRYPTED_MODE_ACK — the client \
             usually goes silent here when it rejects the enter-encrypted-mode signature",
            ACK_TIMEOUT
        )
    })??;
    debug!("modern handshake: ack received");

    // 7. Encryption is now live. The client has already consumed nonce counters for both
    // plaintext handshake frames in each direction.
    let mut crypt = WorldCrypt::server_after_handshake(&keys.aes_key, plaintext_received);
    let info = AuthResponseSuccess {
        virtual_realm_address: ctx.virtual_realm_address,
        virtual_realm_name: "Oxcore".into(),
        active_expansion: ctx.active_expansion,
        account_expansion: ctx.account_expansion,
        time: chrono::Utc::now().timestamp(),
        available_classes: classic_available_classes(),
    };
    stream
        .write_all(&auth_response_frame(&mut crypt, &info))
        .await?;
    stream.flush().await?;

    debug!(account = %session.realm_join_ticket, "modern auth complete");
    Ok(AuthedConnection {
        crypt,
        account: session.realm_join_ticket,
        account_id,
        session_key40: keys.session_key,
    })
}

/// Run the auth handshake and then the post-auth (encrypted) packet loop over one stream.
pub async fn serve_connection<S, P>(
    mut stream: S,
    ctx: &ModernAuthContext<'_, P>,
    world: Arc<World>,
    role: ConnectionRole,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
    P: SessionKeyProvider,
{
    let conn = run_auth(&mut stream, ctx).await?;
    run_world_connection(stream, conn, world, role).await
}

/// Run the instance-connection handshake.
///
/// Same shape as [`run_auth`] but the client presents `CMSG_AUTH_CONTINUED_SESSION` — a key it was
/// handed in `SMSG_CONNECT_TO` — instead of a fresh `CMSG_AUTH_SESSION`, and the handshake ends
/// with `SMSG_RESUME_COMMS` rather than `SMSG_AUTH_RESPONSE`. The account is identified by the key
/// alone; there is no second digest verification.
pub async fn run_instance_auth<S>(
    stream: &mut S,
    keys_store: &ConnectKeyStore,
    signer: &dyn EnterEncryptedModeSigner,
) -> Result<(AuthedConnection, PendingInstance)>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buf: Vec<u8> = Vec::with_capacity(1024);

    let mut greeting = CONNECTION_INITIALIZE_SERVER.as_bytes().to_vec();
    greeting.push(b'\n');
    stream.write_all(&greeting).await?;
    stream.flush().await?;
    read_client_initialize(stream).await?;

    let handshake = HandshakeServer::new();
    stream.write_all(&handshake.auth_challenge_frame()).await?;
    stream.flush().await?;

    let mut plaintext_received = 0u64;
    let packet = read_plaintext(stream, &mut buf).await?;
    plaintext_received += 1;
    if packet.opcode != CMSG_AUTH_CONTINUED_SESSION {
        bail!(
            "expected CMSG_AUTH_CONTINUED_SESSION on the instance connection, got opcode 0x{:04X}",
            packet.opcode
        );
    }
    let continued = AuthContinuedSession::parse(&packet.body)?;

    let key = ConnectToKey::from_raw(continued.key);
    if key.connection_type != CONNECTION_TYPE_INSTANCE {
        bail!("continued session key is not for an instance connection");
    }
    let pending = keys_store
        .redeem(continued.key)
        .context("unknown or already-used instance connect key")?;
    debug!(account = %pending.account, "instance connection redeemed its connect key");

    if !verify_continued_session(
        &pending.session_key40,
        continued.key,
        handshake.server_challenge(),
        &continued.local_challenge,
        &continued.digest,
    ) {
        bail!(
            "continued-session digest did not verify for account '{}'",
            pending.account
        );
    }

    // Keyed off the *realm* connection's 40-byte session key plus this connection's own
    // challenges, so the two sockets get independent ciphers from shared material. This is a
    // different derivation from the realm handshake's -- see the function's docs.
    let aes_key = derive_continued_session_aes_key(
        &pending.session_key40,
        handshake.server_challenge(),
        &continued.local_challenge,
    );
    let derived = DerivedKeys {
        session_key: pending.session_key40,
        aes_key,
    };

    stream
        .write_all(&handshake.enter_encrypted_mode_frame(&derived, signer))
        .await?;
    stream.flush().await?;

    tokio::time::timeout(
        ACK_TIMEOUT,
        read_plaintext_expecting(
            stream,
            &mut buf,
            CMSG_ENTER_ENCRYPTED_MODE_ACK,
            "CMSG_ENTER_ENCRYPTED_MODE_ACK on the instance connection",
            &mut plaintext_received,
        ),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "timed out waiting for CMSG_ENTER_ENCRYPTED_MODE_ACK on the instance connection"
        )
    })??;

    // Encryption is live. `SMSG_RESUME_COMMS` is empty and tells the client the world may start.
    let mut crypt = WorldCrypt::server_after_handshake(&derived.aes_key, plaintext_received);
    let frame = framing::encode(&mut crypt, Opcode::SMSG_RESUME_COMMS.modern(), &[]);
    stream.write_all(&frame).await?;
    stream.flush().await?;
    debug!(account = %pending.account, "instance connection ready");

    Ok((
        AuthedConnection {
            crypt,
            account: pending.account.clone(),
            account_id: pending.account_id,
            session_key40: derived.session_key,
        },
        pending,
    ))
}

/// Inbound opcodes the 1.14 client sends during its bootstrap sweep that we knowingly do not act on.
///
/// Without
/// it, routine client noise and a genuinely unhandled packet look identical in the trace. These
/// appear as `packet.ignored` rather than `packet.rx`.
///
/// Keep this in step with the no-op arm in `crate::handlers::dispatch_packet`.
fn is_known_benign(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::CMSG_QUEUED_MESSAGES_END
            | Opcode::CMSG_VIOLENCE_LEVEL
            | Opcode::CMSG_LOADING_SCREEN_NOTIFY
            | Opcode::CMSG_CHAT_REGISTER_ADDON_PREFIXES
            | Opcode::CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES
            | Opcode::CMSG_QUERY_COUNTDOWN_TIMER
            | Opcode::CMSG_REQUEST_CEMETERY_LIST
            | Opcode::CMSG_REQUEST_BATTLEFIELD_STATUS
            | Opcode::CMSG_LFG_LIST_GET_STATUS
            | Opcode::CMSG_BATTLE_PET_REQUEST_JOURNAL
            | Opcode::CMSG_ARENA_TEAM_ACCEPT
            | Opcode::CMSG_GUILD_SET_ACHIEVEMENT_TRACKING
            | Opcode::CMSG_GM_TICKET_GET_CASE_STATUS
            | Opcode::CMSG_UPDATE_VAS_PURCHASE_STATES
            | Opcode::CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS
            | Opcode::CMSG_BATTLE_PAY_GET_PRODUCT_LIST
            | Opcode::CMSG_BATTLE_PAY_GET_PURCHASE_LIST
            | Opcode::CMSG_TIME_SYNC_RESPONSE
            | Opcode::CMSG_TIME_SYNC_RESPONSE_FAILED
            | Opcode::CMSG_TIME_SYNC_RESPONSE_DROPPED
    )
}

/// Read plaintext packets until the expected opcode arrives, skipping the rest.
///
/// The client interleaves unsolicited packets into the handshake — most reliably
/// `CMSG_LOG_DISCONNECT`, which it sends on a freshly opened instance socket to report why the
/// *previous* connection ended. Treating the first packet as the ack turns that routine report into
/// a failed handshake, which is exactly what stalled the world socket at 70%.
async fn read_plaintext_expecting<S>(
    stream: &mut S,
    buf: &mut Vec<u8>,
    expected: u16,
    what: &str,
    received: &mut u64,
) -> Result<ModernPacket>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    /// Cap on skipped packets, so a client that never acks ends the connection rather than
    /// spinning here forever.
    const MAX_SKIPPED: usize = 16;

    for _ in 0..MAX_SKIPPED {
        let packet = read_plaintext(stream, buf).await?;
        // Counted whether or not we act on it: the client's nonce advances regardless.
        *received += 1;
        if packet.opcode == expected {
            return Ok(packet);
        }
        match Opcode::from_modern_wire(packet.opcode) {
            Some(opcode) => {
                debug!(?opcode, "skipping unsolicited packet while awaiting {what}")
            }
            None => debug!(
                opcode = format!("0x{:04X}", packet.opcode),
                "skipping unknown packet while awaiting {what}"
            ),
        }
    }

    bail!("gave up waiting for {what} after {MAX_SKIPPED} other packets")
}

/// Which of a modern client's two sockets this connection is.
///
/// The split is the client's, not ours: it serves the glue screen on one and the world on the
/// other, and refuses world traffic on the wrong one.
impl ConnectionRole {
    /// Short label for logs. Both sockets run the same loop, so without this it is impossible to
    /// tell from the log which one a packet arrived on -- or whether the world socket is receiving
    /// anything at all.
    fn label(&self) -> &'static str {
        match self {
            Self::Realm { .. } => "realm",
            Self::Instance { .. } => "instance",
        }
    }
}

pub enum ConnectionRole {
    /// Glue screen and character list. Answers `CMSG_PLAYER_LOGIN` with `SMSG_CONNECT_TO` and runs
    /// none of the login sequence itself.
    Realm {
        keys: Arc<ConnectKeyStore>,
        signer: Arc<dyn EnterEncryptedModeSigner>,
        /// Where to tell the client the world lives.
        instance_address: std::net::SocketAddr,
    },
    /// The world. Replays the login the realm connection deferred.
    Instance {
        player_guid: oxcore_shared::protocol::ObjectGuid,
    },
}

/// Run a modern gameplay connection through the shared `WorldSession` dispatcher.
pub async fn run_world_connection<S>(
    mut stream: S,
    mut conn: AuthedConnection,
    world: Arc<World>,
    role: ConnectionRole,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    // Only the realm connection evicts an existing session for the account -- that is a duplicate
    // login, and the older one should go. The instance connection is a *continuation* of the realm
    // one, so evicting here would tear down the very session that just handed us the connect key.
    if matches!(role, ConnectionRole::Realm { .. }) {
        world
            .session_mgr
            .remove_session_for_account(conn.account_id)
            .await;
    }
    let session_id = world.session_mgr.generate_session_id();
    let (packet_tx, mut packet_rx) = tokio::sync::mpsc::unbounded_channel();
    let session = Arc::new(WorldSession::new_with_protocol(
        session_id,
        conn.account_id,
        conn.account.clone(),
        0,
        Protocol::Modern,
        packet_tx,
    ));
    world.session_mgr.add_session(session.clone());

    // The realm connection deferred the login until the world socket existed; run it now.
    //
    // Calls the handler with the GUID directly rather than synthesising a `CMSG_PLAYER_LOGIN` to
    // re-parse: the body layout is protocol-specific, and this session is modern, so a hand-built
    // packet would have to reproduce a packed guid128 exactly to be read back as the number we
    // already have.
    if let ConnectionRole::Instance { player_guid } = &role {
        debug!(account = %conn.account, ?player_guid, "instance connection replaying deferred login");
        if let Err(e) = crate::handlers::character::handle_player_login_with_guid(
            &session,
            player_guid.raw(),
            &world.databases,
            &world,
        )
        .await
        {
            warn!(account = %conn.account, "deferred login failed: {e}");
        }
    }

    packet_log::event("session.start", role.label(), &conn.account);

    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let result = 'connection: loop {
        // Drain every whole packet the buffer holds, re-decoding each time round so that a
        // `continue` below still makes progress.
        loop {
            // A decode failure here is almost always a desynced cipher rather than a malformed
            // packet, and it ends the connection. Name it, or the socket just goes quiet.
            let decoded = match framing::decode(&mut conn.crypt, &buf) {
                Ok(decoded) => decoded,
                Err(e) => {
                    warn!(
                        account = %conn.account,
                        connection = role.label(),
                        "failed to decode packet (likely a desynced cipher): {e}"
                    );
                    break 'connection Err(e);
                }
            };
            let Some((packet, consumed)) = decoded else {
                break;
            };
            buf.drain(..consumed);
            let Some(opcode) = Opcode::from_modern_wire(packet.opcode) else {
                debug!(
                    account = %conn.account,
                    connection = role.label(),
                    opcode = format!("0x{:04X}", packet.opcode),
                    "unknown modern opcode"
                );
                packet_log::packet(
                    "packet.untranslated",
                    role.label(),
                    "?",
                    packet.opcode,
                    &packet.body,
                );
                continue;
            };
            debug!(account = %conn.account, connection = role.label(), ?opcode, "received");
            packet_log::packet(
                if is_known_benign(opcode) {
                    "packet.ignored"
                } else {
                    "packet.rx"
                },
                role.label(),
                opcode.name().unwrap_or("?"),
                packet.opcode,
                &packet.body,
            );
            // On the realm socket a login request is answered by handing the client to the world
            // socket, not by running the login sequence here -- the sequence is instance traffic.
            if let (
                Opcode::CMSG_PLAYER_LOGIN,
                ConnectionRole::Realm {
                    keys,
                    signer,
                    instance_address,
                },
            ) = (opcode, &role)
            {
                match handle_realm_player_login(
                    &mut stream,
                    &mut conn,
                    &packet.body,
                    keys,
                    signer.as_ref(),
                    *instance_address,
                )
                .await
                {
                    Ok(()) => {
                        debug!(account = %conn.account, "handed client to the instance connection")
                    }
                    Err(e) => {
                        warn!(account = %conn.account, "could not hand off to the world: {e}")
                    }
                }
                continue;
            }

            let mut world_packet = WorldPacket::new(opcode);
            world_packet.write_bytes(&packet.body);
            if let Err(e) = route_packet(
                Arc::clone(&session),
                world_packet,
                Arc::clone(&world.databases),
                Arc::clone(&world),
            )
            .await
            {
                warn!(account = %conn.account, ?opcode, "modern packet dispatch failed: {e}");
            }
        }

        tokio::select! {
            read = stream.read(&mut chunk) => {
                let n = read?;
                if n == 0 {
                    break Ok(());
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            packet = packet_rx.recv() => match packet {
                Some(packet) => {
                    if !packet.opcode().has_modern() {
                        debug!(?packet, "dropping packet without a modern opcode");
                        continue;
                    }
                    // Traced *before* encryption: the ciphertext is useless for checking a body,
                    // and this is the only point that still has the plaintext together with the
                    // opcode it will be sent under.
                    packet_log::packet(
                        "packet.tx",
                        role.label(),
                        packet.opcode().name().unwrap_or("?"),
                        packet.opcode().modern(),
                        packet.contents(),
                    );
                    let frame =
                        framing::encode(&mut conn.crypt, packet.opcode().modern(), packet.contents());
                    tracing::trace!(
                        connection = role.label(),
                        opcode = ?packet.opcode(),
                        bytes = frame.len(),
                        "sent"
                    );
                    stream.write_all(&frame).await?;
                    stream.flush().await?;
                }
                None => break Ok(()),
            },
        }
    };

    cleanup_disconnected_session(session_id, &world.session_mgr, &world).await;
    result
}

/// The post-auth loop: read **encrypted** frames off the stream and dispatch them. Only the
/// keep-alive `CMSG_PING` is answered so far (with `SMSG_PONG`); the gameplay opcodes — starting
/// with character enumeration — are the next milestone. Unknown opcodes are logged and skipped so
/// one unhandled packet does not drop the connection.
async fn run_connection<S>(mut stream: S, mut conn: AuthedConnection) -> Result<()>
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

    struct MapProvider(HashMap<String, (u32, Vec<u8>)>);
    impl SessionKeyProvider for MapProvider {
        async fn session_key(&self, ticket: &str) -> Result<Option<(u32, Vec<u8>)>> {
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

        let provider = MapProvider(HashMap::from([(TICKET.to_string(), (1, session_key()))]));
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
        assert_eq!(outcome.account_id, 1);
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
            account_id: 1,
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
        let provider = MapProvider(HashMap::from([(TICKET.to_string(), (1, vec![0xFFu8; 64]))]));
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

/// Answer a realm-socket `CMSG_PLAYER_LOGIN` with `SMSG_CONNECT_TO`.
///
/// Issues a single-use key naming the account and the chosen character, then points the client at
/// the instance listener. The login sequence runs on the socket the client opens next.
async fn handle_realm_player_login<S>(
    stream: &mut S,
    conn: &mut AuthedConnection,
    body: &[u8],
    keys: &ConnectKeyStore,
    signer: &dyn EnterEncryptedModeSigner,
    instance_address: std::net::SocketAddr,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    use oxcore_shared::protocol::bitbuf::BitReader;

    // Modern sends the character as a packed 128-bit GUID; only the counter identifies the row.
    let mut reader = BitReader::new(body);
    let (_high, low) = reader
        .read_packed_guid_128()
        .context("CMSG_PLAYER_LOGIN body is not a packed guid128")?;
    let player_guid = oxcore_shared::protocol::ObjectGuid::new_player(low as u32);

    let key = keys.issue(PendingInstance {
        account_id: conn.account_id,
        account: conn.account.clone(),
        session_key40: conn.session_key40,
        player_guid,
    });

    let body = connect_to_body(
        instance_address,
        CONNECT_TO_SERIAL_WORLD_ATTEMPT_1,
        key,
        signer,
    )
    .context("failed to build SMSG_CONNECT_TO")?;

    let frame = framing::encode(&mut conn.crypt, Opcode::SMSG_CONNECT_TO.modern(), &body);
    stream.write_all(&frame).await?;
    stream.flush().await?;
    Ok(())
}
