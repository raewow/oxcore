//! Modern (1.14.x) world auth crypto: the digest check that proves the client holds the bnet
//! realm-join session key, and the derivation of the AES-128 key that keys [`super::crypt::WorldCrypt`].
//!
//! All three HMACs mix the client's `local_challenge` and the server's
//! `server_challenge` in a **specific order that differs between them** — see each function. The
//! input session key is the 64-byte `client_secret ‖ server_secret` minted at bnet realm-join and
//! persisted to `account.sessionkey`.
//!
//! **Unverified against a live client.** The algorithm is transcribed faithfully and is internally
//! self-consistent under test, but only a real 1.14 client can confirm the digest actually matches.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Fixed 16-byte seed constants for the handshake.
pub const AUTH_CHECK_SEED: [u8; 16] = [
    0xC5, 0xC6, 0x98, 0x95, 0x76, 0x3F, 0x1D, 0xCD, 0xB6, 0xA1, 0x37, 0x28, 0xB3, 0x12, 0xFF, 0x8A,
];
pub const SESSION_KEY_SEED: [u8; 16] = [
    0x58, 0xCB, 0xCF, 0x40, 0xFE, 0x2E, 0xCE, 0xA6, 0x5A, 0x90, 0xB8, 0x01, 0x68, 0x6C, 0x28, 0x0B,
];
pub const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA, 0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];
/// Used by the continued-session (instance) handshake, not the initial one — kept here so the seed
/// set lives in one place for milestone C4's `SMSG_CONNECT_TO` work.
pub const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2, 0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn hmac_sha256(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}

/// The digest the client must present in `CMSG_AUTH_SESSION`:
/// `HMAC-SHA256( SHA256(session_key ‖ seed); local_challenge ‖ server_challenge ‖ AuthCheckSeed )`.
pub fn expected_digest(
    session_key: &[u8],
    seed: &[u8],
    local_challenge: &[u8],
    server_challenge: &[u8],
) -> [u8; 32] {
    let digest_key = sha256(&[session_key, seed]);
    hmac_sha256(
        &digest_key,
        &[local_challenge, server_challenge, &AUTH_CHECK_SEED],
    )
}

/// Constant-time check of the client's digest against the value derived from `seed`.
///
/// The client sends a **truncated** digest (24 bytes in 1.14.x — see `CMSG_AUTH_SESSION`), so we
/// compare it against the leading bytes of our full 32-byte HMAC. A client digest longer than the
/// HMAC (or empty) never matches.
pub fn verify_digest(
    session_key: &[u8],
    seed: &[u8],
    local_challenge: &[u8],
    server_challenge: &[u8],
    client_digest: &[u8],
) -> bool {
    if client_digest.is_empty() || client_digest.len() > 32 {
        return false;
    }
    let expected = expected_digest(session_key, seed, local_challenge, server_challenge);
    constant_time_eq(&expected[..client_digest.len()], client_digest)
}

/// Derive the packet-cipher key for a **continued session** (the instance connection).
///
/// Not the same derivation as the realm connection's. There is no session-key expansion step: the
/// 40-byte key the realm connection already produced is used directly as the HMAC key, mixed with
/// *this* connection's challenges. Running the realm derivation here produces a plausible but wrong
/// key, and the client answers by closing the socket without a word.
///
/// Derives the AES key for a continued (instance) session.
pub fn derive_continued_session_aes_key(
    session_key40: &[u8; 40],
    server_challenge: &[u8],
    local_challenge: &[u8],
) -> [u8; 16] {
    let enc = hmac_sha256(
        session_key40,
        &[local_challenge, server_challenge, &ENCRYPTION_KEY_SEED],
    );
    let mut aes_key = [0u8; 16];
    aes_key.copy_from_slice(&enc[..16]);
    aes_key
}

/// Whether a continued-session digest proves the client holds the realm connection's session key.
///
/// Note the extra input the realm handshake has no equivalent of: the connect key itself, so a
/// digest cannot be replayed onto a different key.
pub fn verify_continued_session(
    session_key40: &[u8; 40],
    connect_key: u64,
    server_challenge: &[u8],
    local_challenge: &[u8],
    client_digest: &[u8],
) -> bool {
    let expected = hmac_sha256(
        session_key40,
        &[
            &connect_key.to_le_bytes(),
            local_challenge,
            server_challenge,
            &CONTINUED_SESSION_SEED,
        ],
    );
    constant_time_eq(&expected[..client_digest.len()], client_digest)
}

/// The session key derived after a successful auth: the 40-byte continued-session key and the
/// 16-byte AES key that keys the packet cipher.
#[derive(Debug, Clone)]
pub struct DerivedKeys {
    /// 40-byte key that becomes the session key for any later continued-session (instance) handshake.
    pub session_key: [u8; 40],
    /// 16-byte AES-128 key for [`super::crypt::WorldCrypt`].
    pub aes_key: [u8; 16],
}

/// Derive the continued-session key and the AES key from the bnet session key and the two
/// challenges. The two HMACs use different challenge orders.
pub fn derive_keys(
    session_key: &[u8],
    server_challenge: &[u8],
    local_challenge: &[u8],
) -> DerivedKeys {
    // prk = HMAC( SHA256(session_key); server_challenge ‖ local_challenge ‖ SessionKeySeed )
    let key_data = sha256(&[session_key]);
    let prk = hmac_sha256(
        &key_data,
        &[server_challenge, local_challenge, &SESSION_KEY_SEED],
    );

    // Expand the 32-byte prk into a 40-byte session key.
    let mut session_key40 = [0u8; 40];
    session_key_generator(&prk, &mut session_key40);

    // aes = HMAC( session_key40; local_challenge ‖ server_challenge ‖ EncryptionKeySeed )[..16]
    let enc = hmac_sha256(
        &session_key40,
        &[local_challenge, server_challenge, &ENCRYPTION_KEY_SEED],
    );
    let mut aes_key = [0u8; 16];
    aes_key.copy_from_slice(&enc[..16]);

    DerivedKeys {
        session_key: session_key40,
        aes_key,
    }
}

/// Expand a 32-byte seed into `out.len()` bytes.
///
/// `o1 = SHA256(seed[..16])`, `o2 = SHA256(seed[16..])`, then repeatedly
/// `o0 = SHA256(o1 ‖ o0 ‖ o2)` (o0 starts all-zero), streaming 32 bytes at a time.
fn session_key_generator(seed: &[u8; 32], out: &mut [u8]) {
    let o1 = sha256(&[&seed[..16]]);
    let o2 = sha256(&[&seed[16..]]);
    let mut o0 = [0u8; 32];

    let fill = |o0: &mut [u8; 32]| {
        *o0 = sha256(&[&o1, &o0[..], &o2]);
    };

    fill(&mut o0);
    let mut taken = 0usize;
    for byte in out.iter_mut() {
        if taken == 32 {
            fill(&mut o0);
            taken = 0;
        }
        *byte = o0[taken];
        taken += 1;
    }
}

/// Length-checked constant-time byte comparison.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // A stand-in 64-byte bnet realm-join session key (client_secret ‖ server_secret).
    fn session_key() -> Vec<u8> {
        (0..64u8).collect()
    }
    const SERVER_CHALLENGE: [u8; 16] = [0x11; 16];
    const LOCAL_CHALLENGE: [u8; 16] = [0x22; 16];
    const SEED: [u8; 16] = [0x33; 16];

    #[test]
    fn a_client_that_knows_the_key_produces_the_expected_digest() {
        let digest = expected_digest(&session_key(), &SEED, &LOCAL_CHALLENGE, &SERVER_CHALLENGE);
        assert!(verify_digest(
            &session_key(),
            &SEED,
            &LOCAL_CHALLENGE,
            &SERVER_CHALLENGE,
            &digest
        ));
    }

    #[test]
    fn a_truncated_24_byte_digest_verifies_against_the_hmac_prefix() {
        // The 1.14.x client sends only the first 24 bytes of the HMAC.
        let full = expected_digest(&session_key(), &SEED, &LOCAL_CHALLENGE, &SERVER_CHALLENGE);
        assert!(verify_digest(
            &session_key(),
            &SEED,
            &LOCAL_CHALLENGE,
            &SERVER_CHALLENGE,
            &full[..24]
        ));
        // A wrong-length or empty digest is rejected.
        assert!(!verify_digest(
            &session_key(),
            &SEED,
            &LOCAL_CHALLENGE,
            &SERVER_CHALLENGE,
            &[]
        ));
    }

    #[test]
    fn a_wrong_key_fails_the_digest() {
        let digest = expected_digest(&session_key(), &SEED, &LOCAL_CHALLENGE, &SERVER_CHALLENGE);
        let wrong_key: Vec<u8> = (0..64u8).map(|b| b ^ 0xFF).collect();
        assert!(!verify_digest(
            &wrong_key,
            &SEED,
            &LOCAL_CHALLENGE,
            &SERVER_CHALLENGE,
            &digest
        ));
    }

    #[test]
    fn digest_depends_on_challenge_order() {
        // Swapping local/server challenges must change the digest (guards the ‖ order).
        let a = expected_digest(&session_key(), &SEED, &LOCAL_CHALLENGE, &SERVER_CHALLENGE);
        let b = expected_digest(&session_key(), &SEED, &SERVER_CHALLENGE, &LOCAL_CHALLENGE);
        assert_ne!(a, b);
    }

    /// The instance connection must key straight off the realm's 40-byte session key, **not** run
    /// the full realm derivation over it again.
    ///
    /// This is the bug that made the client close the instance socket without a word: both produce
    /// a well-formed 16-byte key, so nothing fails locally — the client simply cannot decrypt and
    /// hangs up. The two must not agree on the same input.
    #[test]
    fn continued_session_does_not_re_expand_the_session_key() {
        let session_key40 =
            derive_keys(&session_key(), &SERVER_CHALLENGE, &LOCAL_CHALLENGE).session_key;

        let correct =
            derive_continued_session_aes_key(&session_key40, &SERVER_CHALLENGE, &LOCAL_CHALLENGE);
        // What the driver did at first: treat the 40-byte key as a fresh bnet session key.
        let wrong = derive_keys(&session_key40, &SERVER_CHALLENGE, &LOCAL_CHALLENGE).aes_key;

        assert_ne!(
            correct, wrong,
            "re-expanding the session key yields the wrong cipher key"
        );
    }

    /// The realm handshake's final step is the same HMAC, so deriving from its own session key with
    /// its own challenges reproduces its AES key. That shared step is why only the *input* differs.
    #[test]
    fn the_two_derivations_share_their_final_hmac() {
        let realm = derive_keys(&session_key(), &SERVER_CHALLENGE, &LOCAL_CHALLENGE);
        assert_eq!(
            derive_continued_session_aes_key(
                &realm.session_key,
                &SERVER_CHALLENGE,
                &LOCAL_CHALLENGE
            ),
            realm.aes_key
        );
    }

    /// The connect key is mixed into the digest, so one cannot be replayed against another key.
    #[test]
    fn continued_session_digest_is_bound_to_its_connect_key() {
        let session_key40 =
            derive_keys(&session_key(), &SERVER_CHALLENGE, &LOCAL_CHALLENGE).session_key;
        let digest = hmac_sha256(
            &session_key40,
            &[
                &7u64.to_le_bytes(),
                &LOCAL_CHALLENGE[..],
                &SERVER_CHALLENGE[..],
                &CONTINUED_SESSION_SEED,
            ],
        );

        assert!(verify_continued_session(
            &session_key40,
            7,
            &SERVER_CHALLENGE,
            &LOCAL_CHALLENGE,
            &digest
        ));
        assert!(
            !verify_continued_session(
                &session_key40,
                8,
                &SERVER_CHALLENGE,
                &LOCAL_CHALLENGE,
                &digest
            ),
            "a digest must not verify against a different connect key"
        );
    }

    #[test]
    fn derive_keys_is_deterministic_and_sized() {
        let k1 = derive_keys(&session_key(), &SERVER_CHALLENGE, &LOCAL_CHALLENGE);
        let k2 = derive_keys(&session_key(), &SERVER_CHALLENGE, &LOCAL_CHALLENGE);
        assert_eq!(k1.aes_key, k2.aes_key);
        assert_eq!(k1.session_key, k2.session_key);
        // The 40-byte session key and 16-byte AES key are distinct material.
        assert_ne!(&k1.session_key[..16], &k1.aes_key[..]);
    }

    #[test]
    fn session_key_generator_expands_deterministically_across_the_32_byte_boundary() {
        let seed = [0x5Au8; 32];
        let mut a = [0u8; 40];
        let mut b = [0u8; 40];
        session_key_generator(&seed, &mut a);
        session_key_generator(&seed, &mut b);
        assert_eq!(a, b);

        // The first 32 bytes come from block 1; asking for just 32 must match the 40-byte prefix.
        let mut first32 = [0u8; 32];
        session_key_generator(&seed, &mut first32);
        assert_eq!(&a[..32], &first32[..]);
    }

    #[test]
    fn session_key_generator_matches_a_pinned_reference_block() {
        // Regression snapshot: block 1 = SHA256(o1 ‖ zeros(32) ‖ o2) for seed = 0x5A*32.
        // Computed by this implementation; guards against silent algorithm drift, not a
        // client-verified vector.
        let seed = [0x5Au8; 32];
        let o1 = sha256(&[&seed[..16]]);
        let o2 = sha256(&[&seed[16..]]);
        let expected_block1 = sha256(&[&o1, &[0u8; 32][..], &o2]);

        let mut out = [0u8; 32];
        session_key_generator(&seed, &mut out);
        assert_eq!(out, expected_block1);
    }
}
