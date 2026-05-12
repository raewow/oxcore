use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

/// Generate a TOTP (Time-based One-Time Password) code
/// Based on RFC 6238
pub fn generate_totp(secret: &str, time_step: u64) -> Result<u32> {
    // Decode base32 secret
    // base32 crate 0.4 uses base32::decode with alphabet
    let secret_bytes = base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)
        .ok_or_else(|| anyhow::anyhow!("Failed to decode base32 secret"))?;

    // Calculate time counter
    let counter = time_step / 30; // 30-second windows

    // Generate HMAC-SHA1
    let mut mac = HmacSha1::new_from_slice(&secret_bytes).context("Invalid secret key length")?;
    mac.update(&counter.to_be_bytes());
    let hmac_result = mac.finalize();
    let hmac_bytes = hmac_result.into_bytes();

    // Dynamic truncation (RFC 4226)
    let offset = (hmac_bytes[19] & 0x0F) as usize;
    let code = ((u32::from(hmac_bytes[offset]) & 0x7F) << 24)
        | ((u32::from(hmac_bytes[offset + 1]) & 0xFF) << 16)
        | ((u32::from(hmac_bytes[offset + 2]) & 0xFF) << 8)
        | (u32::from(hmac_bytes[offset + 3]) & 0xFF);

    // Return 6-digit code
    Ok(code % 1_000_000)
}

/// Generate TOTP for current time
pub fn generate_totp_now(secret: &str) -> Result<u32> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX epoch")?
        .as_secs();
    generate_totp(secret, now)
}

/// Generate TOTP with interval offset (for clock skew tolerance)
/// interval: offset in 30-second steps (-2 to +2 typically)
pub fn generate_totp_with_offset(secret: &str, interval: i32) -> Result<u32> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX epoch")?
        .as_secs();
    let time_step = (now / 30) as i64 + interval as i64;
    if time_step < 0 {
        return Err(anyhow::anyhow!("Invalid time step"));
    }
    generate_totp(secret, (time_step * 30) as u64)
}

/// Verify a TOTP code with tolerance for clock skew
/// Returns true if the code is valid within the tolerance window
pub fn verify_totp(secret: &str, code: u32, tolerance: u64) -> Result<bool> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX epoch")?
        .as_secs();

    // Check current time step and adjacent steps (for clock skew tolerance)
    let time_step = now / 30;

    for offset in 0..=tolerance {
        // Check previous steps
        if offset > 0 && time_step >= offset {
            if let Ok(prev_code) = generate_totp(secret, (time_step - offset) * 30) {
                if prev_code == code {
                    return Ok(true);
                }
            }
        }

        // Check current and future steps
        if let Ok(future_code) = generate_totp(secret, (time_step + offset) * 30) {
            if future_code == code {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Generate a random base32 secret for TOTP
pub fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
    base32::encode(base32::Alphabet::RFC4648 { padding: false }, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_generation() {
        let secret = "JBSWY3DPEHPK3PXP"; // Example secret
        let code = generate_totp_now(secret).unwrap();
        assert!(code < 1_000_000);
    }

    #[test]
    fn test_totp_verification() {
        let secret = "JBSWY3DPEHPK3PXP";
        let code = generate_totp_now(secret).unwrap();
        assert!(verify_totp(secret, code, 1).unwrap());
        assert!(!verify_totp(secret, 999999, 1).unwrap()); // Wrong code
    }

    #[test]
    fn test_secret_generation() {
        let secret = generate_secret();
        assert!(!secret.is_empty());
        // Base32 should only contain A-Z, 2-7
        assert!(secret
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }
}
