use sqlx::types::chrono::{DateTime, Utc};
use sqlx::FromRow;

/// Battle.net (SRP6v2) credentials for a login, decoded from the account row.
#[derive(Debug, Clone)]
pub struct BnetCredentials {
    pub id: u32,
    /// Canonical (uppercased) account username.
    pub username: String,
    pub salt: [u8; 32],
    pub verifier: Vec<u8>,
}

/// The account behind a still-valid Battle.net login ticket, resolved during BGS
/// `VerifyWebCredentials`.
#[derive(Debug, Clone)]
pub struct BnetTicketAccount {
    pub id: u32,
    /// Canonical (uppercased) account username.
    pub username: String,
}

/// Represents a row from the `account` table in auth database.
/// Contains all account authentication and metadata.
#[derive(FromRow, Debug, Clone)]
pub struct AccountRow {
    pub id: i64,
    pub username: String,
    pub gmlevel: i16,
    pub sessionkey: Option<String>,
    pub v: Option<String>, // SRP6 verifier
    pub s: Option<String>, // SRP6 salt
    pub reg_mail: String,
    pub token_key: String,
    pub email: Option<String>,
    pub joindate: DateTime<Utc>,
    pub last_ip: String,
    pub last_attempt_ip: String,
    pub last_local_ip: String,
    pub failed_logins: i64,
    pub locked: i16,
    pub lock_country: String,
    pub last_login: DateTime<Utc>,
    pub last_pwd_reset: DateTime<Utc>,
    pub online: i16,
    pub expansion: i16,
    pub mutetime: i64,
    pub mutereason: String,
    pub muteby: String,
    pub locale: i16,
    pub os: String,
    pub platform: String,
    pub recruiter: i32,
    pub current_realm: i16,
    pub banned: i16,
    pub mail_verif: i16,
    pub remember_token: String,
    pub flags: i64,
    pub security: Option<String>,
    pub pass_verif: Option<String>,
    pub email_verif: bool,
    pub email_check: Option<String>,
    pub nostalrius_token: Option<String>,
    pub nostalrius_token_enabled: bool,
    pub nostalrius_email: Option<String>,
    pub nostalrius_reason: Option<String>,
    pub geolock_pin: Option<i32>,
    pub totp_secret: Option<String>,
}

/// Represents a row from the `account_banned` table in auth database.
/// Tracks account ban records with ban/unban dates and reasons.
#[derive(FromRow, Debug, Clone)]
pub struct AccountBannedRow {
    pub banid: i64,
    pub id: i64, // Account ID
    pub bandate: i64,
    pub unbandate: i64,
    pub bannedby: String,
    pub banreason: String,
    pub active: i8,
    pub realm: i8,
    pub gmlevel: i16,
}

/// Represents a row from the `ip_banned` table in auth database.
/// Tracks IP-based bans with ban/unban dates and reasons.
#[derive(FromRow, Debug, Clone)]
pub struct IpBannedRow {
    pub ip: String,
    pub bandate: i32,
    pub unbandate: i32,
    pub bannedby: String,
    pub banreason: String,
}

/// Represents a row from the `account_access` table in auth database.
/// Defines per-realm security levels (GM levels) for accounts.
#[derive(FromRow, Debug, Clone)]
pub struct AccountAccessRow {
    pub id: i64,      // Account ID
    pub gmlevel: i16, // GM level for this realm
    #[sqlx(rename = "RealmID")]
    pub realm_id: i32, // -1 for all realms, or specific realm ID
}

/// Minimal account info needed for login challenge.
/// Used during authentication flow to avoid loading full AccountRow.
#[derive(FromRow, Debug, Clone)]
pub struct AccountLoginInfo {
    pub id: i64,
    pub locked: i16,
    pub last_ip: Option<String>,
    pub v: Option<String>,
    pub s: Option<String>,
    pub security: Option<String>,
    pub email_verif: bool, // tinyint(1) in SQL, treated as BOOLEAN by sqlx
    pub geolock_pin: Option<i32>,
    pub email: Option<String>,
    pub joindate_ts: Option<i64>,
    pub online: i16,
}

/// Session authentication info for world server login.
/// Contains minimal account data needed for CMSG_AUTH_SESSION handling.
#[derive(FromRow, Debug, Clone)]
pub struct SessionAuthInfo {
    pub id: i64,
    pub username: String,
    pub gmlevel: i16,
    pub sessionkey: Option<String>,
    pub last_ip: Option<String>,
    pub locked: i16,
    pub expansion: i16,
    pub mutetime: i64,
    pub locale: i16,
}
