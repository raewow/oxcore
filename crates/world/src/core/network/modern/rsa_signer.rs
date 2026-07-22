//! The production [`EnterEncryptedModeSigner`]: RSA-PKCS1-v1.5 over SHA-256, byte-reversed.
//!
//! `SMSG_ENTER_ENCRYPTED_MODE` (and later `SMSG_CONNECT_TO`) carry an RSA signature the client
//! verifies against a modulus **baked into the executable** — the `CONNECT_TO_MODULUS` the patcher
//! replaces. The server signs the pre-computed 32-byte hash and then **reverses the signature
//! bytes** (the client's bignum representation is little-endian). Transcribed from HermesProxy's
//! `ConnectTo`/`EnterEncryptedMode` (`RsaCrypt.RSA.SignHash(hash, SHA256, Pkcs1).Reverse()`).
//!
//! The private key here and the modulus the patcher embeds must come from the same keypair
//! (produced together by `bnet gen-certs`), or the client rejects encrypted mode.
//!
//! **Unverified against a live client**: the reversal and the modulus byte order are faithful to
//! the reference but confirmed only by a real client.

use anyhow::{Context, Result};
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::Pkcs1v15Sign;
use rsa::sha2::Sha256;
use rsa::RsaPrivateKey;

use super::packets::EnterEncryptedModeSigner;

/// Signs the encrypted-mode / connect-to hashes with the server's RSA key.
pub struct RsaSigner {
    key: RsaPrivateKey,
}

impl RsaSigner {
    /// Load the signing key from a PKCS#1 PEM (`-----BEGIN RSA PRIVATE KEY-----`), as produced by
    /// `bnet gen-certs`.
    pub fn from_pkcs1_pem(pem: &str) -> Result<Self> {
        let key = RsaPrivateKey::from_pkcs1_pem(pem)
            .context("failed to parse world RSA signing key (PKCS#1 PEM)")?;
        Ok(Self { key })
    }

    /// Wrap an already-loaded key.
    pub fn new(key: RsaPrivateKey) -> Self {
        Self { key }
    }
}

impl EnterEncryptedModeSigner for RsaSigner {
    fn sign(&self, hash: &[u8; 32]) -> Vec<u8> {
        // PKCS#1 v1.5 signature over the pre-hashed 32 bytes (SHA-256 DigestInfo), then reversed.
        let mut signature = self
            .key
            .sign(Pkcs1v15Sign::new::<Sha256>(), hash)
            .expect("RSA signing over a fixed-size digest cannot fail");
        signature.reverse();
        signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1v15::Pkcs1v15Sign as VerifyScheme;
    use rsa::RsaPublicKey;

    #[test]
    fn signs_reversed_and_verifies_after_reversing_back() {
        let mut rng = rand::thread_rng();
        // 2048-bit so the signature is the 256 bytes the client's modulus slot expects.
        let key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public = RsaPublicKey::from(&key);
        let signer = RsaSigner::new(key);

        let hash = [0x5Au8; 32];
        let signature = signer.sign(&hash);
        assert_eq!(signature.len(), 256, "2048-bit signature is 256 bytes");

        // Reversing the wire bytes back to big-endian must verify against the public key.
        let mut big_endian = signature.clone();
        big_endian.reverse();
        public
            .verify(VerifyScheme::new::<Sha256>(), &hash, &big_endian)
            .expect("signature must verify once un-reversed");

        // The reversal is real: the as-sent bytes do NOT verify directly.
        assert!(public
            .verify(VerifyScheme::new::<Sha256>(), &hash, &signature)
            .is_err());
    }

    #[test]
    fn round_trips_through_a_pkcs1_pem() {
        use rsa::pkcs1::EncodeRsaPrivateKey;
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pem = key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).unwrap().to_string();

        let signer = RsaSigner::from_pkcs1_pem(&pem).unwrap();
        let sig = signer.sign(&[1u8; 32]);
        assert_eq!(sig.len(), 256);
    }
}
