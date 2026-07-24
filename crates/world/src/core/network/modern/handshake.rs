//! The modern (1.14.x) world auth handshake, as an I/O-free state machine.
//!
//! Composes the framing ([`super::framing`]), the auth crypto ([`super::auth_crypto`]), and the
//! packet layer ([`super::packets`]) into the server side of the pre-encryption handshake:
//!
//! 1. send `SMSG_AUTH_CHALLENGE` (plaintext-framed),
//! 2. receive + parse `CMSG_AUTH_SESSION`, look the account up by its realm-join ticket, verify the
//!    digest against the persisted session key,
//! 3. derive the AES key, send `SMSG_ENTER_ENCRYPTED_MODE` (RSA-signed), and hand back the key so
//!    the socket can switch [`super::crypt::WorldCrypt`] on.
//!
//! This type produces and consumes bytes; the actual TCP socket + the plaintext connection-init
//! exchange + `SMSG_AUTH_RESPONSE` are wired in a later milestone (C4). **Unverified against a live
//! client.**

use super::auth_crypto::{derive_keys, verify_digest, DerivedKeys};
use super::crypt::WorldCrypt;
use super::framing;
use super::opcodes::{SMSG_AUTH_CHALLENGE, SMSG_AUTH_RESPONSE, SMSG_ENTER_ENCRYPTED_MODE};
use super::packets::{
    auth_response_success, enter_encrypted_mode, AuthChallenge, AuthResponseSuccess, AuthSession,
    EnterEncryptedModeSigner,
};

/// Server-side handshake state: the challenges we generated for this connection.
pub struct HandshakeServer {
    server_challenge: [u8; 16],
    dos_challenge: [u8; 32],
}

impl HandshakeServer {
    /// Start a handshake with freshly random challenges.
    pub fn new() -> Self {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut server_challenge = [0u8; 16];
        let mut dos_challenge = [0u8; 32];
        rng.fill_bytes(&mut server_challenge);
        rng.fill_bytes(&mut dos_challenge);
        Self {
            server_challenge,
            dos_challenge,
        }
    }

    #[cfg(test)]
    fn with_challenges(server_challenge: [u8; 16], dos_challenge: [u8; 32]) -> Self {
        Self {
            server_challenge,
            dos_challenge,
        }
    }

    /// The server challenge the client must mix into its digest.
    pub fn server_challenge(&self) -> &[u8; 16] {
        &self.server_challenge
    }

    /// The `SMSG_AUTH_CHALLENGE` frame to send first (plaintext, before encryption).
    pub fn auth_challenge_frame(&self) -> Vec<u8> {
        let body = AuthChallenge {
            dos_challenge: self.dos_challenge,
            challenge: self.server_challenge,
            dos_zero_bits: 1,
        }
        .encode();
        framing::encode_plaintext(SMSG_AUTH_CHALLENGE, &body)
    }

    /// Verify a parsed `CMSG_AUTH_SESSION` against the account's session key and one auth seed.
    /// Returns the derived keys (AES + continued-session) on success, `None` if the digest fails.
    pub fn verify_session(
        &self,
        session: &AuthSession,
        session_key: &[u8],
        seed: &[u8],
    ) -> Option<DerivedKeys> {
        if !verify_digest(
            session_key,
            seed,
            &session.local_challenge,
            &self.server_challenge,
            &session.digest,
        ) {
            return None;
        }
        Some(derive_keys(
            session_key,
            &self.server_challenge,
            &session.local_challenge,
        ))
    }

    /// The `SMSG_ENTER_ENCRYPTED_MODE` frame to send after a successful verify (plaintext-framed;
    /// encryption turns on only once the client acks).
    pub fn enter_encrypted_mode_frame(
        &self,
        keys: &DerivedKeys,
        signer: &dyn EnterEncryptedModeSigner,
    ) -> Vec<u8> {
        let body = enter_encrypted_mode(&keys.aes_key, true, signer);
        framing::encode_plaintext(SMSG_ENTER_ENCRYPTED_MODE, &body)
    }
}

/// The `SMSG_AUTH_RESPONSE` frame — the first **encrypted** packet, sent once the client has
/// acked encrypted mode and the socket has switched `crypt` on (keyed by `DerivedKeys::aes_key`).
pub fn auth_response_frame(crypt: &mut WorldCrypt, info: &AuthResponseSuccess) -> Vec<u8> {
    let body = auth_response_success(info);
    framing::encode(crypt, SMSG_AUTH_RESPONSE, &body)
}

impl Default for HandshakeServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::network::modern::auth_crypto::expected_digest;
    use crate::core::network::modern::framing::decode_plaintext;
    use crate::core::network::modern::opcodes::CMSG_AUTH_SESSION;

    // A stand-in bnet realm-join session key and per-build auth seed.
    fn session_key() -> Vec<u8> {
        (0..64u8).collect()
    }
    const SEED: [u8; 16] = [0x77; 16];

    /// Simulate a well-behaved client: compute the 24-byte digest from the session key and
    /// challenges exactly as the client would, and build a CMSG_AUTH_SESSION frame.
    fn client_session_frame(server_challenge: &[u8; 16], local_challenge: [u8; 16]) -> Vec<u8> {
        let full = expected_digest(&session_key(), &SEED, &local_challenge, server_challenge);
        let mut digest = [0u8; 24];
        digest.copy_from_slice(&full[..24]);

        let body = AuthSession {
            dos_response: 0,
            region_id: 1,
            battlegroup_id: 1,
            realm_id: 1,
            local_challenge,
            digest,
            use_ipv6: false,
            realm_join_ticket: r#"{"gameAccount":"PLAYER#1"}"#.to_string(),
        }
        .encode()
        .unwrap();
        framing::encode_plaintext(CMSG_AUTH_SESSION, &body)
    }

    #[test]
    fn a_correct_client_passes_and_both_sides_derive_the_same_aes_key() {
        let server = HandshakeServer::with_challenges([0x11; 16], [0x22; 32]);

        // Server sends the challenge; sanity-check it frames as SMSG_AUTH_CHALLENGE.
        let challenge_frame = server.auth_challenge_frame();
        let (packet, _) = decode_plaintext(&challenge_frame).unwrap().unwrap();
        assert_eq!(packet.opcode, SMSG_AUTH_CHALLENGE);

        // Client answers.
        let local_challenge = [0x42u8; 16];
        let frame = client_session_frame(server.server_challenge(), local_challenge);
        let (session_packet, _) = decode_plaintext(&frame).unwrap().unwrap();
        assert_eq!(session_packet.opcode, CMSG_AUTH_SESSION);
        let session = AuthSession::parse(&session_packet.body).unwrap();
        assert_eq!(session.realm_join_ticket, "PLAYER#1");

        // Server verifies and derives keys.
        let keys = server
            .verify_session(&session, &session_key(), &SEED)
            .expect("a correct digest must verify");

        // The client, knowing the same inputs, derives the same AES key.
        let client_keys = derive_keys(&session_key(), server.server_challenge(), &local_challenge);
        assert_eq!(keys.aes_key, client_keys.aes_key);
    }

    #[test]
    fn a_wrong_session_key_fails_verification() {
        let server = HandshakeServer::with_challenges([0x11; 16], [0x22; 32]);
        let frame = client_session_frame(server.server_challenge(), [0x42; 16]);
        let (packet, _) = decode_plaintext(&frame).unwrap().unwrap();
        let session = AuthSession::parse(&packet.body).unwrap();

        let wrong_key: Vec<u8> = (0..64u8).map(|b| b ^ 0xFF).collect();
        assert!(server.verify_session(&session, &wrong_key, &SEED).is_none());
    }

    struct StubSigner;
    impl EnterEncryptedModeSigner for StubSigner {
        fn sign(&self, _hash: &[u8; 32]) -> Vec<u8> {
            vec![0u8; 256]
        }
    }

    #[test]
    fn full_flow_the_encrypted_auth_response_decodes_on_the_client() {
        use crate::core::network::modern::crypt::WorldCrypt;
        use crate::core::network::modern::framing::decode;
        use crate::core::network::modern::opcodes::SMSG_AUTH_RESPONSE;
        use crate::core::network::modern::packets::AuthResponseSuccess;

        let server = HandshakeServer::with_challenges([0x11; 16], [0x22; 32]);
        let frame = client_session_frame(server.server_challenge(), [0x42; 16]);
        let (packet, _) = decode_plaintext(&frame).unwrap().unwrap();
        let session = AuthSession::parse(&packet.body).unwrap();
        let keys = server
            .verify_session(&session, &session_key(), &SEED)
            .unwrap();

        // Encryption turns on with the derived AES key. The server sends SMSG_AUTH_RESPONSE
        // encrypted; a client crypt keyed the same way decodes it.
        let mut server_crypt = WorldCrypt::server(&keys.aes_key);
        let mut client_crypt = WorldCrypt::client(&keys.aes_key);

        let info = AuthResponseSuccess {
            virtual_realm_address: 0x0101_0001,
            active_expansion: 0,
            account_expansion: 0,
            time: 42,
            available_classes: Vec::new(),
        };
        let response = auth_response_frame(&mut server_crypt, &info);

        let (decoded, consumed) = decode(&mut client_crypt, &response).unwrap().unwrap();
        assert_eq!(consumed, response.len());
        assert_eq!(decoded.opcode, SMSG_AUTH_RESPONSE);
        assert_eq!(&decoded.body[..4], &[0, 0, 0, 0]); // Result = Ok
    }

    #[test]
    fn enter_encrypted_mode_frame_is_a_well_formed_plaintext_packet() {
        let server = HandshakeServer::with_challenges([0x11; 16], [0x22; 32]);
        let frame = client_session_frame(server.server_challenge(), [0x42; 16]);
        let (packet, _) = decode_plaintext(&frame).unwrap().unwrap();
        let session = AuthSession::parse(&packet.body).unwrap();
        let keys = server
            .verify_session(&session, &session_key(), &SEED)
            .unwrap();

        let eem = server.enter_encrypted_mode_frame(&keys, &StubSigner);
        let (packet, consumed) = decode_plaintext(&eem).unwrap().unwrap();
        assert_eq!(consumed, eem.len());
        assert_eq!(packet.opcode, SMSG_ENTER_ENCRYPTED_MODE);
        assert_eq!(packet.body.len(), 256 + 1); // signature + flush byte
        assert_eq!(packet.body[256], 0x80); // enabled bit
    }
}
