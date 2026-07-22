//! Modern (1.14.x) auth-handshake packet bodies.
//!
//! Layouts transcribed from HermesProxy's `AuthenticationPackets.cs`. These are the packet
//! *bodies* (opcode-stripped); framing lives in [`super::framing`]. Only the handshake packets
//! are modelled here — `SMSG_AUTH_RESPONSE` and the char-enum opcodes follow in a later milestone.
//!
//! Bit-packing note: the modern client bit-packs some fields, but every bit field in these packets
//! is flushed to a whole byte with the value in the top bit (MSB-first), so we read/write it as a
//! plain byte masked with `0x80` rather than needing a general bit engine.

use anyhow::{bail, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// The 16-byte seed mixed into the `SMSG_ENTER_ENCRYPTED_MODE` signature
/// (`EnableEncryptionSeed`, from HermesProxy/TrinityCore).
pub const ENABLE_ENCRYPTION_SEED: [u8; 16] = [
    0x90, 0x9C, 0xD0, 0x50, 0x5A, 0x2C, 0x14, 0xDD, 0x5C, 0x2C, 0xC0, 0x64, 0x14, 0xF3, 0xFE, 0xC9,
];

// ---- SMSG_AUTH_CHALLENGE (server -> client) ----

/// The auth challenge. On the wire: `DosChallenge[32] ‖ Challenge[16] ‖ DosZeroBits(u8)`.
#[derive(Debug, Clone)]
pub struct AuthChallenge {
    pub dos_challenge: [u8; 32],
    pub challenge: [u8; 16],
    pub dos_zero_bits: u8,
}

impl AuthChallenge {
    /// Serialize the packet body.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 16 + 1);
        out.extend_from_slice(&self.dos_challenge);
        out.extend_from_slice(&self.challenge);
        out.push(self.dos_zero_bits);
        out
    }
}

// ---- CMSG_AUTH_SESSION (client -> server) ----

/// The auth session the client sends to prove it holds the realm-join session key.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub dos_response: u64,
    pub region_id: u32,
    pub battlegroup_id: u32,
    pub realm_id: u32,
    pub local_challenge: [u8; 16],
    /// A 24-byte truncated HMAC — compared against the first 24 bytes of our computed digest.
    pub digest: [u8; 24],
    pub use_ipv6: bool,
    /// The game-account name behind the realm-join ticket. If the ticket is the JSON document our
    /// bnet server issues, this is its `gameAccount` field; otherwise the raw ticket string.
    pub realm_join_ticket: String,
}

impl AuthSession {
    /// Parse the packet body.
    ///
    /// Layout: `DosResponse(u64) ‖ RegionID(u32) ‖ BattlegroupID(u32) ‖ RealmID(u32) ‖
    /// LocalChallenge[16] ‖ Digest[24] ‖ <bit-flush byte: UseIPv6 in bit 7> ‖
    /// RealmJoinTicketSize(u32) ‖ RealmJoinTicket[size]`.
    pub fn parse(body: &[u8]) -> Result<Self> {
        let mut r = Reader::new(body);
        let dos_response = r.u64()?;
        let region_id = r.u32()?;
        let battlegroup_id = r.u32()?;
        let realm_id = r.u32()?;
        let local_challenge = r.array::<16>()?;
        let digest = r.array::<24>()?;

        // One flushed byte holds the UseIPv6 bit (MSB-first, so bit 7).
        let bit_byte = r.u8()?;
        let use_ipv6 = bit_byte & 0x80 != 0;

        let ticket_size = r.u32()? as usize;
        let ticket_bytes = r.take(ticket_size)?;
        let raw = String::from_utf8_lossy(ticket_bytes).into_owned();
        let realm_join_ticket = extract_game_account(&raw);

        Ok(Self {
            dos_response,
            region_id,
            battlegroup_id,
            realm_id,
            local_challenge,
            digest,
            use_ipv6,
            realm_join_ticket,
        })
    }
}

/// The realm-join ticket is the JSON document our bnet server issues (`{"gameAccount":…}`); pull
/// the account name out of it, falling back to the raw string if it is not that JSON.
fn extract_game_account(raw: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(name) = value.get("gameAccount").and_then(|v| v.as_str()) {
            return name.to_string();
        }
    }
    raw.to_string()
}

// ---- SMSG_ENTER_ENCRYPTED_MODE (server -> client) ----

/// Signs the 32-byte encrypted-mode hash. The production implementation is RSA-PKCS1-v1.5 over
/// SHA-256 with the signature bytes **reversed**, using a key whose public modulus is patched into
/// the client (`CONNECT_TO_MODULUS`); that lands with the patcher work. Behind a trait so the
/// packet layer composes and tests without the keypair.
pub trait EnterEncryptedModeSigner {
    /// Return the signature bytes exactly as they go on the wire (already reversed for the
    /// RSA implementation).
    fn sign(&self, hash: &[u8; 32]) -> Vec<u8>;
}

/// Compute the `SMSG_ENTER_ENCRYPTED_MODE` body: the signature over
/// `HMAC-SHA256(aes_key; [enabled] ‖ EnableEncryptionSeed)`, then a flushed byte carrying the
/// `enabled` bit in bit 7.
pub fn enter_encrypted_mode(
    aes_key: &[u8; 16],
    enabled: bool,
    signer: &dyn EnterEncryptedModeSigner,
) -> Vec<u8> {
    let hash = enter_encrypted_mode_hash(aes_key, enabled);
    let signature = signer.sign(&hash);

    let mut out = Vec::with_capacity(signature.len() + 1);
    out.extend_from_slice(&signature);
    out.push(if enabled { 0x80 } else { 0x00 });
    out
}

/// The hash the encrypted-mode signature is computed over.
pub fn enter_encrypted_mode_hash(aes_key: &[u8; 16], enabled: bool) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(aes_key).expect("HMAC accepts any key length");
    mac.update(&[enabled as u8]);
    mac.update(&ENABLE_ENCRYPTION_SEED);
    mac.finalize().into_bytes().into()
}

// ---- small byte reader ----

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.buf.len())
            .ok_or_else(|| anyhow::anyhow!("packet truncated: wanted {n} more bytes"))?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }
}

// A serializer for CMSG_AUTH_SESSION, used only by tests (to drive the parser) and, later, by any
// client-role tooling.
#[cfg(test)]
impl AuthSession {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.realm_join_ticket.len() > u32::MAX as usize {
            bail!("realm-join ticket too long");
        }
        let mut out = Vec::new();
        out.extend_from_slice(&self.dos_response.to_le_bytes());
        out.extend_from_slice(&self.region_id.to_le_bytes());
        out.extend_from_slice(&self.battlegroup_id.to_le_bytes());
        out.extend_from_slice(&self.realm_id.to_le_bytes());
        out.extend_from_slice(&self.local_challenge);
        out.extend_from_slice(&self.digest);
        out.push(if self.use_ipv6 { 0x80 } else { 0x00 });
        out.extend_from_slice(&(self.realm_join_ticket.len() as u32).to_le_bytes());
        out.extend_from_slice(self.realm_join_ticket.as_bytes());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_challenge_encodes_in_dos_challenge_first_order() {
        let challenge = AuthChallenge {
            dos_challenge: [0xAA; 32],
            challenge: [0xBB; 16],
            dos_zero_bits: 1,
        };
        let body = challenge.encode();
        assert_eq!(body.len(), 49);
        assert_eq!(&body[..32], &[0xAA; 32]); // DosChallenge first
        assert_eq!(&body[32..48], &[0xBB; 16]); // then Challenge
        assert_eq!(body[48], 1); // then DosZeroBits
    }

    fn sample_session(ticket: &str) -> AuthSession {
        AuthSession {
            dos_response: 0x1122_3344_5566_7788,
            region_id: 1,
            battlegroup_id: 2,
            realm_id: 3,
            local_challenge: [0x22; 16],
            digest: [0x33; 24],
            use_ipv6: false,
            realm_join_ticket: ticket.to_string(),
        }
    }

    #[test]
    fn auth_session_round_trips_through_encode_and_parse() {
        let original = sample_session("PLAYER#1");
        let body = original.encode().unwrap();
        let parsed = AuthSession::parse(&body).unwrap();

        assert_eq!(parsed.dos_response, original.dos_response);
        assert_eq!(parsed.region_id, 1);
        assert_eq!(parsed.realm_id, 3);
        assert_eq!(parsed.local_challenge, [0x22; 16]);
        assert_eq!(parsed.digest, [0x33; 24]);
        assert!(!parsed.use_ipv6);
        assert_eq!(parsed.realm_join_ticket, "PLAYER#1");
    }

    #[test]
    fn auth_session_extracts_game_account_from_a_json_ticket() {
        let session = sample_session(r#"{"gameAccount":"WOWACCOUNT#1","platform":0,"type":0}"#);
        let parsed = AuthSession::parse(&session.encode().unwrap()).unwrap();
        assert_eq!(parsed.realm_join_ticket, "WOWACCOUNT#1");
    }

    #[test]
    fn auth_session_use_ipv6_bit_is_read_from_the_flush_byte() {
        let mut session = sample_session("X");
        session.use_ipv6 = true;
        let parsed = AuthSession::parse(&session.encode().unwrap()).unwrap();
        assert!(parsed.use_ipv6);
    }

    #[test]
    fn auth_session_parse_rejects_a_truncated_body() {
        assert!(AuthSession::parse(&[0u8; 10]).is_err());
    }

    struct CapturingSigner {
        captured: std::cell::RefCell<Option<[u8; 32]>>,
    }
    impl EnterEncryptedModeSigner for CapturingSigner {
        fn sign(&self, hash: &[u8; 32]) -> Vec<u8> {
            *self.captured.borrow_mut() = Some(*hash);
            vec![0x5A; 256] // stand-in for a 256-byte RSA signature
        }
    }

    #[test]
    fn enter_encrypted_mode_signs_the_expected_hash_and_appends_the_enabled_bit() {
        let aes_key = [0x11u8; 16];
        let signer = CapturingSigner {
            captured: std::cell::RefCell::new(None),
        };
        let body = enter_encrypted_mode(&aes_key, true, &signer);

        // The signer saw exactly HMAC(aes_key, [1] ‖ EnableEncryptionSeed).
        assert_eq!(
            signer.captured.borrow().unwrap(),
            enter_encrypted_mode_hash(&aes_key, true)
        );
        // Body = 256-byte signature then a flush byte with the enabled bit set.
        assert_eq!(body.len(), 257);
        assert_eq!(&body[..256], &[0x5A; 256]);
        assert_eq!(body[256], 0x80);
    }

    #[test]
    fn enter_encrypted_mode_hash_depends_on_the_enabled_flag() {
        let key = [9u8; 16];
        assert_ne!(
            enter_encrypted_mode_hash(&key, true),
            enter_encrypted_mode_hash(&key, false)
        );
    }
}
