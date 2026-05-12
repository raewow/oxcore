//! Creature database models

use sqlx::FromRow;

/// Row from creature_template table
#[derive(FromRow, Debug, Clone)]
pub struct CreatureTemplateRow {
    pub entry: u32,
    pub name: String,
    pub subname: Option<String>,
    pub level_min: u8,
    pub level_max: u8,
    pub faction: u16,
    pub npc_flags: u32,
    pub display_id1: u32,
    pub display_id2: u32,
    pub display_id3: u32,
    pub display_id4: u32,
    pub display_scale1: f32,
    pub health_multiplier: f32,
    pub mana_multiplier: f32,
    pub armor_multiplier: f32,
    pub damage_multiplier: f32,
    pub damage_variance: f32,
    pub unit_class: u8,
    pub base_attack_time: u32,
    pub static_flags1: u32,
    pub flags_extra: u32,
    pub creature_type: u8, // Maps to 'type' column - needed for critter detection
    pub gossip_menu_id: u32, // Default gossip menu ID
    pub vendor_id: u32, // Maps to npc_vendor_template.entry for shared vendor item lists
    pub trainer_id: u32, // Maps to npc_trainer_template.entry for shared trainer spell lists
    pub trainer_type: u8, // 0=class, 1=mount, 2=tradeskill, 3=pet
    pub rank: u8,
    pub spell_id1: u32,
    pub spell_id2: u32,
    pub spell_id3: u32,
    pub spell_id4: u32,
}

/// Row from creature_classlevelstats table - base stats per class and level
#[derive(FromRow, Debug, Clone)]
pub struct ClassLevelStatsRow {
    pub class: u8,
    pub level: u8,
    pub melee_damage: f32,
    pub ranged_damage: f32,
    pub attack_power: i32,
    pub ranged_attack_power: i32,
    pub health: i32,
    pub base_health: i32,
    pub mana: i32,
    pub base_mana: i32,
    pub armor: i32,
}

/// Row from creature table (spawn data)
#[derive(FromRow, Debug, Clone)]
pub struct CreatureSpawnRow {
    pub guid: u32,
    pub id: u32, // creature entry
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub spawntimesecsmin: u32,
    pub spawntimesecsmax: u32,
    pub wander_distance: f32,
    pub movement_type: u8,
    // Game event and pool filtering (from JOINs)
    pub event: i16,            // game_event (0 = always spawned)
    pub guid_pool_entry: u16,  // pool entry for this GUID
    pub entry_pool_entry: u16, // pool entry for this entry
    pub patch_min: u8,
    pub patch_max: u8,
}
