//! Modern (1.14.x) world packet header encryption: AES-128-GCM.
//!
//! Unlike the vanilla RC4-drop cipher in [`crate::core::network::crypt`], the modern client
//! encrypts each packet's `opcode ‖ body` with AES-128-GCM and prepends a plaintext size and the
//! 12-byte GCM tag. Transcribed from HermesProxy's `PacketCrypt.cs` (`WorldCrypt`) / TrinityCore.
//!
//! The nonce is `counter (u64 LE) ‖ tag-id (u32 LE)`, where the tag-id distinguishes the two
//! directions — `"SRVR"` for server→client and `"CLNT"` for client→server — and the counter
//! increments once per packet, per direction, starting at zero. Both directions use the same
//! AES key; only the nonce differs, so a single cipher instance serves both.

use aes_gcm::aead::consts::U12;
use aes_gcm::aead::AeadInPlace;
use aes_gcm::aes::Aes128;
use aes_gcm::{AesGcm, KeyInit};
use anyhow::{anyhow, Result};

/// AES-128-GCM with a 12-byte nonce and a **12-byte** tag (the modern header packs the tag into 16
/// bytes alongside a u32 size, leaving 12 for the tag).
type Aes128Gcm12 = AesGcm<Aes128, U12, U12>;

/// Length of the GCM authentication tag, in bytes.
pub const TAG_SIZE: usize = 12;

/// Nonce tag-id for server→client packets (`"SRVR"`, little-endian).
const SERVER_TAG: u32 = 0x5256_5253;
/// Nonce tag-id for client→server packets (`"CLNT"`, little-endian).
const CLIENT_TAG: u32 = 0x544E_4C43;

/// Per-connection AES-GCM state. Holds one cipher plus the outgoing/incoming nonce tag-ids and
/// packet counters. Construct with [`WorldCrypt::server`] once the AES key has been derived from
/// the session key (see Part C's key-derivation step, milestone C2).
pub struct WorldCrypt {
    cipher: Aes128Gcm12,
    out_tag: u32,
    in_tag: u32,
    out_counter: u64,
    in_counter: u64,
}

impl WorldCrypt {
    /// Server-side crypt: encrypts outgoing packets as `"SRVR"`, decrypts incoming as `"CLNT"`.
    pub fn server(key: &[u8; 16]) -> Self {
        Self::with_roles(key, SERVER_TAG, CLIENT_TAG)
    }

    /// Server-side crypt after the plaintext authentication exchange. The client advances the
    /// nonce sequence for all frames, including the two plaintext frames in each direction.
    pub fn server_after_handshake(key: &[u8; 16], plaintext_received: u64) -> Self {
        let mut crypt = Self::server(key);
        // The GCM nonce is a per-direction packet counter, and both sides count the plaintext
        // handshake frames too. We always send exactly two before encryption starts.
        crypt.out_counter = 2; // SMSG_AUTH_CHALLENGE, SMSG_ENTER_ENCRYPTED_MODE
                               // Inbound is *not* fixed: the client interleaves unsolicited packets into the handshake
                               // (`CMSG_LOG_DISCONNECT` on a fresh instance socket, reliably). Each one advances its
                               // counter, so assuming two here desyncs the cipher against a client that sent three, and
                               // every packet it goes on to send fails to decrypt.
        crypt.in_counter = plaintext_received;
        crypt
    }

    fn with_roles(key: &[u8; 16], out_tag: u32, in_tag: u32) -> Self {
        let cipher = Aes128Gcm12::new_from_slice(key).expect("AES-128 key is always 16 bytes here");
        Self {
            cipher,
            out_tag,
            in_tag,
            out_counter: 0,
            in_counter: 0,
        }
    }

    /// Encrypt `data` in place (outgoing direction) and return the 12-byte tag. Advances the
    /// outgoing counter.
    pub fn encrypt(&mut self, data: &mut [u8]) -> [u8; TAG_SIZE] {
        let nonce = build_nonce(self.out_counter, self.out_tag);
        let tag = self
            .cipher
            .encrypt_in_place_detached((&nonce).into(), &[], data)
            .expect("AES-GCM encryption cannot fail with an in-memory buffer");
        self.out_counter = self.out_counter.wrapping_add(1);

        let mut out = [0u8; TAG_SIZE];
        out.copy_from_slice(&tag);
        out
    }

    /// Decrypt `data` in place (incoming direction), verifying `tag`. Advances the incoming counter
    /// only on success — a failed tag check must not desynchronise the stream from a benign
    /// truncation, though in practice a GCM failure means the connection is unusable.
    pub fn decrypt(&mut self, data: &mut [u8], tag: &[u8; TAG_SIZE]) -> Result<()> {
        let nonce = build_nonce(self.in_counter, self.in_tag);
        self.cipher
            .decrypt_in_place_detached((&nonce).into(), &[], data, tag.into())
            .map_err(|_| anyhow!("AES-GCM tag verification failed"))?;
        self.in_counter = self.in_counter.wrapping_add(1);
        Ok(())
    }

    /// Client-side crypt (for tests and, later, any client-role tooling): the mirror of
    /// [`WorldCrypt::server`].
    #[cfg(test)]
    pub fn client(key: &[u8; 16]) -> Self {
        Self::with_roles(key, CLIENT_TAG, SERVER_TAG)
    }

    /// Client-side counterpart to [`WorldCrypt::server_after_handshake`].
    #[cfg(test)]
    pub fn client_after_handshake(key: &[u8; 16]) -> Self {
        let mut crypt = Self::client(key);
        crypt.out_counter = 2;
        crypt.in_counter = 2;
        crypt
    }
}

/// Build the 12-byte nonce: `counter (u64 LE) ‖ tag-id (u32 LE)`.
fn build_nonce(counter: u64, tag: u32) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&counter.to_le_bytes());
    nonce[8..].copy_from_slice(&tag.to_le_bytes());
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
        0xFF,
    ];

    #[test]
    fn nonce_layout_is_counter_then_tag_id() {
        // "SRVR" little-endian is 53 52 56 52.
        let nonce = build_nonce(1, SERVER_TAG);
        assert_eq!(&nonce[..8], &1u64.to_le_bytes());
        assert_eq!(&nonce[8..], &[0x53, 0x52, 0x56, 0x52]);
    }

    #[test]
    fn client_to_server_round_trips() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        let plaintext = b"CMSG_AUTH_SESSION payload".to_vec();
        let mut buf = plaintext.clone();
        let tag = client.encrypt(&mut buf);
        assert_ne!(buf, plaintext, "ciphertext must differ from plaintext");

        server.decrypt(&mut buf, &tag).unwrap();
        assert_eq!(buf, plaintext);
    }

    #[test]
    fn server_to_client_round_trips() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        let plaintext = b"SMSG_AUTH_RESPONSE payload".to_vec();
        let mut buf = plaintext.clone();
        let tag = server.encrypt(&mut buf);
        client.decrypt(&mut buf, &tag).unwrap();
        assert_eq!(buf, plaintext);
    }

    #[test]
    fn counters_advance_so_successive_packets_decrypt_in_order() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        for i in 0u8..4 {
            let plaintext = vec![i; 32];
            let mut buf = plaintext.clone();
            let tag = client.encrypt(&mut buf);
            server.decrypt(&mut buf, &tag).unwrap();
            assert_eq!(buf, plaintext);
        }
    }

    #[test]
    fn out_of_order_or_tampered_packets_fail_the_tag() {
        let mut server = WorldCrypt::server(&KEY);
        let mut client = WorldCrypt::client(&KEY);

        // Encrypt two packets but try to decrypt the second before the first: counter mismatch.
        let mut first = b"first".to_vec();
        let _tag0 = client.encrypt(&mut first);
        let mut second = b"second".to_vec();
        let tag1 = client.encrypt(&mut second);

        // Server is still at in_counter = 0, so the counter-1 packet must not verify.
        assert!(server.decrypt(&mut second, &tag1).is_err());
    }
}
