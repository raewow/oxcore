use anyhow::{Context, Result};
use bytes::{Buf, BufMut, BytesMut};
use rand::Rng;
use sqlx::Row;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

use crate::auth::auth::packets::{
    AuthLogonChallengeC, AuthLogonChallengeS, AuthLogonProofC, AuthLogonProofS,
    AuthReconnectChallengeS, AuthReconnectProofC, AuthReconnectProofS,
};
use crate::auth::auth::srp6_v2::Srp6;
use crate::auth::auth::{pin::verify_pin_data, Geolock};
use crate::auth::common::codes::{AuthCmd, AuthResult, AuthStatus, LockFlag};
use crate::auth::config::Config;
use crate::auth::database::Database;
use crate::auth::metrics::Metrics;
use crate::auth::patch::PatchCache;
use crate::auth::realm::{AllowedBuilds, RealmList};
use sha1::{Digest, Sha1};
use std::path::PathBuf;

/// Version challenge salt sent to client for integrity check
/// This is used by the client as the HMAC key to hash game files
const VERSION_CHALLENGE: [u8; 16] = [
    0xBA, 0xA3, 0x1E, 0x99, 0xA0, 0x0B, 0x21, 0x57, 0xFC, 0x37, 0x3F, 0xB3, 0x69, 0xCD, 0xD2, 0xF1,
];

pub struct AuthSocket {
    stream: TcpStream,
    remote_addr: std::net::SocketAddr,
    buffer: BytesMut,
    status: AuthStatus,
    database: Database,
    realm_list: Arc<RealmList>,
    patch_cache: Arc<PatchCache>,
    allowed_builds: Arc<AllowedBuilds>,
    metrics: Arc<Metrics>,
    config: Config,
    geolock: Geolock,

    // Authentication state
    srp: Option<Srp6>,
    login: String,
    safelogin: String,
    build: u16,
    account_id: Option<u32>,
    localization_name: String,
    os: u32,
    platform: u32,

    // Account security levels
    account_default_security_level: u8,
    account_security_on_realm: std::collections::HashMap<u32, u8>,

    // Reconnect state
    reconnect_proof: Option<[u8; 16]>, // Random 16-byte challenge for reconnect

    // Security features
    prompt_pin: bool,
    lock_flags: u32,
    security_info: String,
    last_ip: Option<String>,
    email: Option<String>,
    geo_unlock_pin: Option<u32>,
    grid_seed: u32,
    server_security_salt: [u8; 16],

    // Patch handling
    patch_file: Option<PathBuf>,

    // Realm list request throttling
    last_realm_list_request: Option<u64>,
}

impl AuthSocket {
    /// Create a new authentication socket from an accepted connection
    pub fn new(
        stream: TcpStream,
        remote_addr: std::net::SocketAddr,
        database: Database,
        realm_list: Arc<RealmList>,
        patch_cache: Arc<PatchCache>,
        allowed_builds: Arc<AllowedBuilds>,
        config: Config,
        metrics: Arc<Metrics>,
    ) -> Self {
        let geolock = Geolock::new(config.geo_locking);
        Self {
            stream,
            remote_addr,
            buffer: BytesMut::with_capacity(4096),
            status: AuthStatus::Challenge,
            database,
            realm_list,
            patch_cache,
            allowed_builds,
            metrics,
            config,
            geolock,
            srp: None,
            login: String::new(),
            safelogin: String::new(),
            build: 0,
            account_id: None,
            localization_name: String::new(),
            os: 0,
            platform: 0,
            account_default_security_level: 0,
            account_security_on_realm: std::collections::HashMap::new(),
            reconnect_proof: None,
            prompt_pin: false,
            lock_flags: 0,
            security_info: String::new(),
            last_ip: None,
            email: None,
            geo_unlock_pin: None,
            grid_seed: 0,
            server_security_salt: [0u8; 16],
            patch_file: None,
            last_realm_list_request: None,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        info!("New connection from {}", self.remote_addr);

        loop {
            let mut read_buf = vec![0u8; 4096];
            match self.stream.read(&mut read_buf).await {
                Ok(0) => break,
                Ok(n) => {
                    self.buffer.extend_from_slice(&read_buf[..n]);
                }
                Err(e) => {
                    error!("Error reading from socket: {}", e);
                    return Err(e.into());
                }
            }
            while let Some(cmd) = self.read_command()? {
                if let Err(e) = self.handle_command(cmd).await {
                    error!("Error handling command {:?}: {}", cmd, e);
                    return Err(e);
                }
            }
        }

        info!(
            "Connection closed: {} (final status: {:?})",
            self.remote_addr, self.status
        );
        Ok(())
    }

    fn read_command(&mut self) -> Result<Option<AuthCmd>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        let cmd_byte = self.buffer[0];

        match AuthCmd::from_u8(cmd_byte) {
            Some(cmd) => Ok(Some(cmd)),
            None => {
                self.buffer.advance(1);
                Ok(None)
            }
        }
    }

    async fn handle_command(&mut self, cmd: AuthCmd) -> Result<()> {
        let allowed = match (self.status, cmd) {
            (AuthStatus::Challenge, AuthCmd::LogonChallenge) => true,
            (AuthStatus::Challenge, AuthCmd::ReconnectChallenge) => true,
            (AuthStatus::LogonProof, AuthCmd::LogonProof) => true,
            (AuthStatus::ReconProof, AuthCmd::ReconnectProof) => true,
            (AuthStatus::Authed, AuthCmd::RealmList) => true,
            (AuthStatus::Patch, AuthCmd::XferAccept) => true,
            (AuthStatus::Patch, AuthCmd::XferResume) => true,
            (AuthStatus::Patch, AuthCmd::XferCancel) => true,
            (AuthStatus::LogonProof, AuthCmd::LogonChallenge) => {
                warn!("Client sent LogonChallenge while in LogonProof status, resetting state");
                self.status = AuthStatus::Challenge;
                true
            }
            _ => false,
        };

        if !allowed {
            warn!("Command {:?} not allowed in status {:?}", cmd, self.status);
            if !self.buffer.is_empty() {
                self.buffer.advance(1);
            }
            return Ok(());
        }

        match cmd {
            AuthCmd::LogonChallenge => self.handle_logon_challenge().await,
            AuthCmd::LogonProof => self.handle_logon_proof().await,
            AuthCmd::ReconnectChallenge => self.handle_reconnect_challenge().await,
            AuthCmd::ReconnectProof => self.handle_reconnect_proof().await,
            AuthCmd::RealmList => self.handle_realm_list().await,
            AuthCmd::XferAccept => self.handle_xfer_accept().await,
            AuthCmd::XferResume => self.handle_xfer_resume().await,
            AuthCmd::XferCancel => self.handle_xfer_cancel().await,
            _ => {
                warn!("Unhandled command: {:?}", cmd);
                Ok(())
            }
        }
    }

    fn read_column<'r, T>(
        row: &'r sqlx::mysql::MySqlRow,
        index: usize,
        column_name: &str,
        account_name: &str,
    ) -> Result<T, sqlx::Error>
    where
        T: sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
    {
        row.try_get::<T, _>(index).map_err(|e| {
            error!(
                "Failed to read column {} (index {}) for account '{}': {:?}",
                column_name, index, account_name, e
            );
            e
        })
    }

    async fn handle_logon_challenge(&mut self) -> Result<()> {
        if self.buffer.len() < 4 {
            return Ok(());
        }

        let cmd = self.buffer[0];
        let _error = self.buffer[1];
        let size = u16::from_le_bytes([self.buffer[2], self.buffer[3]]) as usize;

        if self.buffer.len() < 4 + size {
            return Ok(());
        }

        self.buffer.advance(4);
        let packet_data = self.buffer.split_to(size);

        let mut challenge_buf = BytesMut::new();
        challenge_buf.put_u8(cmd);
        challenge_buf.put_u8(_error);
        challenge_buf.put_u16_le(size as u16);
        challenge_buf.extend_from_slice(&packet_data);

        let challenge = match AuthLogonChallengeC::read_from(&mut challenge_buf) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to parse logon challenge: {}", e);
                self.send_error_response(AuthResult::FailDbBusy).await?;
                return Ok(());
            }
        };

        self.login = challenge.username.clone();
        self.build = challenge.build;

        let mut os_bytes = challenge.os;
        if os_bytes.len() >= 3 {
            os_bytes[3] = 0;
            // Reverse first 3 bytes
            os_bytes.swap(0, 2);
            self.os = u32::from_le_bytes(os_bytes);
        }

        // Parse platform (3-byte string, reversed, stored as u32)
        let mut platform_bytes = challenge.platform;
        if platform_bytes.len() >= 3 {
            platform_bytes[3] = 0;
            // Reverse first 3 bytes
            platform_bytes.swap(0, 2);
            self.platform = u32::from_le_bytes(platform_bytes);
        }

        // Store localization name (country code, reversed)
        // Format: country[3], country[2], country[1], country[0]
        if challenge.country.len() >= 4 {
            self.localization_name = format!(
                "{}{}{}{}",
                challenge.country[3] as char,
                challenge.country[2] as char,
                challenge.country[1] as char,
                challenge.country[0] as char
            );
        }

        self.safelogin = self.login.to_uppercase().replace("'", "''");

        info!(
            "Logon challenge from {} (build: {}, locale: {})",
            self.login, self.build, self.localization_name
        );

        // Check IP ban
        let ip_str = self.remote_addr.ip().to_string();
        let ip_ban_result = self.database.ip_bans.is_ip_banned(&ip_str).await?;

        if ip_ban_result.is_some() {
            warn!("Banned IP {} tried to login", ip_str);
            self.send_error_response(AuthResult::FailDbBusy).await?;
            return Ok(());
        }

        // Get account from database with security fields
        let account_result = self
            .database
            .accounts
            .find_for_login(&self.safelogin)
            .await?;

        let mut response = AuthLogonChallengeS {
            cmd: AuthCmd::LogonChallenge as u8,
            error: 0,
            result: AuthResult::FailUnknownAccount as u8,
            b: [0; 32],
            g_len: 1,
            g: 7,
            n_len: 32,
            n: [0; 32],
            s: vec![],
            unk3: VERSION_CHALLENGE, // Version challenge - used by client as HMAC key for integrity check
            security_flags: None,
            grid_seed: None,
            server_security_salt: None,
        };

        if let Some(account) = account_result {
            self.account_id = Some(account.id);
            self.last_ip = account.last_ip.clone();
            self.email = account.email.clone();
            self.lock_flags = account.locked as u32;

            // Note: online check removed - world server handles session deduplication
            // by kicking existing sessions when a duplicate login is detected

            if let Some(pin_val) = account.geolock_pin {
                if pin_val > 0 {
                    self.geo_unlock_pin = Some(pin_val as u32);
                }
            }

            self.security_info = account.security.clone().unwrap_or_default();

            // Check account ban
            let account_ban_result = self.database.accounts.is_account_banned(account.id).await?;

            if account_ban_result.is_some() {
                warn!("Banned account {} tried to login", self.login);
                self.send_error_response(AuthResult::FailSuspended).await?;
                return Ok(());
            }

            // Check if account is locked
            if account.locked != 0 {
                warn!("Locked account {} tried to login", self.login);
                self.send_error_response(AuthResult::FailLockedEnforced)
                    .await?;
                return Ok(());
            }

            if self.config.req_email_verification {
                if let Some(join_ts) = account.joindate_ts {
                    let req_since = self.config.req_email_since as i64;
                    if join_ts >= req_since && !account.email_verif {
                        warn!("Account {} requires email verification", self.login);
                        self.send_error_response(AuthResult::FailFailNoaccess)
                            .await?;
                        return Ok(());
                    }
                }
            }

            let mut ip_locked = false;
            if LockFlag::has_flag(self.lock_flags, LockFlag::IpLock) {
                if self.last_ip.as_ref().map(|s| s.as_str()) != Some(&ip_str) {
                    if !LockFlag::has_flag(self.lock_flags, LockFlag::Totp)
                        && !LockFlag::has_flag(self.lock_flags, LockFlag::FixedPin)
                    {
                        response.result = AuthResult::FailSuspended as u8;
                        let mut response_buf = BytesMut::new();
                        response.write_to(&mut response_buf);
                        self.send_packet(&response_buf).await?;
                        return Ok(());
                    }
                    ip_locked = true;
                }
            }

            if !self
                .geolock
                .is_ip_allowed(account.last_ip.as_deref(), &self.remote_addr.ip())
            {
                warn!(
                    "Geolock violation for account {} from IP {}",
                    self.login, ip_str
                );
            }

            // Check if verifier and salt exist
            let (v, s) = match (&account.v, &account.s) {
                (Some(v), Some(s)) => (v, s),
                _ => {
                    error!(
                        "Account {} has NULL verifier (v) or salt (s) - password not set correctly",
                        self.login
                    );
                    response.result = AuthResult::FailFailNoaccess as u8;
                    let mut response_buf = BytesMut::new();
                    response.write_to(&mut response_buf);
                    self.send_packet(&response_buf).await?;
                    return Ok(());
                }
            };

            let mut srp = Srp6::new();
            srp.set_username(&self.login);
            if !srp.set_verifier(v) {
                error!(
                    "Failed to set verifier (v) for account {} - invalid hex or zero value",
                    self.login
                );
                response.result = AuthResult::FailFailNoaccess as u8;
            } else if !srp.set_salt(s) {
                error!(
                    "Failed to set salt (s) for account {} - invalid hex or zero value",
                    self.login
                );
                response.result = AuthResult::FailFailNoaccess as u8;
            } else {
                srp.calculate_host_public_ephemeral();

                response.result = AuthResult::Success as u8;
                let b_bytes = srp.get_host_public_ephemeral();
                response.b.copy_from_slice(&b_bytes[..32]);

                let n_bytes = srp.get_prime();
                response.n.copy_from_slice(&n_bytes[..32]);

                response.s = srp.get_salt();

                self.prompt_pin = ip_locked;

                if (!ip_locked && LockFlag::has_flag(self.lock_flags, LockFlag::AlwaysEnforce))
                    || self.geo_unlock_pin.is_some()
                {
                    self.prompt_pin = true;
                }

                if self.prompt_pin {
                    info!(
                        "Account '{}' using IP '{}' requires PIN authentication",
                        self.login, ip_str
                    );

                    let mut rng = rand::thread_rng();
                    self.grid_seed = rng.gen();
                    rng.fill(&mut self.server_security_salt);

                    response.security_flags = Some(1);
                    response.grid_seed = Some(self.grid_seed);
                    response.server_security_salt = Some(self.server_security_salt);
                } else {
                    if self.build >= 5428 {
                        response.security_flags = Some(0);
                    }
                }

                self.load_account_security_levels(account.id).await?;

                self.srp = Some(srp);
                self.status = AuthStatus::LogonProof;
            }
        } else {
            warn!("Account '{}' not found in database", self.login);
        }

        let mut response_buf = BytesMut::new();
        response.write_to(&mut response_buf);
        self.send_packet(&response_buf).await?;

        Ok(())
    }

    async fn handle_logon_proof(&mut self) -> Result<()> {
        if self.srp.is_none() {
            error!("LogonProof received but no SRP6 state");
            self.send_error_response(AuthResult::FailDbBusy).await?;
            return Ok(());
        }

        let min_size = if self.build >= 5428 { 74 } else { 73 };

        if self.buffer.len() < min_size {
            return Ok(());
        }

        let proof = match AuthLogonProofC::read_from(&mut self.buffer, self.build) {
            Ok(p) => p,
            Err(e) => {
                if e.contains("Not enough data") {
                    return Ok(());
                }
                error!("Failed to parse logon proof: {}", e);
                self.send_error_response(AuthResult::FailDbBusy).await?;
                return Ok(());
            }
        };

        let srp = self.srp.as_mut().unwrap();

        if !srp.calculate_session_key(&proof.a) {
            error!("Invalid client public ephemeral A");
            self.send_error_response(AuthResult::FailUnknownAccount)
                .await?;
            return Ok(());
        }

        srp.hash_session_key();
        srp.calculate_proof(&self.login);

        let valid_version = true;

        // If the client has no valid version, check for patch file
        if !valid_version {
            if self.patch_file.is_some() {
                // Already have a patch file, continue with transfer
            } else {
                // Check if we have the appropriate patch on disk
                // File format: {build}{localization}.mpq (e.g., 65535enGB.mpq)
                let patch_filename = format!("{}{}.mpq", self.build, self.localization_name);
                let patch_path = self.config.patches_dir.join(&patch_filename);

                if patch_path.exists() {
                    debug!("Found patch file: {:?}", patch_path);
                    self.patch_file = Some(patch_path.clone());

                    // Get file size
                    let metadata = match tokio::fs::metadata(&patch_path).await {
                        Ok(m) => m,
                        Err(e) => {
                            error!("Failed to get patch file metadata: {}", e);
                            self.send_error_response(AuthResult::FailVersionInvalid)
                                .await?;
                            return Ok(());
                        }
                    };
                    let file_size = metadata.len();

                    // Get MD5 hash from cache (or calculate it)
                    let md5_hash = if let Some(hash) = self
                        .patch_cache
                        .get_hash(&patch_path.to_string_lossy())
                        .await
                    {
                        hash
                    } else {
                        // Reload patch MD5 if not in cache
                        if let Err(e) = self.patch_cache.reload_patch(&patch_path).await {
                            warn!("Failed to reload patch MD5: {}", e);
                        }
                        self.patch_cache
                            .get_hash(&patch_path.to_string_lossy())
                            .await
                            .unwrap_or([0u8; 16])
                    };

                    // Send version update response
                    let mut version_response = BytesMut::new();
                    version_response.put_u8(AuthCmd::LogonProof as u8);
                    version_response.put_u8(AuthResult::FailVersionUpdate as u8);
                    self.send_packet(&version_response).await?;

                    // Send XFER_INIT packet
                    let xfer_init = crate::auth::auth::packets::XferInit::new(file_size, md5_hash);
                    let mut xfer_buf = BytesMut::new();
                    xfer_init.write_to(&mut xfer_buf);
                    self.send_packet(&xfer_buf).await?;

                    // Set status to patch transfer
                    self.status = AuthStatus::Patch;
                    return Ok(());
                } else {
                    debug!("Patch file not found: {:?}", patch_path);
                    self.send_error_response(AuthResult::FailVersionInvalid)
                        .await?;
                    return Ok(());
                }
            }
        }

        // Verify proof
        if !srp.verify_proof(&proof.m1) {
            error!("Proof verification failed for {}", self.login);

            // Handle wrong password attempts
            if self.config.wrong_pass.max_count > 0 {
                let _ = self
                    .database
                    .accounts
                    .increment_failed_logins_by_username(&self.safelogin)
                    .await;

                // Check if we should ban
                if let Some(failed_count) = self
                    .database
                    .accounts
                    .get_failed_logins_by_username(&self.safelogin)
                    .await?
                {
                    if failed_count >= self.config.wrong_pass.max_count {
                        warn!(
                            "Account {} exceeded wrong password limit, banning",
                            self.login
                        );
                        let ban_duration = self.config.wrong_pass.ban_time as i64;
                        // Ban the account or IP based on config
                        if self.config.wrong_pass.ban_type != 0 {
                            // Ban IP
                            let ip_str = self.remote_addr.ip().to_string();
                            let _ = self
                                .database
                                .ip_bans
                                .auto_ban_ip(&ip_str, ban_duration)
                                .await;
                            self.metrics.increment_ip_ban();
                        } else {
                            // Ban account
                            let _ = self
                                .database
                                .accounts
                                .auto_ban_account(self.account_id.unwrap_or(0), ban_duration)
                                .await;
                            self.metrics.increment_account_ban();
                        }
                    }
                }
            }

            self.send_error_response(AuthResult::FailIncorrectPassword)
                .await?;
            return Ok(());
        }

        let mut pin_result = true;

        if self.prompt_pin && proof.security_flags.is_none() {
            pin_result = false;
        }

        if self.prompt_pin && proof.security_flags.is_some() && proof.security_flags.unwrap() != 0 {
            if let Some(pin_data) = &proof.pin_data {
                let security_result = sqlx::query(
                    "SELECT `security`, `geolock_pin`, `totp_secret` FROM `account` WHERE `id` = ?",
                )
                .bind(self.account_id.unwrap_or(0))
                .fetch_optional(self.database.pool())
                .await?;

                if let Some(row) = security_result {
                    let _account_security: u8 = row.get(0);
                    let geolock_pin: Option<String> = row.get(1);
                    let totp_secret: Option<String> = row.get(2);

                    pin_result = false;

                    if LockFlag::has_flag(self.lock_flags, LockFlag::FixedPin) {
                        if let Ok(pin_val) = self.security_info.parse::<u32>() {
                            pin_result = verify_pin_data(
                                pin_val,
                                pin_data,
                                self.grid_seed,
                                &self.server_security_salt,
                            )?;
                        }
                    } else if LockFlag::has_flag(self.lock_flags, LockFlag::Totp) {
                        if let Some(secret) = totp_secret {
                            for i in -2..=2 {
                                if let Ok(totp_pin) =
                                    crate::auth::auth::totp::generate_totp_with_offset(&secret, i)
                                {
                                    if verify_pin_data(
                                        totp_pin,
                                        pin_data,
                                        self.grid_seed,
                                        &self.server_security_salt,
                                    )? {
                                        pin_result = true;
                                        break;
                                    }
                                }
                            }
                        }
                    } else if let Some(geo_pin_str) = geolock_pin {
                        if let Ok(geo_pin_val) = geo_pin_str.parse::<u32>() {
                            pin_result = verify_pin_data(
                                geo_pin_val,
                                pin_data,
                                self.grid_seed,
                                &self.server_security_salt,
                            )?;
                        }
                    } else {
                        error!(
                            "Invalid PIN flags set for user {} - user cannot log-in until fixed",
                            self.login
                        );
                        pin_result = false;
                    }
                }
            } else {
                pin_result = false;
            }
        }

        let session_key = srp.get_strong_session_key();
        let m2 = srp.finalize();
        let _ = srp;

        if !pin_result {
            self.send_error_response(AuthResult::FailParentcontrol)
                .await?;
            self.metrics.increment_auth_failed();
            return Ok(());
        }

        if !self
            .verify_version(&proof.a, &proof.crc_hash, false)
            .await?
        {
            error!(
                "Account {} tried to login with modified client!",
                self.login
            );
            self.send_error_response(AuthResult::FailVersionInvalid)
                .await?;
            self.metrics.increment_auth_failed();
            return Ok(());
        }

        if let Some(_geo_pin) = self.geo_unlock_pin {
            if let Some(account_id) = self.account_id {
                let _ = self
                    .database
                    .accounts
                    .update_geolock_pin(account_id, Some(0))
                    .await;
            }
        }

        if self.config.geo_locking {
            if let Some(last_ip_str) = &self.last_ip {
                if last_ip_str != &self.remote_addr.ip().to_string() {
                    let pin = rand::thread_rng().gen_range(100000..=999999);
                    if let Some(account_id) = self.account_id {
                        let _ = self
                            .database
                            .accounts
                            .update_geolock_pin(account_id, Some(pin))
                            .await;
                    }

                    info!(
                        "Account '{}' ({}) using IP '{}' has been geolocked with PIN {}",
                        self.login,
                        self.account_id.unwrap_or(0),
                        self.remote_addr.ip(),
                        pin
                    );

                    self.send_error_response(AuthResult::FailParentcontrol)
                        .await?;
                    return Ok(());
                }
            }
        }

        info!("Account {} successfully authenticated", self.login);
        self.metrics.increment_auth_success();

        let ip_str = self.remote_addr.ip().to_string();

        let os_bytes = self.os.to_le_bytes();
        let os_str = String::from_utf8_lossy(&os_bytes[..3]).into_owned();

        let platform_bytes = self.platform.to_le_bytes();
        let platform_str = String::from_utf8_lossy(&platform_bytes[..3]).into_owned();

        let locale = self.get_locale_by_name(&self.localization_name);

        self.database
            .accounts
            .update_login_info(
                &self.safelogin,
                &session_key,
                &ip_str,
                locale,
                &os_str,
                &platform_str,
            )
            .await?;

        // Send proof response
        let proof_response = AuthLogonProofS {
            cmd: AuthCmd::LogonProof as u8,
            error: AuthResult::Success as u8,
            m2,
            account_flags: None,
            survey_id: None,
            login_flags: None,
        };

        let mut response_buf = BytesMut::new();
        proof_response.write_to(&mut response_buf, self.build);
        self.send_packet(&response_buf).await?;

        self.status = AuthStatus::Authed;

        Ok(())
    }

    async fn handle_reconnect_challenge(&mut self) -> Result<()> {
        let challenge = match AuthLogonChallengeC::read_from(&mut self.buffer) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to parse reconnect challenge: {}", e);
                return Ok(());
            }
        };

        self.status = AuthStatus::Closed;

        self.login = challenge.username.clone();
        self.safelogin = challenge.username.clone();
        self.build = challenge.build;

        let mut os_bytes = challenge.os;
        if os_bytes.len() >= 3 {
            os_bytes[3] = 0;
            os_bytes.swap(0, 2);
            self.os = u32::from_le_bytes(os_bytes);
        }

        let mut platform_bytes = challenge.platform;
        if platform_bytes.len() >= 3 {
            platform_bytes[3] = 0;
            platform_bytes.swap(0, 2);
            self.platform = u32::from_le_bytes(platform_bytes);
        }

        if challenge.country.len() >= 4 {
            self.localization_name = format!(
                "{}{}{}{}",
                challenge.country[3] as char,
                challenge.country[2] as char,
                challenge.country[1] as char,
                challenge.country[0] as char
            );
        }

        let (session_key_hex, account_id) = match self
            .database
            .accounts
            .find_session_key(&self.safelogin)
            .await?
        {
            Some((key, id)) if !key.is_empty() => (key, id),
            Some(_) => {
                error!(
                    "User {} tried to reconnect but has no session key",
                    self.login
                );
                return Ok(());
            }
            None => {
                error!(
                    "User {} tried to reconnect but account not found in database",
                    self.login
                );
                return Ok(());
            }
        };

        self.account_id = Some(account_id);

        let mut reconnect_proof = [0u8; 16];
        rand::thread_rng().fill(&mut reconnect_proof);
        self.reconnect_proof = Some(reconnect_proof);

        let response = AuthReconnectChallengeS {
            cmd: AuthCmd::ReconnectChallenge as u8,
            error: 0,
            reconnect_proof,
            version_challenge: VERSION_CHALLENGE,
        };

        let mut buf = BytesMut::new();
        response.write_to(&mut buf);
        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;

        self.status = AuthStatus::ReconProof;

        Ok(())
    }

    async fn handle_reconnect_proof(&mut self) -> Result<()> {
        let proof = match AuthReconnectProofC::read_from(&mut self.buffer) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to parse reconnect proof: {}", e);
                return Ok(());
            }
        };

        self.status = AuthStatus::Closed;

        let session_key_hex = match self
            .database
            .accounts
            .find_session_key(&self.safelogin)
            .await?
        {
            Some((key, _)) if !key.is_empty() => key,
            Some(_) => {
                error!("User {} session key is NULL or empty", self.login);
                return Ok(());
            }
            None => {
                error!("User {} session key not found", self.login);
                return Ok(());
            }
        };

        let reconnect_proof_challenge = match self.reconnect_proof {
            Some(rp) => rp,
            None => {
                error!("No reconnect proof challenge stored");
                return Ok(());
            }
        };

        let session_key_bytes = match hex::decode(&session_key_hex) {
            Ok(mut bytes) => {
                bytes.reverse();
                bytes
            }
            Err(e) => {
                error!("Failed to decode session key: {}", e);
                return Ok(());
            }
        };

        let mut hasher = Sha1::new();
        hasher.update(self.login.as_bytes());
        hasher.update(&proof.r1);
        hasher.update(&reconnect_proof_challenge);
        hasher.update(&session_key_bytes);
        let expected_digest = hasher.finalize();

        if &expected_digest[..] != &proof.r2[..] {
            error!(
                "User {} tried to reconnect, but session invalid",
                self.login
            );
            return Ok(());
        }

        if !self.verify_version(&proof.r1, &proof.r3, true).await? {
            error!(
                "Account {} tried to reconnect with modified client!",
                self.login
            );

            let response = AuthReconnectProofS {
                cmd: AuthCmd::ReconnectProof as u8,
                error: AuthResult::FailVersionInvalid as u8,
            };

            let mut buf = BytesMut::new();
            response.write_to(&mut buf);
            self.stream.write_all(&buf).await?;
            self.stream.flush().await?;

            return Ok(());
        }

        let response = AuthReconnectProofS {
            cmd: AuthCmd::ReconnectProof as u8,
            error: AuthResult::Success as u8,
        };

        let mut buf = BytesMut::new();
        response.write_to(&mut buf);
        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;

        self.status = AuthStatus::Authed;

        info!("Account {} successfully reconnected", self.login);

        Ok(())
    }

    async fn handle_realm_list(&mut self) -> Result<()> {
        if self.buffer.len() < 5 {
            return Ok(());
        }

        self.buffer.advance(5);

        if self.account_id.is_none() {
            warn!("RealmList requested but account_id is not set");
            return Ok(());
        }

        if self.status != AuthStatus::Authed {
            warn!("RealmList requested but not authenticated");
            return Ok(());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(last_request) = self.last_realm_list_request {
            let delay = now.saturating_sub(last_request);
            let min_delay = self.config.min_realm_list_delay as u64;
            if delay < min_delay {
                warn!(
                    "User {} IP {} is sending CMD_REALM_LIST too frequently. Delay = {} seconds",
                    self.login,
                    self.remote_addr.ip(),
                    delay
                );
                return Ok(());
            }
        }
        self.last_realm_list_request = Some(now);

        let packet = self
            .realm_list
            .build_packet(
                self.build,
                self.account_id,
                &self.account_security_on_realm,
                self.account_default_security_level,
                &self.allowed_builds,
                Some(&self.remote_addr.ip()),
            )
            .await?;

        let mut response = BytesMut::new();
        response.put_u8(AuthCmd::RealmList as u8);
        response.put_u16_le(packet.len() as u16);
        response.extend_from_slice(&packet);

        self.send_packet(&response).await?;
        self.metrics.increment_realm_list();

        Ok(())
    }

    async fn handle_xfer_accept(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.buffer.advance(1);

        if let Some(patch_path) = &self.patch_file {
            let mut file = tokio::fs::File::open(patch_path).await?;
            self.transfer_patch_file(&mut file).await?;
        }
        Ok(())
    }

    async fn handle_xfer_resume(&mut self) -> Result<()> {
        if self.buffer.len() < 9 {
            return Ok(());
        }

        self.buffer.advance(1);
        let start_pos = self.buffer.get_u64_le();

        if self.patch_file.is_none() {
            error!("XferResume but no patch file");
            return Ok(());
        }

        let patch_path = self.patch_file.as_ref().unwrap();
        let metadata = tokio::fs::metadata(patch_path).await?;
        let file_size = metadata.len();

        if start_pos >= file_size {
            error!("Invalid resume position: {} >= {}", start_pos, file_size);
            return Ok(());
        }

        let mut file = tokio::fs::File::open(patch_path).await?;
        if start_pos > 0 {
            use tokio::io::{AsyncSeekExt, SeekFrom};
            file.seek(SeekFrom::Start(start_pos)).await?;
        }

        self.transfer_patch_file(&mut file).await?;
        Ok(())
    }

    async fn handle_xfer_cancel(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            self.buffer.advance(1);
        }
        Ok(())
    }

    async fn transfer_patch_file(&mut self, file: &mut tokio::fs::File) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut buffer = vec![0u8; 4096];
        let mut chunk_buf = BytesMut::with_capacity(4096 + 3);

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        loop {
            let n = match file.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    error!("Error reading patch file: {}", e);
                    return Err(e.into());
                }
            };

            chunk_buf.clear();
            chunk_buf.put_u8(crate::auth::common::codes::AuthCmd::XferData as u8);
            chunk_buf.put_u16_le(n as u16);
            chunk_buf.extend_from_slice(&buffer[..n]);

            if let Err(e) = self.stream.write_all(&chunk_buf).await {
                error!("Error sending patch chunk: {}", e);
                return Err(e.into());
            }

            if let Err(e) = self.stream.flush().await {
                error!("Error flushing patch data: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    async fn load_account_security_levels(&mut self, account_id: u32) -> Result<()> {
        let rows = self
            .database
            .accounts
            .find_account_access(account_id)
            .await?;

        for row in rows {
            if row.realm_id < 0 {
                self.account_default_security_level = row.gmlevel;
            } else {
                self.account_security_on_realm
                    .insert(row.realm_id as u32, row.gmlevel);
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn get_security_on(&self, realm_id: u32) -> u8 {
        self.account_security_on_realm
            .get(&realm_id)
            .copied()
            .unwrap_or(self.account_default_security_level)
    }

    async fn verify_version(
        &self,
        a: &[u8],
        version_proof: &[u8],
        is_reconnect: bool,
    ) -> Result<bool> {
        let allowed_clients = self
            .allowed_builds
            .find_build_with_os_platform(self.build, self.os, self.platform)
            .await;

        if allowed_clients.is_empty() {
            return Ok(false);
        }

        if !self.config.strict_version_check {
            return Ok(true);
        }

        for build_info in &allowed_clients {
            let version_hash = if is_reconnect {
                &[0u8; 20]
            } else {
                &build_info.integrity_hash
            };

            if !is_reconnect && version_hash.iter().all(|&b| b == 0) {
                return Ok(true);
            }

            if a.len() != 32 {
                continue;
            }

            let a_bytes: [u8; 32] = match a.try_into() {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            let mut hasher = Sha1::new();
            hasher.update(&a_bytes);
            hasher.update(version_hash);
            let expected_hash = hasher.finalize();

            if &expected_hash[..] == version_proof {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn get_locale_by_name(&self, name: &str) -> u8 {
        match name {
            "enUS" | "enGB" => 0,
            "koKR" => 1,
            "frFR" => 2,
            "deDE" => 3,
            "zhCN" => 4,
            "zhTW" => 5,
            "esES" => 6,
            "esMX" => 7,
            "ruRU" => 8,
            _ => 0,
        }
    }

    async fn send_error_response(&mut self, result: AuthResult) -> Result<()> {
        let mut response = BytesMut::new();
        response.put_u8(AuthCmd::LogonChallenge as u8);
        response.put_u8(0);
        response.put_u8(result as u8);
        self.send_packet(&response).await
    }

    async fn send_packet(&mut self, data: &BytesMut) -> Result<()> {
        self.stream
            .write_all(data)
            .await
            .context("Failed to write to socket")?;
        self.stream
            .flush()
            .await
            .context("Failed to flush socket")?;
        Ok(())
    }
}
