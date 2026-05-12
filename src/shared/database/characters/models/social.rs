use sqlx::FromRow;

/// Character social table row
///
/// Maps to the `character_social` table in the characters database.
/// Contains friends and ignore list entries.
/// Flags indicate relationship type: 0x01 = friend, 0x02 = ignore
#[derive(FromRow, Debug, Clone)]
pub struct CharacterSocialRow {
    /// Character GUID (the one who has the friend/ignore entry)
    pub guid: u32,
    /// Friend/ignored character GUID
    pub friend: u32,
    /// Relationship flags (0x01 = friend, 0x02 = ignore)
    pub flags: u8,
}
