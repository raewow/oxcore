use sqlx::types::chrono::{DateTime, Utc};
use sqlx::FromRow;

/// Represents a row from the `realmlist` table in the auth database.
/// Contains realm server information for the realm selection screen.
#[derive(FromRow, Debug, Clone)]
pub struct RealmRow {
    pub id: u32,
    pub name: String,
    pub address: String,
    #[sqlx(rename = "localAddress")]
    pub local_address: String,
    #[sqlx(rename = "localSubnetMask")]
    pub local_subnet_mask: String,
    pub port: i32,
    pub icon: u8,
    pub realmflags: u8,
    pub timezone: u8,
    #[sqlx(rename = "allowedSecurityLevel")]
    pub allowed_security_level: u8,
    pub population: f32,
    pub gamebuild_min: u32,
    pub gamebuild_max: u32,
    pub flag: u8,
    pub realmbuilds: String,
    pub last_seen: Option<DateTime<Utc>>,
}

/// Represents a row from the `realmcharacters` table in the auth database.
/// Tracks character counts per realm for each account (shown on realm selection screen).
#[derive(FromRow, Debug, Clone)]
pub struct RealmCharactersRow {
    pub realmid: u32,
    pub acctid: u64,
    pub numchars: u8,
}

/// Represents a row from the `allowed_clients` table in the auth database.
/// Defines which client builds (versions) are allowed to connect to the server.
#[derive(FromRow, Debug, Clone)]
pub struct AllowedClientRow {
    pub major_version: u8,
    pub minor_version: u8,
    pub bugfix_version: u8,
    pub hotfix_version: String, // CHAR(1) in SQL
    pub build: u32,             // MEDIUMINT UNSIGNED
    pub os: String,
    pub platform: String,
    pub integrity_hash: String,
}
