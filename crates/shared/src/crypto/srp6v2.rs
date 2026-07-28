//! Blizzard's SRP6v2, as the modern (1.14.x) client runs it over the REST login service.
//!
//! Lives in `oxcore-shared` so both the bnet login server and the account repository
//! (`create_account`) can compute verifiers from one implementation.
//!
//! This is a faithful port of TrinityCore's `BnetSRP6v2<SHA256>`
//! (`src/common/Cryptography/Authentication/SRP6.{h,cpp}`). It shares nothing with the vanilla
//! SRP in `crates/auth` beyond the acronym: different prime, different generator handling,
//! SHA-256 instead of SHA-1, a PBKDF2-derived private exponent, and a different evidence
//! construction.
//!
//! ## Parameters (all transcribed verbatim from TrinityCore)
//!
//! - `N` — the RFC 5054 2048-bit group prime; `g = 2`.
//! - `k = H(N ‖ g)` (SRP-6a), each padded big-endian to 256 bytes.
//! - Hash `H` = SHA-256; salt = 32 bytes; PBKDF2-HMAC-SHA512 with 15000 iterations.
//! - **All** BigNumber↔bytes conversions are **big-endian** (TrinityCore passes `false`, whose
//!   default parameter name is `littleEndian`).
//!
//! ## SRP username
//!
//! The identity fed to the KDF is not the raw email but `HEX(SHA256(UPPER(login)))` — the same
//! value the server echoes back to the client in the challenge's `username` field, so both
//! sides feed an identical string into PBKDF2.
//!
//! ## Evidence
//!
//! `M1 = H(broken(A) ‖ broken(B) ‖ broken(S))`, `M2 = H(broken(A) ‖ broken(M1) ‖ broken(S))`,
//! where `broken(n)` is `n` big-endian in `(bits(n)+8)/8` bytes — one extra leading zero byte
//! whenever the bit length is an exact multiple of 8 (an OpenSSL BN quirk Blizzard preserved).
//! The session key is the raw premaster secret `S`, **not** hashed.
//!
//! ## Interop caveat
//!
//! The math is unit-tested for self-consistency (a matching client proof verifies and both
//! sides derive the same `S`/`M2`). It has **not** been checked against a live client — the
//! documented stopping point for this work. The two details most likely to need adjustment if
//! interop fails are the private-exponent MSB fix in [`compute_x`] and the `broken` evidence
//! width; both are isolated here for that reason.

use num_bigint::{BigInt, BigUint, Sign};
use num_traits::Zero;
use sha2::{Digest, Sha256, Sha512};

/// Salt length in bytes.
pub const SALT_LENGTH: usize = 32;
/// PBKDF2 iterations used to derive the private exponent `x` (SRP version 2).
pub const X_ITERATIONS: u32 = 15_000;
/// Width, in bytes, of the group prime `N`.
pub const N_BYTES: usize = 256;
/// SRP version advertised in the challenge.
pub const SRP_VERSION: u32 = 2;
/// Hash function name advertised in the challenge.
pub const HASH_FUNCTION: &str = "SHA256";

/// RFC 5054 2048-bit group prime, big-endian hex.
const N_HEX: &str = "AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050\
A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50\
E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B8\
55F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B14773B\
CA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87748\
544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E57AE6\
AF874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9DBFBB6\
94B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E4AFF73";

fn prime_n() -> BigUint {
    BigUint::from_bytes_be(&hex::decode(N_HEX).expect("N_HEX is valid hex"))
}

fn generator() -> BigUint {
    BigUint::from(2u32)
}

/// `n` big-endian in exactly `width` bytes, left-padded with zeros. Panics if `n` does not fit,
/// which cannot happen for any value reduced mod `N` written at width [`N_BYTES`].
fn to_fixed_be(n: &BigUint, width: usize) -> Vec<u8> {
    let raw = n.to_bytes_be();
    assert!(raw.len() <= width, "value wider than {width} bytes");
    let mut out = vec![0u8; width - raw.len()];
    out.extend_from_slice(&raw);
    out
}

/// The "broken" evidence encoding: `n` big-endian in `(bits(n)+8)/8` bytes. This is one byte
/// wider than the minimal encoding whenever `bits(n)` is a multiple of 8, matching OpenSSL's
/// `BN_num_bytes(bn) + 1` behaviour that Blizzard's client depends on.
fn broken(n: &BigUint) -> Vec<u8> {
    let width = ((n.bits() as usize) + 8) >> 3;
    to_fixed_be(n, width)
}

/// SHA-256 of the concatenated big-endian encodings, as a big-endian [`BigUint`].
fn sha256_to_biguint(parts: &[&[u8]]) -> BigUint {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    BigUint::from_bytes_be(&h.finalize())
}

/// Evidence hash over "broken"-encoded numbers.
fn evidence(parts: &[&BigUint]) -> BigUint {
    let encoded: Vec<Vec<u8>> = parts.iter().map(|n| broken(n)).collect();
    let refs: Vec<&[u8]> = encoded.iter().map(|v| v.as_slice()).collect();
    sha256_to_biguint(&refs)
}

/// `k = H(N ‖ g)`, each fixed-width big-endian at [`N_BYTES`].
fn compute_k(n: &BigUint, g: &BigUint) -> BigUint {
    sha256_to_biguint(&[&to_fixed_be(n, N_BYTES), &to_fixed_be(g, N_BYTES)])
}

/// `u = H(A ‖ B)`, each fixed-width big-endian at [`N_BYTES`].
fn compute_u(a: &BigUint, b: &BigUint) -> BigUint {
    sha256_to_biguint(&[&to_fixed_be(a, N_BYTES), &to_fixed_be(b, N_BYTES)])
}

/// The SRP username fed to the KDF: `HEX(SHA256(UPPER(login)))`.
pub fn srp_username(login: &str) -> String {
    hex::encode_upper(Sha256::digest(login.to_uppercase().as_bytes()))
}

/// Derive the private exponent `x` from the SRP username, password and salt.
///
/// `x = PBKDF2-HMAC-SHA512(username:password, salt, 15000)` interpreted big-endian, minus
/// `2^512` when its top bit is set (so it is treated as signed), reduced mod `N-1`.
fn compute_x(srp_username: &str, password: &str, salt: &[u8], n: &BigUint) -> BigUint {
    let input = format!("{srp_username}:{password}");
    let mut out = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha512>(input.as_bytes(), salt, X_ITERATIONS, &mut out);

    let mut x = BigInt::from_bytes_be(Sign::Plus, &out);
    if out[0] & 0x80 != 0 {
        x -= BigInt::from(1u8) << 512;
    }

    let n_minus_1 = BigInt::from_biguint(Sign::Plus, n - 1u32);
    // Euclidean remainder: non-negative even when x went negative from the MSB fix.
    let reduced = ((&x % &n_minus_1) + &n_minus_1) % &n_minus_1;
    reduced
        .to_biguint()
        .expect("euclidean remainder is non-negative")
}

/// Salt and verifier stored for an account, both big-endian bytes.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub salt: [u8; SALT_LENGTH],
    pub verifier: Vec<u8>,
}

/// Compute the salt and verifier to store for a new account. Generates a random salt.
pub fn register(login: &str, password: &str) -> Credentials {
    use rand::RngCore;
    let mut salt = [0u8; SALT_LENGTH];
    rand::thread_rng().fill_bytes(&mut salt);
    register_with_salt(login, password, salt)
}

/// [`register`] with a caller-supplied salt (deterministic; for tests and migrations).
pub fn register_with_salt(login: &str, password: &str, salt: [u8; SALT_LENGTH]) -> Credentials {
    let n = prime_n();
    let g = generator();
    let x = compute_x(&srp_username(login), password, &salt, &n);
    let verifier = g.modpow(&x, &n).to_bytes_be();
    Credentials { salt, verifier }
}

/// Verify a plaintext password against an existing SRP verifier. Legacy Arctium web login uses
/// this path instead of the SRP challenge/evidence exchange.
pub fn verify_password(
    login: &str,
    password: &str,
    salt: [u8; SALT_LENGTH],
    verifier: &[u8],
) -> bool {
    let n = prime_n();
    let x = compute_x(&srp_username(login), password, &salt, &n);
    generator().modpow(&x, &n) == BigUint::from_bytes_be(verifier)
}

/// The challenge fields sent to the client (all big-endian, uppercase hex).
#[derive(Debug, Clone)]
pub struct Challenge {
    pub version: u32,
    pub iterations: u32,
    pub modulus: String,
    pub generator: String,
    pub hash_function: String,
    pub username: String,
    pub salt: String,
    pub public_b: String,
}

/// A successful verification: the session key and the server evidence to return to the client.
#[derive(Debug, Clone)]
pub struct Verified {
    /// Raw premaster secret `S`, big-endian — the session key handed to the world server.
    pub session_key: Vec<u8>,
    /// Server evidence `M2`, uppercase hex, for the client to check.
    pub server_m2: String,
}

/// Server side of one SRP6v2 exchange. Holds the ephemeral `b`/`B` for a single challenge and
/// must be used for exactly one verification attempt.
pub struct SrpServer {
    n: BigUint,
    g: BigUint,
    verifier: BigUint,
    salt: [u8; SALT_LENGTH],
    srp_username: String,
    b_priv: BigUint,
    b_pub: BigUint,
}

impl SrpServer {
    /// Begin an exchange for an account with the given SRP username, salt and stored verifier.
    pub fn new(srp_username: String, salt: [u8; SALT_LENGTH], verifier: &[u8]) -> Self {
        let n = prime_n();
        let g = generator();
        let k = compute_k(&n, &g);
        let verifier = BigUint::from_bytes_be(verifier);

        let b_priv = random_private_b(&n);
        // B = (g^b + k*v) mod N
        let b_pub = (g.modpow(&b_priv, &n) + (&k * &verifier)) % &n;

        Self {
            n,
            g,
            verifier,
            salt,
            srp_username,
            b_priv,
            b_pub,
        }
    }

    /// The challenge to send to the client.
    pub fn challenge(&self) -> Challenge {
        Challenge {
            version: SRP_VERSION,
            iterations: X_ITERATIONS,
            modulus: hex::encode_upper(self.n.to_bytes_be()),
            generator: hex::encode_upper(self.g.to_bytes_be()),
            hash_function: HASH_FUNCTION.to_string(),
            username: self.srp_username.clone(),
            salt: hex::encode_upper(self.salt),
            public_b: hex::encode_upper(self.b_pub.to_bytes_be()),
        }
    }

    /// Verify the client's public key `A` and evidence `M1` (both big-endian hex). Returns the
    /// session key and server evidence `M2` on success, or `None` on any failure.
    pub fn verify(&self, public_a_hex: &str, client_m1_hex: &str) -> Option<Verified> {
        let a = parse_hex_biguint(public_a_hex)?;
        // Reject A ≡ 0 (mod N): a malicious or broken client, and a trivially forgeable session.
        if (&a % &self.n).is_zero() {
            return None;
        }

        let u = compute_u(&a, &self.b_pub);
        if (&u % &self.n).is_zero() {
            return None;
        }

        // S = (A * v^u)^b mod N
        let s = (&a * self.verifier.modpow(&u, &self.n)).modpow(&self.b_priv, &self.n);

        let our_m1 = evidence(&[&a, &self.b_pub, &s]);
        let client_m1 = parse_hex_biguint(client_m1_hex)?;
        if our_m1 != client_m1 {
            return None;
        }

        let m2 = evidence(&[&a, &client_m1, &s]);
        Some(Verified {
            session_key: s.to_bytes_be(),
            server_m2: hex::encode_upper(m2.to_bytes_be()),
        })
    }
}

/// The client's response to a challenge: what it POSTs plus what it expects back.
#[derive(Debug, Clone)]
pub struct ClientProof {
    /// Client public key `A`, uppercase hex — sent as `public_A`.
    pub public_a: String,
    /// Client evidence `M1`, uppercase hex — sent as `client_evidence_M1`.
    pub client_m1: String,
    /// The `M2` the server should return if authentication succeeds.
    pub expected_m2: String,
    /// The session key `S` both sides derive, big-endian.
    pub session_key: Vec<u8>,
}

/// Client side of the exchange. This is the peer of [`SrpServer`] implementing the same spec —
/// used by tests and by interop/diagnostic tooling to drive a real login without a WoW client.
pub struct SrpClient {
    n: BigUint,
    g: BigUint,
    k: BigUint,
}

impl Default for SrpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SrpClient {
    pub fn new() -> Self {
        let n = prime_n();
        let g = generator();
        let k = compute_k(&n, &g);
        Self { n, g, k }
    }

    /// Given a `password` and a received [`Challenge`], produce the values to send and the
    /// `M2`/session key to expect. The challenge already carries the SRP username, so the
    /// password is the only secret needed here.
    pub fn prove(&self, password: &str, challenge: &Challenge) -> ClientProof {
        let salt = hex::decode(&challenge.salt).expect("challenge salt is hex");
        let b_pub = parse_hex_biguint(&challenge.public_b).expect("challenge B is hex");

        let x = compute_x(&challenge.username, password, &salt, &self.n);

        // Ephemeral a, A = g^a mod N.
        let a_priv = {
            use rand::RngCore;
            let mut buf = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut buf);
            BigUint::from_bytes_be(&buf) % (&self.n - 1u32)
        };
        let a_pub = self.g.modpow(&a_priv, &self.n);

        let u = compute_u(&a_pub, &b_pub);

        // S = (B - k*g^x)^(a + u*x) mod N, subtraction done signed then reduced.
        let g_x = self.g.modpow(&x, &self.n);
        let base = {
            let b = BigInt::from_biguint(Sign::Plus, b_pub.clone());
            let kv = BigInt::from_biguint(Sign::Plus, (&self.k * &g_x) % &self.n);
            let n = BigInt::from_biguint(Sign::Plus, self.n.clone());
            (((b - kv) % &n) + &n) % &n
        }
        .to_biguint()
        .expect("reduced base is non-negative");
        let exp = &a_priv + &u * &x;
        let s = base.modpow(&exp, &self.n);

        let m1 = evidence(&[&a_pub, &b_pub, &s]);
        let m2 = evidence(&[&a_pub, &m1, &s]);

        ClientProof {
            public_a: hex::encode_upper(a_pub.to_bytes_be()),
            client_m1: hex::encode_upper(m1.to_bytes_be()),
            expected_m2: hex::encode_upper(m2.to_bytes_be()),
            session_key: s.to_bytes_be(),
        }
    }
}

/// A random private exponent `b`: `N.bits()` random bits reduced mod `N-1`.
fn random_private_b(n: &BigUint) -> BigUint {
    use rand::RngCore;
    let byte_len = ((n.bits() as usize) + 7) >> 3;
    let mut buf = vec![0u8; byte_len];
    rand::thread_rng().fill_bytes(&mut buf);
    BigUint::from_bytes_be(&buf) % (n - 1u32)
}

fn parse_hex_biguint(s: &str) -> Option<BigUint> {
    let bytes = hex::decode(s.trim()).ok()?;
    Some(BigUint::from_bytes_be(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n_is_the_rfc5054_2048_bit_prime() {
        let n = prime_n();
        assert_eq!(n.to_bytes_be().len(), N_BYTES);
        assert_eq!(n.bits(), 2048);
    }

    #[test]
    fn srp_username_is_uppercase_sha256_hex_of_uppercased_login() {
        // Case-insensitive in the login: two casings hash identically.
        assert_eq!(
            srp_username("Player@Example.com"),
            srp_username("PLAYER@EXAMPLE.COM")
        );
        let u = srp_username("player");
        assert_eq!(u.len(), 64);
        assert!(u
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
    }

    #[test]
    fn broken_adds_a_leading_zero_on_exact_byte_boundaries() {
        // 0x80 is 8 bits → (8+8)/8 = 2 bytes → one extra leading zero.
        assert_eq!(broken(&BigUint::from(0x80u32)), vec![0x00, 0x80]);
        // 0xFF is 8 bits → 2 bytes.
        assert_eq!(broken(&BigUint::from(0xFFu32)), vec![0x00, 0xFF]);
        // 0x0100 is 9 bits → (9+8)/8 = 2 bytes → no extra byte.
        assert_eq!(broken(&BigUint::from(0x0100u32)), vec![0x01, 0x00]);
        // zero → 1 byte.
        assert_eq!(broken(&BigUint::zero()), vec![0x00]);
    }

    #[test]
    fn full_exchange_round_trips_and_both_sides_agree() {
        let creds = register_with_salt("player@example.com", "hunter2", [7u8; SALT_LENGTH]);
        let server = SrpServer::new(
            srp_username("player@example.com"),
            creds.salt,
            &creds.verifier,
        );
        let challenge = server.challenge();

        let proof = SrpClient::new().prove("hunter2", &challenge);

        let verified = server
            .verify(&proof.public_a, &proof.client_m1)
            .expect("correct password verifies");
        assert_eq!(
            verified.server_m2, proof.expected_m2,
            "M2 must match the client's"
        );
        assert_eq!(
            verified.session_key, proof.session_key,
            "both sides derive the same S"
        );
    }

    #[test]
    fn wrong_password_is_rejected() {
        let creds = register_with_salt("player@example.com", "hunter2", [7u8; SALT_LENGTH]);
        let server = SrpServer::new(
            srp_username("player@example.com"),
            creds.salt,
            &creds.verifier,
        );
        let challenge = server.challenge();

        let proof = SrpClient::new().prove("wrong-password", &challenge);

        assert!(server.verify(&proof.public_a, &proof.client_m1).is_none());
    }

    #[test]
    fn plaintext_password_verifies_against_the_stored_verifier() {
        let salt = [0x5Au8; SALT_LENGTH];
        let credentials = register_with_salt("player@example.test", "hunter2", salt);

        assert!(verify_password(
            "player@example.test",
            "hunter2",
            credentials.salt,
            &credentials.verifier
        ));
        assert!(!verify_password(
            "player@example.test",
            "wrong-password",
            credentials.salt,
            &credentials.verifier
        ));
    }

    #[test]
    fn challenge_advertises_the_expected_parameters() {
        let creds = register_with_salt("a@b.c", "pw", [1u8; SALT_LENGTH]);
        let server = SrpServer::new(srp_username("a@b.c"), creds.salt, &creds.verifier);
        let c = server.challenge();

        assert_eq!(c.version, 2);
        assert_eq!(c.iterations, 15_000);
        assert_eq!(c.hash_function, "SHA256");
        assert_eq!(c.generator, "02");
        assert_eq!(c.modulus.len(), N_BYTES * 2);
        assert_eq!(c.salt, hex::encode_upper([1u8; SALT_LENGTH]));
        assert_eq!(c.username, srp_username("a@b.c"));
    }

    #[test]
    fn a_equal_to_n_is_rejected() {
        let creds = register_with_salt("a@b.c", "pw", [1u8; SALT_LENGTH]);
        let server = SrpServer::new(srp_username("a@b.c"), creds.salt, &creds.verifier);
        // A = N ≡ 0 (mod N) must be refused regardless of M1.
        let a_hex = hex::encode_upper(prime_n().to_bytes_be());
        assert!(server.verify(&a_hex, "00").is_none());
    }
}
