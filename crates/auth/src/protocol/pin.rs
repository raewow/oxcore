use anyhow::Result;
use sha1::{Digest, Sha1};

/// PIN data structure from client
#[derive(Debug, Clone)]
pub struct PinData {
    pub salt: [u8; 16],
    pub hash: [u8; 20],
}

/// Verify PIN entry data using grid remapping
/// This matches the C++ implementation in AuthSocket::VerifyPinData
pub fn verify_pin_data(
    pin: u32,
    client_data: &PinData,
    grid_seed: u32,
    server_security_salt: &[u8; 16],
) -> Result<bool> {
    // Remap the grid to match the client's layout
    let mut grid = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut remapped_grid = vec![0u8; 10];

    let mut seed = grid_seed;
    for i in (1..=10).rev() {
        let remainder = (seed % i as u32) as usize;
        seed /= i as u32;
        remapped_grid[10 - i] = grid[remainder];

        // Remove the selected element from grid
        if remainder + 1 < grid.len() {
            grid.copy_within(remainder + 1.., remainder);
        }
        grid.pop();
    }

    // Convert the PIN to bytes (e.g., 1234 -> [1, 2, 3, 4])
    let mut pin_bytes = Vec::new();
    let mut pin_value = pin;

    while pin_value != 0 {
        pin_bytes.push((pin_value % 10) as u8);
        pin_value /= 10;
    }

    pin_bytes.reverse();

    // Validate PIN length
    if pin_bytes.len() < 4 || pin_bytes.len() > 10 {
        return Ok(false); // PIN outside of expected range
    }

    // Remap the PIN to calculate the expected client input sequence
    for i in 0..pin_bytes.len() {
        let digit = pin_bytes[i];
        let index = remapped_grid.iter().position(|&x| x == digit).unwrap_or(0);
        pin_bytes[i] = index as u8;
    }

    // Convert PIN bytes to their ASCII values
    for i in 0..pin_bytes.len() {
        pin_bytes[i] += 0x30;
    }

    // Validate the PIN: x = H(client_salt | H(server_salt | ascii(pin_bytes)))
    let mut sha_first = Sha1::new();
    sha_first.update(server_security_salt);
    sha_first.update(&pin_bytes);
    let sha_first_hash = sha_first.finalize();

    let mut sha_second = Sha1::new();
    sha_second.update(&client_data.salt);
    sha_second.update(&sha_first_hash);
    let sha_second_hash = sha_second.finalize();

    // Compare with client hash
    Ok(&sha_second_hash[..] == client_data.hash)
}

/// Verify a PIN using SHA1 hash (simple version for storage)
/// The PIN is hashed and compared against the stored hash
pub fn verify_pin(pin: &str, stored_hash: &str) -> Result<bool> {
    // Hash the provided PIN
    let mut hasher = Sha1::new();
    hasher.update(pin.as_bytes());
    let pin_hash = hex::encode(hasher.finalize());

    // Compare with stored hash (case-insensitive)
    Ok(pin_hash.to_lowercase() == stored_hash.to_lowercase())
}

/// Hash a PIN for storage
pub fn hash_pin(pin: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(pin.as_bytes());
    hex::encode(hasher.finalize())
}

/// Check if a PIN is valid format (typically 4-10 digits)
pub fn is_valid_pin_format(pin: &str) -> bool {
    // PIN should be 4-10 digits
    pin.len() >= 4 && pin.len() <= 10 && pin.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_hashing() {
        let pin = "1234";
        let hash = hash_pin(pin);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 40); // SHA1 produces 40 hex chars
    }

    #[test]
    fn test_pin_verification() {
        let pin = "1234";
        let hash = hash_pin(pin);
        assert!(verify_pin(pin, &hash).unwrap());
        assert!(!verify_pin("5678", &hash).unwrap());
    }

    #[test]
    fn test_pin_format() {
        assert!(is_valid_pin_format("1234"));
        assert!(is_valid_pin_format("1234567890"));
        assert!(!is_valid_pin_format("123")); // Too short
        assert!(!is_valid_pin_format("12345678901")); // Too long
        assert!(!is_valid_pin_format("abcd")); // Not digits
    }
}
