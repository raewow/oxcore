use super::super::models::account::*;
use anyhow::{anyhow, Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::SrpVerifier;

#[derive(Clone)]
pub struct AccountRepository {
    pool: Arc<MySqlPool>,
}

impl AccountRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// The underlying connection pool, for callers that need raw queries against the same auth
    /// database (e.g. integration tests).
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Look up the Battle.net (SRP6v2) credentials for a login. Returns the account id, the
    /// canonical (uppercased) username, and the stored salt/verifier bytes, or `None` if the
    /// account does not exist or has no bnet credentials set.
    pub async fn find_bnet_credentials(&self, login: &str) -> Result<Option<BnetCredentials>> {
        let username = login.to_uppercase();
        let row: Option<(u32, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT `id`, `username`, `bnet_salt`, `bnet_verifier` FROM `account` \
             WHERE `username` = ?",
        )
        .bind(&username)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to look up bnet credentials")?;

        let Some((id, username, Some(salt_hex), Some(verifier_hex))) = row else {
            return Ok(None);
        };

        let salt_vec = hex::decode(&salt_hex).context("stored bnet_salt is not valid hex")?;
        let salt: [u8; 32] = salt_vec
            .try_into()
            .map_err(|_| anyhow!("stored bnet_salt is not 32 bytes"))?;
        let verifier =
            hex::decode(&verifier_hex).context("stored bnet_verifier is not valid hex")?;

        Ok(Some(BnetCredentials {
            id,
            username,
            salt,
            verifier,
        }))
    }

    /// Resolve a Battle.net login ticket to its account, rejecting tickets that are unknown or
    /// past their stored expiry. Used by BGS `VerifyWebCredentials` to consume the ticket the
    /// client obtained from the REST web-auth flow.
    pub async fn find_by_login_ticket(
        &self,
        ticket: &str,
    ) -> Result<Option<BnetTicketAccount>> {
        let now = chrono::Utc::now().timestamp();
        let row: Option<(u32, String)> = sqlx::query_as(
            "SELECT `id`, `username` FROM `account` \
             WHERE `bnet_login_ticket` = ? AND `bnet_login_ticket_expiry` > ?",
        )
        .bind(ticket)
        .bind(now)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to look up account by login ticket")?;

        Ok(row.map(|(id, username)| BnetTicketAccount { id, username }))
    }

    /// Find account by username (for authentication)
    pub async fn find_by_username(&self, username: &str) -> Result<Option<AccountRow>> {
        sqlx::query_as::<_, AccountRow>(
            r#"SELECT `id`, `username`, `gmlevel`, `sessionkey`, `v`, `s`, `reg_mail`, `token_key`,
               `email`, `joindate`, `last_ip`, `last_attempt_ip`, `last_local_ip`, `failed_logins`,
               `locked`, `lock_country`, `last_login`, `last_pwd_reset`, `online`, `expansion`,
               `mutetime`, `mutereason`, `muteby`, `locale`, `os`, `platform`, `recruiter`,
               `current_realm`, `banned`, `mail_verif`, `remember_token`, `flags`, `security`,
               `pass_verif`, `email_verif`, `email_check`, `nostalrius_token`,
               `nostalrius_token_enabled`, `nostalrius_email`, `nostalrius_reason`,
               `geolock_pin`, `totp_secret`
               FROM `account`
               WHERE `username` = ?"#,
        )
        .bind(username)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| anyhow!("Failed to find account by username: {:#}", e))
    }

    /// Find account by ID
    pub async fn find_by_id(&self, id: u32) -> Result<Option<AccountRow>> {
        sqlx::query_as::<_, AccountRow>(
            r#"SELECT `id`, `username`, `gmlevel`, `sessionkey`, `v`, `s`, `reg_mail`, `token_key`,
               `email`, `joindate`, `last_ip`, `last_attempt_ip`, `last_local_ip`, `failed_logins`,
               `locked`, `lock_country`, `last_login`, `last_pwd_reset`, `online`, `expansion`,
               `mutetime`, `mutereason`, `muteby`, `locale`, `os`, `platform`, `recruiter`,
               `current_realm`, `banned`, `mail_verif`, `remember_token`, `flags`, `security`,
               `pass_verif`, `email_verif`, `email_check`, `nostalrius_token`,
               `nostalrius_token_enabled`, `nostalrius_email`, `nostalrius_reason`,
               `geolock_pin`, `totp_secret`
               FROM `account`
               WHERE `id` = ?"#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| anyhow!("Failed to find account by ID: {:#}", e))
    }

    /// Check if account is banned (active ban check)
    pub async fn is_account_banned(&self, account_id: u32) -> Result<Option<AccountBannedRow>> {
        sqlx::query_as::<_, AccountBannedRow>(
            r#"SELECT `banid`, `id`, `bandate`, `unbandate`, `bannedby`, `banreason`, `active`, `realm`, `gmlevel`
               FROM `account_banned`
               WHERE `id` = ? AND `active` = 1
               AND (`unbandate` = `bandate` OR `unbandate` > UNIX_TIMESTAMP())"#,
        )
        .bind(account_id as i64)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to check account ban status")
    }

    /// Find all account access records for an account
    pub async fn find_account_access(&self, account_id: u32) -> Result<Vec<AccountAccessRow>> {
        sqlx::query_as::<_, AccountAccessRow>(
            "SELECT `id`, `gmlevel`, `RealmID` FROM `account_access` WHERE `id` = ?",
        )
        .bind(account_id)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to find account access records")
    }

    /// Get account login info for authentication challenge
    /// Returns minimal account data needed for SRP6 authentication
    pub async fn find_for_login(&self, username: &str) -> Result<Option<AccountLoginInfo>> {
        let result = sqlx::query_as::<_, AccountLoginInfo>(
            r#"SELECT `id`, `locked`, `last_ip`, `v`, `s`, `security`, `email_verif`,
               `geolock_pin`, `email`, UNIX_TIMESTAMP(`joindate`) as `joindate_ts`, `online`
               FROM `account` WHERE `username` = ?"#,
        )
        .bind(username)
        .fetch_optional(&*self.pool)
        .await;

        if let Err(ref e) = result {
            tracing::error!(
                "Database error in find_for_login for username '{}': {:?}",
                username,
                e
            );
        }

        result.context("Failed to find account for login")
    }

    /// Get session authentication info for world server login
    /// Returns minimal account data needed for CMSG_AUTH_SESSION handling.
    pub async fn find_for_world_auth(&self, username: &str) -> Result<Option<SessionAuthInfo>> {
        sqlx::query_as::<_, SessionAuthInfo>(
            r#"SELECT `id`, `username`, `gmlevel`, `sessionkey`, `last_ip`, `locked`,
                      `expansion`, `mutetime`, `locale`
               FROM `account`
               WHERE `username` = ?"#,
        )
        .bind(username)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find account for world auth")
    }

    /// Get failed login count by username
    pub async fn get_failed_logins_by_username(&self, username: &str) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, u32>("SELECT `failed_logins` FROM `account` WHERE `username` = ?")
            .bind(username)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to get failed logins by username")
    }

    /// Get session key and account ID for reconnect authentication
    pub async fn find_session_key(&self, username: &str) -> Result<Option<(String, u32)>> {
        sqlx::query_as::<_, (String, u32)>(
            "SELECT `sessionkey`, `id` FROM `account` WHERE `username` = ?",
        )
        .bind(username)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to get session key for reconnect")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new account, computing credentials for both clients: the vanilla SRP6 verifier
    /// (`v`/`s`, SHA-1) for the 1.12 client and the Battle.net SRP6v2 verifier
    /// (`bnet_verifier`/`bnet_salt`, SHA-256) for the 1.14.x client. A single password sets
    /// both, so the account works from either client.
    pub async fn create_account(&self, username: &str, password: &str) -> Result<u32> {
        let username_norm =
            NormalizedString::new(username).map_err(|e| anyhow!("Invalid username: {}", e))?;
        let password_norm =
            NormalizedString::new(password).map_err(|e| anyhow!("Invalid password: {}", e))?;

        let verifier = SrpVerifier::from_username_and_password(username_norm, password_norm);
        let stored_username = verifier.username().to_string();

        if self.find_by_username(&stored_username).await?.is_some() {
            return Err(anyhow!("Account '{}' already exists", stored_username));
        }
        let salt_hex = hex::encode_upper(verifier.salt());
        let v_hex = hex::encode_upper(verifier.password_verifier());

        // Bnet identity is the account username; register() applies HEX(SHA256(UPPER(..))).
        let bnet = crate::crypto::srp6v2::register(&stored_username, password);
        let bnet_salt_hex = hex::encode_upper(bnet.salt);
        let bnet_verifier_hex = hex::encode_upper(&bnet.verifier);

        let result = sqlx::query(
            "INSERT INTO `account` (`username`, `v`, `s`, `bnet_srp_version`, `bnet_salt`, \
             `bnet_verifier`) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&stored_username)
        .bind(&v_hex)
        .bind(&salt_hex)
        .bind(crate::crypto::srp6v2::SRP_VERSION as u8)
        .bind(&bnet_salt_hex)
        .bind(&bnet_verifier_hex)
        .execute(&*self.pool)
        .await
        .context("Failed to create account")?;

        Ok(result.last_insert_id() as u32)
    }

    /// Update the Battle.net (SRP6v2) credentials for an existing account, e.g. after a
    /// password change. The vanilla `v`/`s` are updated separately by the caller.
    pub async fn set_bnet_credentials(&self, username: &str, password: &str) -> Result<()> {
        let stored_username = username.to_uppercase();
        let bnet = crate::crypto::srp6v2::register(&stored_username, password);

        sqlx::query(
            "UPDATE `account` SET `bnet_srp_version` = ?, `bnet_salt` = ?, `bnet_verifier` = ? \
             WHERE `username` = ?",
        )
        .bind(crate::crypto::srp6v2::SRP_VERSION as u8)
        .bind(hex::encode_upper(bnet.salt))
        .bind(hex::encode_upper(&bnet.verifier))
        .bind(&stored_username)
        .execute(&*self.pool)
        .await
        .context("Failed to update bnet credentials")?;

        Ok(())
    }

    /// Store a freshly issued login ticket and its expiry (unix seconds) for an account.
    pub async fn store_bnet_login_ticket(
        &self,
        account_id: u32,
        ticket: &str,
        expiry_unix: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE `account` SET `bnet_login_ticket` = ?, `bnet_login_ticket_expiry` = ? \
             WHERE `id` = ?",
        )
        .bind(ticket)
        .bind(expiry_unix)
        .bind(account_id)
        .execute(&*self.pool)
        .await
        .context("Failed to store bnet login ticket")?;

        Ok(())
    }

    /// Set the gmlevel column on an account.
    pub async fn set_gmlevel(&self, account_id: u32, gmlevel: u8) -> Result<()> {
        sqlx::query("UPDATE `account` SET `gmlevel` = ? WHERE `id` = ?")
            .bind(gmlevel)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to set account gmlevel")?;
        Ok(())
    }

    /// Set per-realm GM access (-1 = all realms).
    pub async fn upsert_account_access(
        &self,
        account_id: u32,
        gmlevel: u8,
        realm_id: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO `account_access` (`id`, `gmlevel`, `RealmID`)
               VALUES (?, ?, ?)
               ON DUPLICATE KEY UPDATE `gmlevel` = ?"#,
        )
        .bind(account_id)
        .bind(gmlevel)
        .bind(realm_id)
        .bind(gmlevel)
        .execute(&*self.pool)
        .await
        .context("Failed to set account access")?;
        Ok(())
    }

    /// Update session key after successful login
    pub async fn update_session_key(&self, account_id: u32, session_key: &str) -> Result<()> {
        sqlx::query("UPDATE `account` SET `sessionkey` = ? WHERE `id` = ?")
            .bind(session_key)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update session key")?;
        Ok(())
    }

    /// Increment failed login counter
    pub async fn increment_failed_logins(&self, account_id: u32) -> Result<()> {
        sqlx::query("UPDATE `account` SET `failed_logins` = `failed_logins` + 1 WHERE `id` = ?")
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to increment failed logins")?;
        Ok(())
    }

    /// Increment failed login counter by username (before account_id is known)
    pub async fn increment_failed_logins_by_username(&self, username: &str) -> Result<()> {
        sqlx::query(
            "UPDATE `account` SET `failed_logins` = `failed_logins` + 1 WHERE `username` = ?",
        )
        .bind(username)
        .execute(&*self.pool)
        .await
        .context("Failed to increment failed logins by username")?;
        Ok(())
    }

    /// Reset failed logins counter (on successful authentication)
    pub async fn reset_failed_logins(&self, account_id: u32) -> Result<()> {
        sqlx::query("UPDATE `account` SET `failed_logins` = 0 WHERE `id` = ?")
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to reset failed logins")?;
        Ok(())
    }

    /// Update geolock PIN
    pub async fn update_geolock_pin(&self, account_id: u32, pin: Option<i32>) -> Result<()> {
        sqlx::query("UPDATE `account` SET `geolock_pin` = ? WHERE `id` = ?")
            .bind(pin)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update geolock PIN")?;
        Ok(())
    }

    /// Update online status
    pub async fn update_online_status(&self, account_id: u32, online: u8) -> Result<()> {
        sqlx::query("UPDATE `account` SET `online` = ? WHERE `id` = ?")
            .bind(online)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update online status")?;
        Ok(())
    }

    /// Update last IP address
    pub async fn update_last_ip(&self, account_id: u32, ip: &str) -> Result<()> {
        sqlx::query("UPDATE `account` SET `last_ip` = ? WHERE `id` = ?")
            .bind(ip)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update last IP")?;
        Ok(())
    }

    /// Update OS and platform information
    pub async fn update_os_platform(
        &self,
        account_id: u32,
        os: &str,
        platform: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE `account` SET `os` = ?, `platform` = ? WHERE `id` = ?")
            .bind(os)
            .bind(platform)
            .bind(account_id)
            .execute(&*self.pool)
            .await
            .context("Failed to update OS/platform")?;
        Ok(())
    }

    /// Update session key and login metadata after successful authentication
    pub async fn update_login_info(
        &self,
        username: &str,
        session_key: &str,
        last_ip: &str,
        locale: u8,
        os: &str,
        platform: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE `account`
               SET `sessionkey` = ?, `last_ip` = ?, `last_login` = NOW(),
                   `locale` = ?, `failed_logins` = 0, `os` = ?, `platform` = ?
               WHERE `username` = ?"#,
        )
        .bind(session_key)
        .bind(last_ip)
        .bind(locale)
        .bind(os)
        .bind(platform)
        .bind(username)
        .execute(&*self.pool)
        .await
        .context("Failed to update login info")?;
        Ok(())
    }

    /// Auto-ban account for too many failed logins
    pub async fn auto_ban_account(&self, account_id: u32, ban_duration_seconds: i64) -> Result<()> {
        let unban_timestamp = chrono::Utc::now().timestamp() + ban_duration_seconds;

        sqlx::query(
            r#"INSERT INTO `account_banned` (`id`, `bandate`, `unbandate`, `bannedby`, `banreason`, `active`)
               VALUES (?, UNIX_TIMESTAMP(), ?, 'auth', 'Too many failed login attempts', 1)
               ON DUPLICATE KEY UPDATE `unbandate` = ?, `active` = 1"#,
        )
        .bind(account_id as i64)
        .bind(unban_timestamp)
        .bind(unban_timestamp)
        .execute(&*self.pool)
        .await
        .context("Failed to auto-ban account")?;

        Ok(())
    }

    /// Deactivate expired account bans
    pub async fn deactivate_expired_bans(&self) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE `account_banned` SET `active` = 0 WHERE `active` = 1 AND `unbandate` <= UNIX_TIMESTAMP() AND `unbandate` <> `bandate`",
        )
        .execute(&*self.pool)
        .await
        .context("Failed to deactivate expired bans")?;

        Ok(result.rows_affected())
    }
}
