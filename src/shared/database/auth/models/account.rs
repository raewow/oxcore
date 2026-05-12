use sqlx::types::chrono::NaiveDateTime;
use sqlx::FromRow;

/// Represents a row from the `account` table in auth database.
/// Contains all account authentication and metadata.
#[derive(FromRow, Debug, Clone)]
pub struct AccountRow {
    pub id: u32,
    pub username: String,
    pub gmlevel: u8,
    pub sessionkey: Option<String>,
    pub v: Option<String>, // SRP6 verifier
    pub s: Option<String>, // SRP6 salt
    pub reg_mail: String,
    pub token_key: String,
    pub email: Option<String>,
    pub joindate: NaiveDateTime,
    pub last_ip: String,
    pub last_attempt_ip: String,
    pub last_local_ip: String,
    pub failed_logins: u32,
    pub locked: u8,
    pub lock_country: String,
    pub last_login: NaiveDateTime,
    pub last_pwd_reset: NaiveDateTime,
    pub online: u8,
    pub expansion: u8,
    pub mutetime: i64,
    pub mutereason: String,
    pub muteby: String,
    pub locale: u8,
    pub os: String,
    pub platform: String,
    pub recruiter: i32,
    pub current_realm: u8,
    pub banned: u8,
    pub mail_verif: u8,
    pub remember_token: String,
    pub flags: u32,
    pub security: Option<String>,
    pub pass_verif: Option<String>,
    pub email_verif: u8,
    pub email_check: Option<String>,
    pub nostalrius_token: Option<String>,
    pub nostalrius_token_enabled: u8,
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
    pub gmlevel: u8,
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
    pub id: u32,     // Account ID
    pub gmlevel: u8, // GM level for this realm
    #[sqlx(rename = "RealmID")]
    pub realm_id: i32, // -1 for all realms, or specific realm ID
}

/// Minimal account info needed for login challenge.
/// Used during authentication flow to avoid loading full AccountRow.
#[derive(FromRow, Debug, Clone)]
pub struct AccountLoginInfo {
    pub id: u32,
    pub locked: u8, // tinyint unsigned in SQL
    pub last_ip: Option<String>,
    pub v: Option<String>,
    pub s: Option<String>,
    pub security: Option<String>,
    pub email_verif: bool, // tinyint(1) in SQL, treated as BOOLEAN by sqlx
    pub geolock_pin: Option<i32>,
    pub email: Option<String>,
    pub joindate_ts: Option<i64>,
    pub online: u8,
}

/// Session authentication info for world server login.
/// Contains minimal account data needed for CMSG_AUTH_SESSION handling.
#[derive(FromRow, Debug, Clone)]
pub struct SessionAuthInfo {
    pub id: u32,
    pub username: String,
    pub gmlevel: u8,
    pub sessionkey: Option<String>,
    pub last_ip: Option<String>,
    pub locked: u8,
    pub expansion: u8,
    pub mutetime: i64,
    pub locale: u8,
}
