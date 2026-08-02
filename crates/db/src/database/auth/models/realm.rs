use sqlx::types::chrono::{DateTime, Utc};
use sqlx::FromRow;

/// Represents a row from the `realmlist` table in the auth database.
/// Contains realm server information for the realm selection screen.
#[derive(FromRow, Debug, Clone)]
pub struct RealmRow {
    pub id: i64,
    pub name: String,
    pub address: String,
    #[sqlx(rename = "localAddress")]
    pub local_address: String,
    #[sqlx(rename = "localSubnetMask")]
    pub local_subnet_mask: String,
    pub port: i32,
    pub icon: i16,
    pub realmflags: i16,
    pub timezone: i16,
    #[sqlx(rename = "allowedSecurityLevel")]
    pub allowed_security_level: i16,
    pub population: f32,
    pub gamebuild_min: i64,
    pub gamebuild_max: i64,
    pub flag: i16,
    pub realmbuilds: String,
    pub last_seen: Option<DateTime<Utc>>,
}

/// Represents a row from the `realmcharacters` table in the auth database.
/// Tracks character counts per realm for each account (shown on realm selection screen).
#[derive(FromRow, Debug, Clone)]
pub struct RealmCharactersRow {
    pub realmid: i64,
    pub acctid: i64,
    pub numchars: i16,
}

/// Represents a row from the `allowed_clients` table in the auth database.
/// Defines which client builds (versions) are allowed to connect to the server.
#[derive(FromRow, Debug, Clone)]
pub struct AllowedClientRow {
    pub major_version: i16,
    pub minor_version: i16,
    pub bugfix_version: i16,
    pub hotfix_version: String, // CHAR(1) in SQL
    pub build: i32,
    pub os: String,
    pub platform: String,
    pub integrity_hash: String,
}
