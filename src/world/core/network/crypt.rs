//! AuthCrypt - packet header encryption/decryption
//!
//! Encrypts first 4 bytes of outgoing packets and decrypts first 6 bytes of incoming packets.
//! Based on the MaNGOS AuthCrypt implementation.

/// Authentication cryptography handler for WorldSocket
///
/// This implements the RC4-like stream cipher used for packet header
/// encryption in World of Warcraft 1.12.x.
pub struct AuthCrypt {
    /// Encryption key derived from session key
    key: Vec<u8>,
    /// Send key index
    send_i: u8,
    /// Send key state
    send_j: u8,
    /// Receive key index
    recv_i: u8,
    /// Receive key state
    recv_j: u8,
    /// Whether encryption is initialized
    initialized: bool,
}

impl AuthCrypt {
    /// Size of encrypted header for outgoing packets (server to client)
    /// [2 bytes size] + [2 bytes opcode]
    pub const CRYPTED_SEND_LEN: usize = 4;

    /// Size of encrypted header for incoming packets (client to server)
    /// [2 bytes size] + [4 bytes opcode]
    pub const CRYPTED_RECV_LEN: usize = 6;

    /// Create a new uninitialized AuthCrypt instance
    pub fn new() -> Self {
        Self {
            key: Vec::new(),
            send_i: 0,
            send_j: 0,
            recv_i: 0,
            recv_j: 0,
            initialized: false,
        }
    }

    /// Create a new AuthCrypt with the given key
    pub fn with_key(key: &[u8]) -> Self {
        let mut crypt = Self::new();
        crypt.set_key_from_slice(key);
        crypt.init();
        crypt
    }

    /// Initialize the crypt (reset state counters)
    pub fn init(&mut self) {
        self.send_i = 0;
        self.send_j = 0;
        self.recv_i = 0;
        self.recv_j = 0;
        self.initialized = true;
    }

    /// Set the encryption key
    pub fn set_key(&mut self, key: Vec<u8>) {
        self.key = key;
        if self.key.is_empty() {
            self.key.resize(1, 0); // Ensure at least 1 byte
        }
    }

    /// Set the encryption key from a slice
    pub fn set_key_from_slice(&mut self, key: &[u8]) {
        self.key = key.to_vec();
        if self.key.is_empty() {
            self.key.resize(1, 0);
        }
    }

    /// Check if the crypt is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Decrypt incoming packet header (first 6 bytes)
    ///
    /// The client encrypts the header with its send key, which uses the same
    /// key material but in the opposite direction. We decrypt it here.
    pub fn decrypt_recv(&mut self, data: &mut [u8]) {
        if !self.initialized {
            return;
        }
        if data.len() < Self::CRYPTED_RECV_LEN {
            return;
        }

        for t in 0..Self::CRYPTED_RECV_LEN {
            self.recv_i %= self.key.len() as u8;
            let x = (data[t].wrapping_sub(self.recv_j)) ^ self.key[self.recv_i as usize];
            self.recv_i = self.recv_i.wrapping_add(1);
            self.recv_j = data[t];
            data[t] = x;
        }
    }

    /// Encrypt outgoing packet header (first 4 bytes)
    ///
    /// Server sends 4-byte headers: [2 bytes size BE] [2 bytes opcode LE]
    pub fn encrypt_send(&mut self, data: &mut [u8]) {
        if !self.initialized {
            return;
        }
        if data.len() < Self::CRYPTED_SEND_LEN {
            return;
        }

        for t in 0..Self::CRYPTED_SEND_LEN {
            self.send_i %= self.key.len() as u8;
            let x = (data[t] ^ self.key[self.send_i as usize]).wrapping_add(self.send_j);
            self.send_i = self.send_i.wrapping_add(1);
            data[t] = x;
            self.send_j = x;
        }
    }
}

impl Default for AuthCrypt {
    fn default() -> Self {
        Self::new()
    }
}
