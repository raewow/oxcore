//! Authentication handler for CMSG_AUTH_SESSION
//!
//! Implements full SRP6-based authentication with comprehensive security checks.

use anyhow::anyhow;
use bytes::Buf;
use sha1::{Digest, Sha1};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::shared::database::{AccountRepository, Databases};
use crate::shared::messages::login::AuthErrorCode;
use crate::shared::protocol::WorldPacket;
use crate::world::core::session::{SessionManager, WorldSession};
use crate::world::World;

/// Result of authentication attempt
pub enum AuthResult {
    Success {
        account_id: u32,
        account_name: String,
        security: u8,
        session_key: Vec<u8>,
    },
    Error {
        error_code: u8,
    },
}

/// Handle CMSG_AUTH_SESSION with full security checks
pub async fn handle_auth_session(
    remote_addr: std::net::SocketAddr,
    mut packet: WorldPacket,
    server_seed: u32,
    databases: &Databases,
    session_mgr: &SessionManager,
) -> anyhow::Result<AuthResult> {
    debug!("Auth session received from {}", remote_addr);

    // ========== 1. PARSE CMSG_AUTH_SESSION ==========
    // Format: build (u32), server_id (u32), account (string), client_seed (u32), digest (20 bytes)
    let client_build = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read client build"))?;

    let _server_id = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read server ID"))?;

    let account_name = packet
        .read_string()
        .ok_or_else(|| anyhow!("Failed to read account name"))?;

    let client_seed = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read client seed"))?;

    if packet.data().remaining() < 20 {
        return Ok(AuthResult::Error {
            error_code: AuthErrorCode::Failed as u8,
        });
    }

    let mut client_digest = [0u8; 20];
    packet.data_mut().copy_to_slice(&mut client_digest);

    debug!(
        "[AUTH] Parsed: build={}, account={}, client_seed={:08x}",
        client_build, account_name, client_seed
    );

    // ========== 2. LOOKUP ACCOUNT IN DATABASE ==========
    let auth_repo = AccountRepository::new(Arc::new(databases.auth.clone()));

    let account_info = match auth_repo.find_for_world_auth(&account_name).await? {
        Some(info) => info,
        None => {
            warn!("Account '{}' not found in database", account_name);
            return Ok(AuthResult::Error {
                error_code: AuthErrorCode::UnknownAccount as u8,
            });
        }
    };

    // ========== 3. CHECK IF ACCOUNT IS LOCKED ==========
    if account_info.locked != 0 {
        warn!("Account '{}' is locked", account_name);
        return Ok(AuthResult::Error {
            error_code: AuthErrorCode::AccountBanned as u8,
        });
    }

    // ========== 4. CHECK FOR ACTIVE BANS ==========
    if let Some(ban_info) = auth_repo.is_account_banned(account_info.id).await? {
        warn!(
            "Account '{}' has active ban: {}",
            account_name, ban_info.banreason
        );
        return Ok(AuthResult::Error {
            error_code: AuthErrorCode::AccountBanned as u8,
        });
    }

    // ========== 5. VERIFY SESSION KEY EXISTS ==========
    let session_key_hex = match account_info.sessionkey {
        Some(key) if !key.is_empty() => key,
        _ => {
            warn!(
                "Account '{}' has no session key - must auth with realmd first",
                account_name
            );
            return Ok(AuthResult::Error {
                error_code: AuthErrorCode::Failed as u8,
            });
        }
    };

    // ========== 6. VALIDATE SESSION KEY FORMAT ==========
    let mut session_key = match hex::decode(&session_key_hex) {
        Ok(key) => key,
        Err(e) => {
            warn!(
                "Invalid session key hex for account '{}': {}",
                account_name, e
            );
            return Ok(AuthResult::Error {
                error_code: AuthErrorCode::Failed as u8,
            });
        }
    };

    if session_key.len() != 40 {
        warn!(
            "Invalid session key length for account '{}': {} (expected 40)",
            account_name,
            session_key.len()
        );
        return Ok(AuthResult::Error {
            error_code: AuthErrorCode::Failed as u8,
        });
    }

    // ========== 7. REVERSE SESSION KEY (big-endian for digest) ==========
    // Session key is stored reversed in DB (for BigNumber compatibility)
    // Digest calculation needs it unreversed (big-endian)
    session_key.reverse();

    // ========== 8. VERIFY SRP6 DIGEST ==========
    // SHA1(account + [0,0,0,0] + clientSeed + serverSeed + sessionKey)
    let mut hasher = Sha1::new();
    hasher.update(account_name.as_bytes());
    hasher.update(&[0u8; 4]);
    hasher.update(&client_seed.to_le_bytes());
    hasher.update(&server_seed.to_le_bytes());
    hasher.update(&session_key);
    let expected_digest: [u8; 20] = hasher.finalize().into();

    if expected_digest != client_digest {
        warn!("Digest verification failed for account '{}'", account_name);
        debug!(
            "Expected: {:02x?}, Got: {:02x?}",
            expected_digest, client_digest
        );

        // Increment failed login counter
        let _ = auth_repo
            .increment_failed_logins_by_username(&account_name)
            ;

        return Ok(AuthResult::Error {
            error_code: AuthErrorCode::Failed as u8,
        });
    }

    // ========== 9. CHECK IP MISMATCH (optional, log only) ==========
    if let Some(ref stored_ip) = account_info.last_ip {
        let remote_ip = remote_addr.ip().to_string();
        if stored_ip != &remote_ip {
            debug!(
                "IP mismatch for account '{}': stored={}, remote={}",
                account_name, stored_ip, remote_ip
            );
            // For development, allow and log. For production, may reject.
        }
    }

    // ========== 10. UPDATE LAST IP ==========
    let remote_ip = remote_addr.ip().to_string();
    let _ = auth_repo.update_last_ip(account_info.id, &remote_ip);

    // ========== 11. CHECK FOR DUPLICATE LOGIN ==========
    debug!(
        "[AUTH] Checking for duplicate session for account {}",
        account_info.id
    );
    if !session_mgr
        .remove_session_for_account(account_info.id).await

    {
        // Existing session is still loading - reject new connection
        warn!(
            "Rejecting login for account {} - existing session still loading",
            account_name
        );
        return Ok(AuthResult::Error {
            error_code: AuthErrorCode::AlreadyOnline as u8,
        });
    }

    // ========== 12. CHECK STALE ONLINE FLAG (server crash recovery) ==========
    // TODO: Implement when character repository has online flag checking
    // if char_repo.is_account_online(account_info.id).await? {
    //     info!("Clearing stale online flag for account {}", account_info.id);
    //     char_repo.set_account_online(account_info.id, false).await?;
    // }

    // ========== 13. RESET FAILED LOGINS ==========
    let _ = auth_repo.reset_failed_logins(account_info.id);

    // ========== 14. CLAMP SECURITY LEVEL ==========
    let security = if account_info.gmlevel > 7 {
        7
    } else {
        account_info.gmlevel
    };

    // ========== 15. RETURN SUCCESS ==========
    debug!(
        "Authentication successful for account '{}' (ID: {}, security: {})",
        account_name, account_info.id, security
    );

    Ok(AuthResult::Success {
        account_id: account_info.id,
        account_name,
        security,
        session_key,
    })
}
