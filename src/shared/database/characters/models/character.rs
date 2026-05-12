use sqlx::FromRow;

/// Character table row
///
/// Maps to the `characters` table in the characters database.
/// Contains all core character data including position, stats, customization, and flags.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterRow {
    pub guid: u32,
    pub account: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_hair: u8,
    pub level: u8,
    pub xp: u32,
    pub money: u32,
    pub character_flags: u32,
    pub zone: u32,
    pub map: u32,
    pub instance: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub transport_guid: u64,
    pub transport_x: f32,
    pub transport_y: f32,
    pub transport_z: f32,
    pub transport_o: f32,
    pub known_taxi_mask: Option<String>,
    pub current_taxi_path: Option<String>,
    pub online: u8,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub create_time: u64,
    pub logout_time: u64,
    pub rest_bonus: f32,
    pub reset_talents_multiplier: u32,
    pub reset_talents_time: u64,
    pub death_expire_time: u64,
    pub stable_slots: u8,
    pub bank_bag_slots: u8,
    pub extra_flags: u32,
    pub honor_rank_points: f32,
    pub honor_highest_rank: u32,
    pub honor_standing: u32,
    pub honor_last_week_hk: u32,
    pub honor_last_week_cp: f32,
    pub honor_stored_hk: i32,
    pub honor_stored_dk: i32,
    pub watched_faction: i32,
    pub drunk: u16,
    pub health: u32,
    pub power1: u32,
    pub power2: u32,
    pub power3: u32,
    pub power4: u32,
    pub power5: u32,
    pub explored_zones: Option<String>,
    pub equipment_cache: Option<String>,
    pub ammo_id: u32,
    pub action_bars: u8,
    pub deleted_account: Option<u32>,
    pub deleted_name: Option<String>,
    pub deleted_time: Option<i64>,
    pub world_phase_mask: Option<i32>,
}

/// Character spell table row
///
/// Maps to the `character_spell` table in the characters database.
/// Contains learned spells for each character.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterSpellRow {
    pub guid: u32,
    pub spell: u32,
    pub active: u8,
    pub disabled: u8,
}

/// Character aura table row
///
/// Maps to the `character_aura` table in the characters database.
/// Contains active buffs/debuffs on the character.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterAuraRow {
    pub guid: u32,
    pub caster_guid: u64,
    pub item_guid: u32,
    pub spell: u32,
    pub stacks: u32,
    pub charges: u32,
    pub base_points0: f32,
    pub base_points1: f32,
    pub base_points2: f32,
    pub periodic_time0: u32,
    pub periodic_time1: u32,
    pub periodic_time2: u32,
    pub max_duration: i32,
    pub duration: i32,
    pub effect_index_mask: u8,
}

/// Character inventory table row
///
/// Maps to the `character_inventory` table in the characters database.
/// Contains equipped items and bag slot assignments.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterInventoryRow {
    pub guid: u32,
    pub bag: u32,
    pub slot: u8,
    pub item_guid: u32,
    pub item_id: u32,
}

/// Character skills table row
///
/// Maps to the `character_skills` table in the characters database.
/// Contains skill values (e.g., weapon skills, professions).
#[derive(FromRow, Debug, Clone)]
pub struct CharacterSkillRow {
    pub guid: u32,
    /// MEDIUMINT UNSIGNED - use u32 for proper range
    pub skill: u32,
    /// MEDIUMINT UNSIGNED
    pub value: u32,
    /// MEDIUMINT UNSIGNED
    pub max: u32,
}

/// Character reputation table row
///
/// Maps to the `character_reputation` table in the characters database.
/// Contains faction standing values.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterReputationRow {
    pub guid: u32,
    pub faction: u32,
    /// INT (signed) - reputation standing can be negative
    pub standing: i32,
    /// INT (signed)
    pub flags: i32,
}

/// Character action table row
///
/// Maps to the `character_action` table in the characters database.
/// Contains action bar button assignments.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterActionRow {
    pub guid: u32,
    pub button: u8,
    pub action: u32,
    pub r#type: u8,
}

/// Character quest status table row
///
/// Maps to the `character_queststatus` table in the characters database.
/// Contains quest progress and completion tracking.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterQuestStatusRow {
    pub guid: u32,
    pub quest: u32,
    pub status: u32,
    pub rewarded: u8,
    pub explored: u8,
    pub timer: u64,
    pub mob_count1: u32,
    pub mob_count2: u32,
    pub mob_count3: u32,
    pub mob_count4: u32,
    pub item_count1: u32,
    pub item_count2: u32,
    pub item_count3: u32,
    pub item_count4: u32,
    pub reward_choice: u32,
}

/// Character homebind table row
///
/// Maps to the `character_homebind` table in the characters database.
/// Contains hearthstone bind location.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterHomebindRow {
    pub guid: u32,
    pub map: u32,
    pub zone: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
}

/// Character spell cooldown table row
///
/// Maps to the `character_spell_cooldown` table in the characters database.
/// Contains spell cooldown tracking.
#[derive(FromRow, Debug, Clone)]
pub struct CharacterSpellCooldownRow {
    pub guid: u32,
    pub spell: u32,
    pub spell_expire_time: u64,
    pub category: u32,
    pub category_expire_time: u64,
    pub item_id: u32,
}
