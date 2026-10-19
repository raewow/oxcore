//! Creature database models

use sqlx::FromRow;

/// Row from creature_template table
#[derive(FromRow, Debug, Clone)]
pub struct CreatureTemplateRow {
    pub entry: i64,
    pub name: String,
    pub subname: Option<String>,
    pub level_min: i64,
    pub level_max: i64,
    pub faction: i64,
    pub npc_flags: i64,
    pub display_id1: i64,
    pub display_id2: i64,
    pub display_id3: i64,
    pub display_id4: i64,
    pub display_scale1: f32,
    pub display_scale2: f32,
    pub display_scale3: f32,
    pub display_scale4: f32,
    pub display_probability1: i64,
    pub display_probability2: i64,
    pub display_probability3: i64,
    pub display_probability4: i64,
    pub health_multiplier: f32,
    pub mana_multiplier: f32,
    pub armor_multiplier: f32,
    pub damage_multiplier: f32,
    pub damage_variance: f32,
    pub unit_class: i64,
    pub base_attack_time: i64,
    pub static_flags1: i64,
    pub static_flags2: i64,
    pub inhabit_type: i64,
    pub flags_extra: i64,
    pub creature_type: i64, // Maps to 'type' column - needed for critter detection
    pub gossip_menu_id: i64, // Default gossip menu ID
    pub vendor_id: i64,     // Maps to npc_vendor_template.entry for shared vendor item lists
    pub trainer_id: i64,    // Maps to npc_trainer_template.entry for shared trainer spell lists
    pub trainer_type: i64,  // 0=class, 1=mount, 2=tradeskill, 3=pet
    pub rank: i64,
    pub spell_id1: i64,
    pub spell_id2: i64,
    pub spell_id3: i64,
    pub spell_id4: i64,
    pub gold_min: i64,
    pub gold_max: i64,
    pub loot_id: i64,
    pub pickpocket_loot_id: i64,
    pub skinning_loot_id: i64,
    pub ai_name: String,
    pub script_name: String,
    pub spell_list_id: i64,
    pub detection_range: f32,
    pub call_for_help_range: f32,
    pub leash_range: f32,
    pub civilian: i64,
    pub racial_leader: i64,
}

/// Row from creature_classlevelstats table - base stats per class and level
#[derive(FromRow, Debug, Clone)]
pub struct ClassLevelStatsRow {
    pub class: i16,
    pub level: i16,
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
    pub guid: i64,
    pub id: i64, // creature entry
    pub map: i64,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub spawntimesecsmin: i64,
    pub spawntimesecsmax: i64,
    pub wander_distance: f32,
    pub movement_type: i64,
    pub spawn_flags: i64,
    // Game event and pool filtering (from JOINs)
    pub event: i16,            // game_event (0 = always spawned)
    pub guid_pool_entry: i64,  // pool entry for this GUID
    pub entry_pool_entry: i64, // pool entry for this entry
    pub patch_min: i64,
    pub patch_max: i64,
}

/// One spell slot of a creature_spells list.
#[derive(Debug, Clone)]
pub struct CreatureSpellRow {
    pub spell_id: u32,
    pub probability: u8,
    pub cast_target: u8,
    pub target_param1: u32,
    pub target_param2: u32,
    pub cast_flags: u16,
    pub delay_initial_min: u32,
    pub delay_initial_max: u32,
    pub delay_repeat_min: u32,
    pub delay_repeat_max: u32,
    pub script_id: u32,
}

/// A creature_spells list: up to eight spells cast in combat.
#[derive(Debug, Clone)]
pub struct CreatureSpellsListRow {
    pub entry: u32,
    pub spells: Vec<CreatureSpellRow>,
}
