use sqlx::FromRow;

/// Groups table row
///
/// Maps to the `groups` table in the characters database.
/// Contains core group/raid data including leader, loot settings, and raid markers.
#[derive(FromRow, Debug, Clone)]
pub struct GroupRow {
    pub group_id: u32,
    pub leader_guid: u32,
    pub main_tank_guid: u32,
    pub main_assistant_guid: u32,
    pub loot_method: u8,
    pub loot_threshold: u8,
    pub looter_guid: u32,
    /// Raid target icon 1
    pub icon1: u32,
    /// Raid target icon 2
    pub icon2: u32,
    /// Raid target icon 3
    pub icon3: u32,
    /// Raid target icon 4
    pub icon4: u32,
    /// Raid target icon 5
    pub icon5: u32,
    /// Raid target icon 6
    pub icon6: u32,
    /// Raid target icon 7
    pub icon7: u32,
    /// Raid target icon 8
    pub icon8: u32,
    /// Whether this group is a raid (vs 5-man party)
    pub is_raid: u8,
}

/// Group member table row
///
/// Maps to the `group_member` table in the characters database.
/// Contains group membership information.
#[derive(FromRow, Debug, Clone)]
pub struct GroupMemberRow {
    pub group_id: u32,
    pub member_guid: u32,
    pub assistant: u8,
    /// SMALLINT UNSIGNED
    pub subgroup: u16,
}

/// Group member with character data (from JOIN query)
///
/// Result of LEFT JOIN between group_member and characters tables.
/// Character fields are Option<T> because they may be NULL if character was deleted.
#[derive(FromRow, Debug, Clone)]
pub struct GroupMemberWithCharacterDataRow {
    // Group member columns
    pub member_guid: u32,
    pub assistant: u8,
    pub subgroup: u16,

    // Character data columns (from LEFT JOIN - may be NULL)
    pub name: Option<String>,
    pub level: Option<u8>,
    pub class: Option<u8>,
    pub zone: Option<u32>,
    pub online: Option<u8>,
}
