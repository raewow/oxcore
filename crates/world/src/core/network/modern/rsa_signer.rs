//! The production [`EnterEncryptedModeSigner`]: RSA-PKCS1-v1.5 over SHA-256, byte-reversed.
//!
//! `SMSG_ENTER_ENCRYPTED_MODE` (and later `SMSG_CONNECT_TO`) carry an RSA signature the client
//! verifies against a modulus **baked into the executable**. The server signs the pre-computed
//! 32-byte hash and then **reverses the signature
//! bytes** (the client's bignum representation is little-endian). Transcribed from HermesProxy's
//! `ConnectTo`/`EnterEncryptedMode` (`RsaCrypt.RSA.SignHash(hash, SHA256, Pkcs1).Reverse()`).
//!
//! The private key must match the client patch's GameCrypt modulus, or the client rejects
//! encrypted mode.
//!
//! **Unverified against a live client**: the reversal and the modulus byte order are faithful to
//! the reference but confirmed only by a real client.

use anyhow::{Context, Result};
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::Pkcs1v15Sign;
use rsa::sha2::Sha256;
use rsa::{BigUint, RsaPrivateKey};

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

    /// Construct the public Arctium/HermesProxy legacy signing key used by their 1.14.x client
    /// patches. Its private parameters are published by HermesProxy's `RsaStore`.
    pub fn arctium() -> Result<Self> {
        const P: &str = "F3B359686F5AF3F3286FA1A06380552C7255392CF315D372300FB82DF49BB7380E376452672783D09A43A30C17B2CC395CEC9451CB63D9C2CB765302A437DDCE4E05FCF11A925A03256A5AB289F7966BABD3FE4EAB74FDDFE7E735497877750EB358DC275C8643F05FAD3C914DC128671F0CBBD989E22B6E5642AE2DE1B9BD7D";
        const Q: &str = "FABFF0401252EA40F240F8F593F48C0A55215A1C800F00E8774DE11D340773D065789CA35E6572FBFA5684DEDA10B86380B6DFA3F1A6DDA2898C2C52E2A066A942B002F1A8495BB1D41A3666371F17BB17F415C13A51531BE6CF542654A1A92C4F25CD83B1AC0357EB2A45969204E39E2B7A3FA7D219A0197DDEF1207131A0B";

        let p = BigUint::parse_bytes(P.as_bytes(), 16).expect("Arctium RSA prime P is valid");
        let q = BigUint::parse_bytes(Q.as_bytes(), 16).expect("Arctium RSA prime Q is valid");
        let key = RsaPrivateKey::from_p_q(p, q, BigUint::from(65_537u32))
            .context("failed to construct the Arctium world signing key")?;
        Ok(Self::new(key))
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
        let pem = key
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .unwrap()
            .to_string();

        let signer = RsaSigner::from_pkcs1_pem(&pem).unwrap();
        let sig = signer.sign(&[1u8; 32]);
        assert_eq!(sig.len(), 256);
    }

    #[test]
    fn arctium_key_has_the_patched_modulus() {
        use rsa::traits::PublicKeyParts;

        let signer = RsaSigner::arctium().unwrap();
        assert_eq!(
            hex::encode_upper(signer.key.n().to_bytes_be()),
            "EEB3DCD4D3C3B45451CE665BCB32B8F0F79253C619F20C852F8A26A97A459F60C4EBCDEA7F8D59D857B2607B094C9B68B8C7EDEF1E800DE66B375B5390EB18130D7F436483DA98E6ACC230A282A5C6CBC7FB869F9FA9026A0349C538FBC0C855CCC0CE2591BE85CFD1D137CECC83D2EA3080077B809F9D44542229BE86DADB48C5A9F91336952376F10EDC840D940212A897F33B14EEAA6F9805274E1FA360A5A9DAD817DF33CBE213548B18B0CAB9BB886406DF75A6D76100BBB05A0E7AD477084D15E21083B004AA9E8B77A906895D085D0FB82E6BC1CB64CF6E5CDB4F58650851FB0D481A6FB63D1F0BDDFE1B1DF0BFB0276BF58EBCC74001FFA70B80D65F"
        );
    }
}
