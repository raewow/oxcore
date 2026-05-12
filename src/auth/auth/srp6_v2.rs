//! SRP6 v2 implementation wrapping the wow_srp crate
//! This is a drop-in replacement for the custom SRP6 implementation

use hex;
use rand::Rng;
use sha1::{Digest, Sha1};
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::{SrpProof, SrpServer, SrpVerifier};
use wow_srp::{
    GENERATOR, LARGE_SAFE_PRIME_LITTLE_ENDIAN, PASSWORD_VERIFIER_LENGTH, PROOF_LENGTH,
    PUBLIC_KEY_LENGTH, SALT_LENGTH,
};

/// SRP6 v2 implementation wrapping wow_srp
/// This provides the same interface as the original Srp6 but uses the proven wow_srp implementation
pub struct Srp6 {
    // Server state
    verifier: Option<SrpVerifier>,
    proof: Option<SrpProof>,
    server: Option<SrpServer>,
    server_proof: Option<[u8; PROOF_LENGTH as usize]>,

    // Salt and verifier as hex strings (for compatibility)
    salt_hex: String,
    verifier_hex: String,

    // Username for proof calculation (needed for wow_srp)
    username: Option<String>,

    // Store client public key and proof for M2 calculation
    client_public_key_bytes: Option<[u8; PUBLIC_KEY_LENGTH as usize]>,
    client_proof_bytes: Option<[u8; PROOF_LENGTH as usize]>,
}

impl Srp6 {
    pub const BYTE_SIZE: usize = 32;

    /// Create a new SRP6 instance
    pub fn new() -> Self {
        Self {
            verifier: None,
            proof: None,
            server: None,
            server_proof: None,
            salt_hex: String::new(),
            verifier_hex: String::new(),
            username: None,
            client_public_key_bytes: None,
            client_proof_bytes: None,
        }
    }

    /// Set the salt from hex string
    pub fn set_salt(&mut self, salt_hex: &str) -> bool {
        let salt_hex = salt_hex.trim().to_uppercase();
        // Validate hex string
        if salt_hex.is_empty() || salt_hex.len() != 64 {
            return false;
        }
        // Validate it's valid hex
        if hex::decode(&salt_hex).is_err() {
            return false;
        }
        self.salt_hex = salt_hex;
        true
    }

    /// Set the verifier from hex string
    pub fn set_verifier(&mut self, verifier_hex: &str) -> bool {
        let verifier_hex = verifier_hex.trim().to_uppercase();
        // Validate hex string (v is typically 63-64 hex chars = 31-32 bytes)
        if verifier_hex.is_empty() || verifier_hex.len() < 62 || verifier_hex.len() > 64 {
            return false;
        }
        // Validate it's valid hex
        if hex::decode(&verifier_hex).is_err() {
            return false;
        }
        self.verifier_hex = verifier_hex;
        true
    }

    /// Calculate host public ephemeral (B)
    /// This creates the SrpProof which contains B
    /// Note: Username must be set before calling this (via set_username or calculate_proof)
    pub fn calculate_host_public_ephemeral(&mut self) {
        if self.verifier.is_none() {
            // Create verifier from stored salt and verifier hex
            // We need username to create verifier
            if self.username.is_none() {
                // Can't create verifier without username - this will be set later in calculate_proof
                // For now, we'll just store the values and create verifier when username is available
                return;
            }
            if let Some(verifier) = self.create_verifier_from_hex() {
                self.verifier = Some(verifier);
            } else {
                return;
            }
        }

        // Convert to proof (this generates B)
        if let Some(verifier) = &self.verifier {
            self.proof = Some(verifier.clone().into_proof());
        }
    }

    /// Calculate session key from client public ephemeral A
    /// Note: wow_srp combines this with verify_proof, but we separate for compatibility
    pub fn calculate_session_key(&mut self, a_bytes: &[u8]) -> bool {
        if a_bytes.len() != PUBLIC_KEY_LENGTH as usize {
            return false;
        }

        // Store client public key bytes for later use
        let mut a_arr = [0u8; PUBLIC_KEY_LENGTH as usize];
        a_arr.copy_from_slice(a_bytes);
        self.client_public_key_bytes = Some(a_arr);

        // Ensure we have a proof
        if self.proof.is_none() {
            self.calculate_host_public_ephemeral();
        }

        // Validate the public key
        match wow_srp::PublicKey::from_le_bytes(a_arr) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Hash the session key (K calculation)
    /// This is done automatically by wow_srp when we convert proof to server
    pub fn hash_session_key(&mut self) {
        // Session key hashing is done automatically by wow_srp
        // This is a no-op for compatibility
    }

    /// Calculate proof (M) for the given username
    pub fn calculate_proof(&mut self, username: &str) {
        let normalized_username = username.to_uppercase();
        self.username = Some(normalized_username.clone());

        // If we don't have a verifier yet, try to create it
        if self.verifier.is_none() && !self.salt_hex.is_empty() && !self.verifier_hex.is_empty() {
            if let Some(verifier) = self.create_verifier_from_hex() {
                self.verifier = Some(verifier);
            }
        }

        // If we have a verifier but no proof, create it
        if self.proof.is_none() && self.verifier.is_some() {
            if let Some(verifier) = &self.verifier {
                self.proof = Some(verifier.clone().into_proof());
            }
        }
    }

    /// Verify the client proof (M1) and calculate server proof (M2)
    pub fn verify_proof(&mut self, m1_bytes: &[u8]) -> bool {
        use tracing::debug;

        if m1_bytes.len() != PROOF_LENGTH as usize {
            debug!(
                "SRP6 v2: Invalid M1 length: {} (expected {})",
                m1_bytes.len(),
                PROOF_LENGTH
            );
            return false;
        }

        // Store client proof bytes
        let mut m1_arr = [0u8; PROOF_LENGTH as usize];
        m1_arr.copy_from_slice(m1_bytes);
        self.client_proof_bytes = Some(m1_arr);
        debug!("SRP6 v2: Client proof M1: {:02x?}", m1_arr);

        // We need both client public key and proof
        let a_bytes = match self.client_public_key_bytes {
            Some(a) => {
                debug!("SRP6 v2: Client public key A: {:02x?}", a);
                a
            }
            None => {
                debug!("SRP6 v2: No client public key A stored");
                return false;
            }
        };

        let proof = match &self.proof.take() {
            Some(p) => {
                debug!(
                    "SRP6 v2: Have proof, server public key B: {:02x?}",
                    p.server_public_key()
                );
                debug!("SRP6 v2: Proof salt: {:02x?}", p.salt());
                p.clone()
            }
            None => {
                debug!("SRP6 v2: No proof available");
                return false;
            }
        };

        // Parse client public key
        let client_public_key = match wow_srp::PublicKey::from_le_bytes(a_bytes) {
            Ok(k) => {
                debug!("SRP6 v2: Client public key parsed successfully");
                k
            }
            Err(e) => {
                debug!("SRP6 v2: Failed to parse client public key: {:?}", e);
                self.proof = Some(proof);
                return false;
            }
        };

        // Parse client proof - into_server expects [u8; PROOF_LENGTH]
        // Verify and convert to server
        // Note: into_server consumes proof, so we can't restore it on error
        match proof.into_server(client_public_key, m1_arr) {
            Ok((server, server_proof)) => {
                debug!(
                    "SRP6 v2: Proof verification successful! Server proof M2: {:02x?}",
                    server_proof
                );
                self.server = Some(server);
                self.server_proof = Some(server_proof);
                true
            }
            Err(e) => {
                debug!("SRP6 v2: Proof verification failed: {:?}", e);
                // Proof verification failed - can't restore proof as it's been consumed
                // A new proof would need to be created from the verifier for retry
                false
            }
        }
    }

    /// Get the server proof (M2) for finalization
    pub fn finalize(&self) -> [u8; 20] {
        // wow_srp provides M2 directly from into_server
        if let Some(server_proof) = &self.server_proof {
            *server_proof
        } else {
            // Calculate M2 = SHA1(A || M1 || K) if we have the data
            if let (Some(a_bytes), Some(m1_bytes), Some(server)) = (
                &self.client_public_key_bytes,
                &self.client_proof_bytes,
                &self.server,
            ) {
                let mut hasher = Sha1::new();
                hasher.update(a_bytes);
                hasher.update(m1_bytes);
                hasher.update(server.session_key());
                let hash = hasher.finalize();
                let mut result = [0u8; 20];
                result.copy_from_slice(&hash);
                result
            } else {
                [0u8; 20]
            }
        }
    }

    /// Get host public ephemeral (B)
    pub fn get_host_public_ephemeral(&self) -> Vec<u8> {
        if let Some(proof) = &self.proof {
            proof.server_public_key().to_vec()
        } else {
            vec![0u8; PUBLIC_KEY_LENGTH as usize]
        }
    }

    /// Get generator modulo (g)
    pub fn get_generator_modulo(&self) -> Vec<u8> {
        vec![GENERATOR]
    }

    /// Get prime (N)
    pub fn get_prime(&self) -> Vec<u8> {
        LARGE_SAFE_PRIME_LITTLE_ENDIAN.to_vec()
    }

    /// Get proof (M) - server proof (M2)
    pub fn get_proof(&self) -> Vec<u8> {
        if let Some(server_proof) = &self.server_proof {
            server_proof.to_vec()
        } else {
            // Return M2 if we can calculate it
            let m2 = self.finalize();
            m2.to_vec()
        }
    }

    /// Get salt
    pub fn get_salt(&self) -> Vec<u8> {
        if let Some(proof) = &self.proof {
            proof.salt().to_vec()
        } else if !self.salt_hex.is_empty() {
            // Parse from hex
            hex::decode(&self.salt_hex).unwrap_or_default()
        } else {
            vec![0u8; SALT_LENGTH as usize]
        }
    }

    /// Get strong session key as hex string
    pub fn get_strong_session_key(&self) -> String {
        if let Some(server) = &self.server {
            // C++ stores K as BigNumber, which reverses bytes in SetBinary()
            // So we need to reverse the session key bytes to match
            // C++: K.SetBinary(vK, 40) reverses bytes at line 60: t[i] = bytes[len - 1 - i]
            // Then AsHexStr() returns the BigNumber as hex
            let mut session_key_bytes = server.session_key().to_vec();
            session_key_bytes.reverse();
            hex::encode_upper(&session_key_bytes)
        } else {
            String::new()
        }
    }

    /// Calculate verifier from username:password hash
    /// Note: This is a compatibility method - wow_srp doesn't work with pre-hashed passwords
    /// We need to calculate the verifier manually using the same algorithm as wow_srp
    pub fn calculate_verifier_from_hash(
        &mut self,
        username_password_hash_hex: &str,
        salt_hex: &str,
    ) -> bool {
        use num_bigint::BigUint;

        // Parse salt
        let salt_bytes = match hex::decode(salt_hex.trim().to_uppercase()) {
            Ok(bytes) if bytes.len() == SALT_LENGTH as usize => {
                let mut salt_arr = [0u8; SALT_LENGTH as usize];
                salt_arr.copy_from_slice(&bytes);
                salt_arr
            }
            _ => return false,
        };

        // Parse username:password hash
        let user_pass_hash = match hex::decode(username_password_hash_hex.trim().to_uppercase()) {
            Ok(bytes) if bytes.len() == 20 => bytes,
            _ => return false,
        };

        // Calculate x = SHA1(salt || username:password_hash)
        let mut hasher = Sha1::new();
        hasher.update(&salt_bytes);
        hasher.update(&user_pass_hash);
        let x_hash = hasher.finalize();

        // Convert x to BigUint (little-endian)
        let x = BigUint::from_bytes_le(&x_hash);

        // Calculate v = g^x mod N
        let g = BigUint::from(7u32);
        let n = BigUint::parse_bytes(
            b"894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7",
            16,
        )
        .unwrap();

        let v = g.modpow(&x, &n);

        // Store as hex (pad to 32 bytes for consistency)
        let v_bytes = v.to_bytes_le();
        let mut v_padded = vec![0u8; 32];
        if v_bytes.len() <= 32 {
            v_padded[..v_bytes.len()].copy_from_slice(&v_bytes);
        } else {
            v_padded.copy_from_slice(&v_bytes[..32]);
        }

        self.verifier_hex = hex::encode(v_padded);
        self.salt_hex = hex::encode(salt_bytes);

        true
    }

    /// Calculate verifier (generates random salt)
    pub fn calculate_verifier(&mut self, username_password_hash_hex: &str) -> bool {
        // Generate random salt
        let mut salt = [0u8; SALT_LENGTH as usize];
        rand::thread_rng().fill(&mut salt);

        let salt_hex = hex::encode(salt);
        self.calculate_verifier_from_hash(username_password_hash_hex, &salt_hex)
    }

    /// Get salt as hex string
    pub fn get_salt_hex(&self) -> String {
        if !self.salt_hex.is_empty() {
            self.salt_hex.clone()
        } else if let Some(proof) = &self.proof {
            hex::encode(proof.salt())
        } else {
            String::new()
        }
    }

    /// Get verifier as hex string
    pub fn get_verifier_hex(&self) -> String {
        self.verifier_hex.clone()
    }

    /// Helper: Create verifier from stored hex values
    fn create_verifier_from_hex(&self) -> Option<SrpVerifier> {
        if self.salt_hex.is_empty() || self.verifier_hex.is_empty() {
            return None;
        }

        // We need username to create verifier
        let username_str = self.username.as_ref()?;
        let username = match NormalizedString::new(username_str) {
            Ok(u) => u,
            Err(_) => return None,
        };

        // Parse salt
        let salt_bytes = hex::decode(&self.salt_hex).ok()?;
        if salt_bytes.len() != SALT_LENGTH as usize {
            return None;
        }
        let mut salt_arr = [0u8; SALT_LENGTH as usize];
        salt_arr.copy_from_slice(&salt_bytes);

        // Parse verifier (may need padding)
        let verifier_bytes = hex::decode(&self.verifier_hex).ok()?;
        let mut verifier_arr = [0u8; PASSWORD_VERIFIER_LENGTH as usize];
        if verifier_bytes.len() <= PASSWORD_VERIFIER_LENGTH as usize {
            verifier_arr[..verifier_bytes.len()].copy_from_slice(&verifier_bytes);
        } else {
            verifier_arr.copy_from_slice(&verifier_bytes[..PASSWORD_VERIFIER_LENGTH as usize]);
        }

        Some(SrpVerifier::from_database_values(
            username,
            verifier_arr,
            salt_arr,
        ))
    }

    /// Set username (needed for creating verifier from database values)
    pub fn set_username(&mut self, username: &str) {
        self.username = Some(username.to_uppercase());
    }

    /// Set client public ephemeral (for testing)
    pub fn set_client_public_ephemeral(&mut self, a_bytes: &[u8]) -> Result<(), String> {
        if a_bytes.len() != PUBLIC_KEY_LENGTH as usize {
            return Err("Invalid A length".to_string());
        }

        // Store for later use
        let mut a_arr = [0u8; PUBLIC_KEY_LENGTH as usize];
        a_arr.copy_from_slice(a_bytes);
        self.client_public_key_bytes = Some(a_arr);
        Ok(())
    }
}

impl Default for Srp6 {
    fn default() -> Self {
        Self::new()
    }
}
