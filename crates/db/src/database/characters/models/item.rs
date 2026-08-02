use sqlx::FromRow;

/// Item instance table row
///
/// Maps to the `item_instance` table in the characters database.
/// Contains individual item instances with their properties, enchantments, and durability.
#[derive(FromRow, Debug, Clone)]
pub struct ItemInstanceRow {
    pub guid: u32,
    /// MEDIUMINT UNSIGNED - use u32 for proper range
    pub item_id: u32,
    pub owner_guid: u32,
    pub creator_guid: u32,
    pub gift_creator_guid: u32,
    pub count: u32,
    /// INT (signed) - duration can be negative for special cases
    pub duration: i32,
    /// TINYTEXT - charges string (space-separated values)
    pub charges: Option<String>,
    /// MEDIUMINT UNSIGNED
    pub flags: u32,
    /// TEXT - enchantments string
    pub enchantments: String,
    /// SMALLINT (signed) - random property can be negative
    pub random_property_id: i16,
    /// SMALLINT UNSIGNED
    pub durability: u16,
    /// INT UNSIGNED - reference to item_text table
    pub text: u32,
    /// TINYINT - whether loot has been generated
    pub generated_loot: Option<i8>,
}

/// Item loot table row
///
/// Maps to the `item_loot` table in the characters database.
/// Contains loot contents for container items (bags, chests, etc.).
#[derive(FromRow, Debug, Clone)]
pub struct ItemLootRow {
    /// Container item GUID
    pub guid: u32,
    pub owner_guid: u32,
    /// Loot item ID
    pub item_id: u32,
    pub amount: u32,
    /// INT (signed) - property can be negative
    pub property: i32,
}
