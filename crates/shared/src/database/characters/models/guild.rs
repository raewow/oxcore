use sqlx::FromRow;

/// Guild table row
///
/// Maps to the `guild` table in the characters database.
/// Contains core guild information including name, leader, emblem, and bank data.
#[derive(FromRow, Debug, Clone)]
pub struct GuildRow {
    pub guild_id: u32,
    pub name: String,
    pub leader_guid: u32,
    /// INT (signed) - emblem style
    pub emblem_style: i32,
    /// INT (signed) - emblem color
    pub emblem_color: i32,
    /// INT (signed) - border style
    pub border_style: i32,
    /// INT (signed) - border color
    pub border_color: i32,
    /// INT (signed) - background color
    pub background_color: i32,
    pub info: String,
    pub motd: String,
    pub create_date: i64,
    pub bank_money: u32,
}

/// Guild member table row
///
/// Maps to the `guild_member` table in the characters database.
/// Contains basic guild membership information.
#[derive(FromRow, Debug, Clone)]
pub struct GuildMemberRow {
    pub guild_id: u32,
    pub guid: u32,
    pub rank: u8,
    pub player_note: String,
    pub officer_note: String,
}

/// Guild member with character data (from JOIN query)
///
/// Result of LEFT JOIN between guild_member and characters tables.
/// Character fields are Option<T> because they may be NULL if character was deleted.
#[derive(FromRow, Debug, Clone)]
pub struct GuildMemberWithCharacterDataRow {
    // Guild member columns
    pub guid: u32,
    pub rank: u8,
    pub player_note: String,
    pub officer_note: String,

    // Character data columns (from LEFT JOIN - may be NULL)
    pub name: Option<String>,
    pub level: Option<u8>,
    pub class: Option<u8>,
    pub zone: Option<u32>,
    pub account: Option<u32>,
    pub logout_time: Option<u64>,
}

/// Guild rank table row
///
/// Maps to the `guild_rank` table in the characters database.
/// Note: `id` is INT UNSIGNED (u32), not TINYINT.
#[derive(FromRow, Debug, Clone)]
pub struct GuildRankRow {
    pub guild_id: u32,
    /// INT UNSIGNED - rank ID (not tinyint!)
    pub id: u32,
    pub name: String,
    pub rights: u32,
}

/// Guild bank tab table row
///
/// Maps to the `guild_bank_tab` table in the characters database.
/// Contains bank tab configuration and permissions.
#[derive(FromRow, Debug, Clone)]
pub struct GuildBankTabRow {
    pub guild_id: u32,
    pub tab_id: u8,
    pub name: String,
    pub icon: String,
    pub view_rank: u8,
    pub withdraw_rank: u8,
    pub deposit_rank: u8,
}

/// Guild event log table row
///
/// Maps to the `guild_eventlog` table in the characters database.
/// Tracks guild events like member joins, leaves, promotions, etc.
#[derive(FromRow, Debug, Clone)]
pub struct GuildEventLogRow {
    pub guild_id: i32,
    pub log_guid: i32,
    pub event_type: i8,
    pub player_guid1: i32,
    pub player_guid2: i32,
    pub new_rank: i8,
    pub timestamp: i64,
}
