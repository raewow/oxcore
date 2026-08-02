use super::super::models::account::*;
use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::server::SrpVerifier;

const ACCOUNT_COLUMNS: &str = "id, username, gmlevel, sessionkey, v, s, reg_mail, token_key, email, joindate, last_ip, last_attempt_ip, last_local_ip, failed_logins, locked, lock_country, last_login, last_pwd_reset, online, expansion, mutetime, mutereason, muteby, locale, os, platform, recruiter, current_realm, banned, mail_verif, remember_token, flags, security, pass_verif, email_verif, email_check, nostalrius_token, nostalrius_token_enabled, nostalrius_email, nostalrius_reason, geolock_pin, totp_secret";
#[derive(Clone)]
pub struct AccountRepository {
    pool: Arc<PgPool>,
}
impl AccountRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    pub async fn find_bnet_credentials(&self, login: &str) -> Result<Option<BnetCredentials>> {
        let row: Option<(i64, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id, username, bnet_salt, bnet_verifier FROM auth.account WHERE username = $1",
        )
        .bind(login.to_uppercase())
        .fetch_optional(&*self.pool)
        .await?;
        row.map(|(id, username, salt, verifier)| {
            let salt: [u8; 32] = hex::decode(salt.ok_or_else(|| anyhow!("missing bnet salt"))?)?
                .try_into()
                .map_err(|_| anyhow!("stored bnet salt is not 32 bytes"))?;
            Ok(BnetCredentials {
                id: id
                    .try_into()
                    .map_err(|_| anyhow!("account id outside u32 range"))?,
                username,
                salt,
                verifier: hex::decode(verifier.ok_or_else(|| anyhow!("missing bnet verifier"))?)?,
            })
        })
        .transpose()
    }
    pub async fn find_by_login_ticket(&self, ticket: &str) -> Result<Option<BnetTicketAccount>> {
        let row: Option<(i64,String)> = sqlx::query_as("SELECT id, username FROM auth.account WHERE bnet_login_ticket = $1 AND bnet_login_ticket_expiry > $2").bind(ticket).bind(chrono::Utc::now().timestamp()).fetch_optional(&*self.pool).await?;
        row.map(|(id, username)| {
            Ok(BnetTicketAccount {
                id: id
                    .try_into()
                    .map_err(|_| anyhow!("account id outside u32 range"))?,
                username,
            })
        })
        .transpose()
    }
    pub async fn find_by_username(&self, username: &str) -> Result<Option<AccountRow>> {
        sqlx::query_as(&format!(
            "SELECT {ACCOUNT_COLUMNS} FROM auth.account WHERE username = $1"
        ))
        .bind(username)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find account")
    }
    pub async fn find_by_id(&self, id: u32) -> Result<Option<AccountRow>> {
        sqlx::query_as(&format!(
            "SELECT {ACCOUNT_COLUMNS} FROM auth.account WHERE id = $1"
        ))
        .bind(i64::from(id))
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to find account")
    }
    pub async fn is_account_banned(&self, id: u32) -> Result<Option<AccountBannedRow>> {
        sqlx::query_as("SELECT banid, id, bandate, unbandate, bannedby, banreason, active, realm, gmlevel FROM auth.account_banned WHERE id = $1 AND active = 1 AND (unbandate = bandate OR unbandate > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT)").bind(i64::from(id)).fetch_optional(&*self.pool).await.context("Failed to check ban")
    }
    pub async fn find_account_access(&self, id: u32) -> Result<Vec<AccountAccessRow>> {
        sqlx::query_as("SELECT id, gmlevel, \"RealmID\" FROM auth.account_access WHERE id = $1")
            .bind(i64::from(id))
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load access")
    }
    pub async fn find_for_login(&self, username: &str) -> Result<Option<AccountLoginInfo>> {
        sqlx::query_as("SELECT id, locked, last_ip, v, s, security, email_verif, geolock_pin, email, EXTRACT(EPOCH FROM joindate)::BIGINT AS joindate_ts, online FROM auth.account WHERE username = $1").bind(username).fetch_optional(&*self.pool).await.context("Failed to find login")
    }
    pub async fn find_for_world_auth(&self, username: &str) -> Result<Option<SessionAuthInfo>> {
        sqlx::query_as("SELECT id, username, gmlevel, sessionkey, last_ip, locked, expansion, mutetime, locale FROM auth.account WHERE username = $1").bind(username).fetch_optional(&*self.pool).await.context("Failed to find world auth")
    }
    pub async fn get_failed_logins_by_username(&self, username: &str) -> Result<Option<u32>> {
        let value: Option<i64> =
            sqlx::query_scalar("SELECT failed_logins FROM auth.account WHERE username = $1")
                .bind(username)
                .fetch_optional(&*self.pool)
                .await?;
        value
            .map(|x| {
                x.try_into()
                    .map_err(|_| anyhow!("failed login count outside u32 range"))
            })
            .transpose()
    }
    pub async fn find_session_key(&self, username: &str) -> Result<Option<(String, u32)>> {
        let row: Option<(Option<String>, i64)> =
            sqlx::query_as("SELECT sessionkey, id FROM auth.account WHERE username = $1")
                .bind(username)
                .fetch_optional(&*self.pool)
                .await?;
        row.map(|(key, id)| {
            Ok((
                key.unwrap_or_default(),
                id.try_into()
                    .map_err(|_| anyhow!("account id outside u32 range"))?,
            ))
        })
        .transpose()
    }
    pub async fn create_account(&self, username: &str, password: &str) -> Result<u32> {
        let user = NormalizedString::new(username).map_err(|e| anyhow!("Invalid username: {e}"))?;
        let pass = NormalizedString::new(password).map_err(|e| anyhow!("Invalid password: {e}"))?;
        let verifier = SrpVerifier::from_username_and_password(user, pass);
        let name = verifier.username().to_string();
        let bnet = oxcore_shared::crypto::srp6v2::register(&name, password);
        let id: i64 = sqlx::query_scalar("INSERT INTO auth.account (username, v, s, bnet_srp_version, bnet_salt, bnet_verifier) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id").bind(&name).bind(hex::encode_upper(verifier.password_verifier())).bind(hex::encode_upper(verifier.salt())).bind(oxcore_shared::crypto::srp6v2::SRP_VERSION as i16).bind(hex::encode_upper(bnet.salt)).bind(hex::encode_upper(bnet.verifier)).fetch_one(&*self.pool).await.context("Failed to create account")?;
        id.try_into()
            .map_err(|_| anyhow!("account id outside u32 range"))
    }
    pub async fn set_password(&self, id: u32, username: &str, password: &str) -> Result<()> {
        let u = NormalizedString::new(username).map_err(|e| anyhow!("Invalid username: {e}"))?;
        let p = NormalizedString::new(password).map_err(|e| anyhow!("Invalid password: {e}"))?;
        let v = SrpVerifier::from_username_and_password(u, p);
        let b = oxcore_shared::crypto::srp6v2::register(v.username(), password);
        let result=sqlx::query("UPDATE auth.account SET v=$1,s=$2,bnet_srp_version=$3,bnet_salt=$4,bnet_verifier=$5,last_pwd_reset=CURRENT_TIMESTAMP WHERE id=$6 AND username=$7").bind(hex::encode_upper(v.password_verifier())).bind(hex::encode_upper(v.salt())).bind(oxcore_shared::crypto::srp6v2::SRP_VERSION as i16).bind(hex::encode_upper(b.salt)).bind(hex::encode_upper(b.verifier)).bind(i64::from(id)).bind(v.username()).execute(&*self.pool).await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!("Account no longer exists or username changed"));
        }
        Ok(())
    }
    pub async fn set_bnet_credentials(&self, username: &str, password: &str) -> Result<()> {
        let name = username.to_uppercase();
        let b = oxcore_shared::crypto::srp6v2::register(&name, password);
        sqlx::query("UPDATE auth.account SET bnet_srp_version=$1,bnet_salt=$2,bnet_verifier=$3 WHERE username=$4").bind(oxcore_shared::crypto::srp6v2::SRP_VERSION as i16).bind(hex::encode_upper(b.salt)).bind(hex::encode_upper(b.verifier)).bind(name).execute(&*self.pool).await?;
        Ok(())
    }
    pub async fn store_bnet_login_ticket(&self, id: u32, ticket: &str, expiry: i64) -> Result<()> {
        sqlx::query(
            "UPDATE auth.account SET bnet_login_ticket=$1,bnet_login_ticket_expiry=$2 WHERE id=$3",
        )
        .bind(ticket)
        .bind(expiry)
        .bind(i64::from(id))
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
    pub async fn set_gmlevel(&self, id: u32, level: u8) -> Result<()> {
        self.account_i16("gmlevel", id, i16::from(level)).await
    }
    pub async fn upsert_account_access(&self, id: u32, level: u8, realm: i32) -> Result<()> {
        sqlx::query("INSERT INTO auth.account_access (id,gmlevel,\"RealmID\") VALUES ($1,$2,$3) ON CONFLICT (id,\"RealmID\") DO UPDATE SET gmlevel=EXCLUDED.gmlevel").bind(i64::from(id)).bind(i16::from(level)).bind(realm).execute(&*self.pool).await?;
        Ok(())
    }
    async fn account_i16(&self, column: &str, id: u32, value: i16) -> Result<()> {
        sqlx::query(&format!("UPDATE auth.account SET {column}=$1 WHERE id=$2"))
            .bind(value)
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_session_key(&self, id: u32, key: &str) -> Result<()> {
        sqlx::query("UPDATE auth.account SET sessionkey=$1 WHERE id=$2")
            .bind(key)
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn increment_failed_logins(&self, id: u32) -> Result<()> {
        sqlx::query("UPDATE auth.account SET failed_logins=failed_logins+1 WHERE id=$1")
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn increment_failed_logins_by_username(&self, name: &str) -> Result<()> {
        sqlx::query("UPDATE auth.account SET failed_logins=failed_logins+1 WHERE username=$1")
            .bind(name)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn reset_failed_logins(&self, id: u32) -> Result<()> {
        sqlx::query("UPDATE auth.account SET failed_logins=0 WHERE id=$1")
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_geolock_pin(&self, id: u32, pin: Option<i32>) -> Result<()> {
        sqlx::query("UPDATE auth.account SET geolock_pin=$1 WHERE id=$2")
            .bind(pin)
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_online_status(&self, id: u32, online: u8) -> Result<()> {
        self.account_i16("online", id, i16::from(online)).await
    }
    pub async fn update_last_ip(&self, id: u32, ip: &str) -> Result<()> {
        sqlx::query("UPDATE auth.account SET last_ip=$1 WHERE id=$2")
            .bind(ip)
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_os_platform(&self, id: u32, os: &str, platform: &str) -> Result<()> {
        sqlx::query("UPDATE auth.account SET os=$1,platform=$2 WHERE id=$3")
            .bind(os)
            .bind(platform)
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
    pub async fn update_login_info(
        &self,
        name: &str,
        key: &str,
        ip: &str,
        locale: u8,
        os: &str,
        platform: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE auth.account SET sessionkey=$1,last_ip=$2,last_login=CURRENT_TIMESTAMP,locale=$3,failed_logins=0,os=$4,platform=$5 WHERE username=$6").bind(key).bind(ip).bind(i16::from(locale)).bind(os).bind(platform).bind(name).execute(&*self.pool).await?;
        Ok(())
    }
    pub async fn auto_ban_account(&self, id: u32, duration: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO auth.account_banned (id,bandate,unbandate,bannedby,banreason,active) VALUES ($1,$2,$3,'auth','Too many failed login attempts',1) ON CONFLICT (id,bandate) DO UPDATE SET unbandate=EXCLUDED.unbandate,active=1").bind(i64::from(id)).bind(now).bind(now+duration).execute(&*self.pool).await?;
        Ok(())
    }
    pub async fn deactivate_expired_bans(&self) -> Result<u64> {
        Ok(sqlx::query("UPDATE auth.account_banned SET active=0 WHERE active=1 AND unbandate <= EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT AND unbandate <> bandate").execute(&*self.pool).await?.rows_affected())
    }
}
