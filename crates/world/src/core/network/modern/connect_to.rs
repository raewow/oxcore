//! The instance-connection handshake: `SMSG_CONNECT_TO` and the key store behind it.
//!
//! A 1.14 client uses **two** sockets. The first (realm) serves the glue screen and character list.
//! When the player picks a character the server answers `CMSG_PLAYER_LOGIN` not with the login
//! sequence but with `SMSG_CONNECT_TO`, naming an address, port and one-shot key. The client opens
//! a second socket, presents that key in `CMSG_AUTH_CONTINUED_SESSION`, and *the world runs there*.
//!
//! This matters because most world packets — `SMSG_UPDATE_OBJECT` among them — are declared
//! `ConnectionType::Instance`. Sending them down the realm socket leaves the client
//! waiting on a connection that never opens, and then killing itself when world traffic arrives
//! somewhere it does not belong.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;
use sha2::{Digest, Sha256};

use super::packets::EnterEncryptedModeSigner;

/// `ConnectionType::Instance`. The realm connection is 0.
pub const CONNECTION_TYPE_INSTANCE: u8 = 1;

/// `ConnectToSerial::WorldAttempt1` — the first attempt at handing the client to the world.
///
/// The client retries with successive serials if a connection fails; we only ever issue the first,
/// and a `CMSG_CONNECT_TO_FAILED` naming a later one means the client could not reach the address
/// we advertised.
pub const CONNECT_TO_SERIAL_WORLD_ATTEMPT_1: u32 = 14;

/// The `IPv4` discriminant of the address union.
const ADDRESS_TYPE_IPV4: u8 = 1;

/// A connect key, packed the way the client echoes it back.
///
/// Layout: account id in the low 32 bits, connection type at bit
/// 32, and a random value above that. The client returns the whole `u64` verbatim, so the account
/// and connection type survive the round trip without any server-side lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectToKey {
    pub account_id: u32,
    pub connection_type: u8,
    pub key: u64,
}

impl ConnectToKey {
    pub fn to_raw(self) -> u64 {
        u64::from(self.account_id) | (u64::from(self.connection_type & 1) << 32) | (self.key << 33)
    }

    pub fn from_raw(raw: u64) -> Self {
        Self {
            account_id: (raw & 0xFFFF_FFFF) as u32,
            connection_type: ((raw >> 32) & 1) as u8,
            key: raw >> 33,
        }
    }
}

/// What an instance connection needs to resume the session the realm connection started.
#[derive(Debug, Clone)]
pub struct PendingInstance {
    pub account_id: u32,
    pub account: String,
    /// The session key the realm connection verified against, reused to key the instance cipher.
    pub session_key40: [u8; 40],
    /// The character the client asked to log in as.
    ///
    /// The realm socket answers `CMSG_PLAYER_LOGIN` with `SMSG_CONNECT_TO` and runs none of the
    /// login sequence itself, because that sequence belongs on the instance socket. The instance
    /// connection replays the request once it is up.
    pub player_guid: ObjectGuid,
}

/// Keys issued but not yet redeemed.
///
/// Entries are removed on use, so a key is good exactly once. A client that never connects leaves
/// an entry behind; they are small, and a reconnect issues a fresh one.
#[derive(Debug, Default)]
pub struct ConnectKeyStore {
    pending: DashMap<u64, PendingInstance>,
}

impl ConnectKeyStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Register a pending instance connection, returning the raw key to advertise.
    pub fn issue(&self, pending: PendingInstance) -> u64 {
        let raw = ConnectToKey {
            account_id: pending.account_id,
            connection_type: CONNECTION_TYPE_INSTANCE,
            key: rand::random::<u64>() >> 33,
        }
        .to_raw();
        self.pending.insert(raw, pending);
        raw
    }

    /// Redeem a key presented by an instance connection. `None` means it was never issued, was
    /// already used, or was forged.
    pub fn redeem(&self, raw: u64) -> Option<PendingInstance> {
        self.pending.remove(&raw).map(|(_, pending)| pending)
    }
}

/// Build the `SMSG_CONNECT_TO` body.
///
/// The signature covers the address, its type and the port, so a client cannot be redirected to
/// another host by anything that tampers with the packet in flight. It is signed with the same key
/// as `SMSG_ENTER_ENCRYPTED_MODE` and, like it, is byte-reversed on the wire.
pub fn connect_to_body(
    address: SocketAddr,
    serial: u32,
    key: u64,
    signer: &dyn EnterEncryptedModeSigner,
) -> Option<Vec<u8>> {
    let SocketAddr::V4(v4) = address else {
        // The address union does carry IPv6, but the client is only ever pointed at whatever the
        // realm list advertised, which is IPv4 here.
        tracing::error!(%address, "SMSG_CONNECT_TO needs an IPv4 instance address");
        return None;
    };
    let port = v4.port();

    let mut where_buffer = Vec::with_capacity(5);
    where_buffer.push(ADDRESS_TYPE_IPV4);
    where_buffer.extend_from_slice(&v4.ip().octets());

    // SHA-256 over the address bytes, then the address type as a u32, finishing with the port.
    let mut hasher = Sha256::new();
    hasher.update(&where_buffer);
    hasher.update(u32::from(ADDRESS_TYPE_IPV4).to_le_bytes());
    hasher.update(port.to_le_bytes());
    let digest: [u8; 32] = hasher.finalize().into();

    let signature = signer.sign(&digest);
    if signature.is_empty() {
        tracing::error!("failed to sign SMSG_CONNECT_TO; the client cannot reach the world");
        return None;
    }

    let mut body = Vec::with_capacity(signature.len() + where_buffer.len() + 15);
    body.extend_from_slice(&signature);
    body.extend_from_slice(&where_buffer);
    body.extend_from_slice(&port.to_le_bytes());
    body.extend_from_slice(&serial.to_le_bytes());
    body.push(CONNECTION_TYPE_INSTANCE);
    body.extend_from_slice(&key.to_le_bytes());
    Some(body)
}

/// The address to advertise: the externally reachable host, on the instance port.
pub fn instance_address(external_ip: Ipv4Addr, instance_port: u16) -> SocketAddr {
    SocketAddr::from((external_ip, instance_port))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client echoes the key back verbatim, so the account and connection type must survive
    /// the round trip — the instance socket has no other way to know who is connecting.
    #[test]
    fn connect_key_round_trips_through_its_packed_form() {
        let key = ConnectToKey {
            account_id: 4_294_967_290,
            connection_type: CONNECTION_TYPE_INSTANCE,
            key: 0x1234_5678,
        };
        assert_eq!(ConnectToKey::from_raw(key.to_raw()), key);
    }

    #[test]
    fn a_key_is_redeemable_exactly_once() {
        let store = ConnectKeyStore::new();
        let raw = store.issue(PendingInstance {
            account_id: 7,
            account: "tester".into(),
            session_key40: [0; 40],
            player_guid: ObjectGuid::from_low(4),
        });

        assert_eq!(store.redeem(raw).map(|p| p.account_id), Some(7));
        assert!(
            store.redeem(raw).is_none(),
            "a used key must not work twice"
        );
    }

    #[test]
    fn an_unissued_key_is_rejected() {
        assert!(ConnectKeyStore::new().redeem(0xDEAD_BEEF).is_none());
    }
}
