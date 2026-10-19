//! Creature repository for database access

use super::super::models::creature::*;
use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;

pub struct CreatureRepository {
    pool: Arc<PgPool>,
}

impl CreatureRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Load all creature templates from database
    pub async fn load_all_templates(&self) -> Result<Vec<CreatureTemplateRow>> {
        let result = sqlx::query_as::<_, CreatureTemplateRow>(
            r#"SELECT entry, name, subname, level_min::BIGINT AS level_min, level_max::BIGINT AS level_max,
                       faction::BIGINT AS faction, npc_flags,
                       display_id1, display_id2, display_id3, display_id4,
                       display_scale1, display_scale2, display_scale3, display_scale4,
                       display_probability1::BIGINT AS display_probability1,
                       display_probability2::BIGINT AS display_probability2,
                       display_probability3::BIGINT AS display_probability3,
                       display_probability4::BIGINT AS display_probability4,
                      health_multiplier, mana_multiplier, armor_multiplier, damage_multiplier,
                       damage_variance, unit_class::BIGINT AS unit_class,
                       base_attack_time, static_flags1, static_flags2, inhabit_type::BIGINT AS inhabit_type, flags_extra, type::BIGINT AS creature_type,
                       gossip_menu_id, vendor_id, trainer_id, trainer_type::BIGINT AS trainer_type,
                       trainer_spell::BIGINT AS trainer_spell, trainer_class::BIGINT AS trainer_class,
                       trainer_race::BIGINT AS trainer_race,
                       "rank"::BIGINT AS rank, spell_id1::BIGINT AS spell_id1,
                       spell_id2::BIGINT AS spell_id2, spell_id3::BIGINT AS spell_id3,
                       spell_id4::BIGINT AS spell_id4, gold_min, gold_max,
                       loot_id, pickpocket_loot_id, skinning_loot_id,
                       TRIM(ai_name) AS ai_name, TRIM(script_name) AS script_name,
                       spell_list_id::BIGINT AS spell_list_id,
                       detection_range::REAL AS detection_range,
                       call_for_help_range::REAL AS call_for_help_range,
                       leash_range::REAL AS leash_range, civilian::BIGINT AS civilian, racial_leader::BIGINT AS racial_leader
               FROM world.creature_template
               WHERE patch = 0"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature templates")?;
        Ok(result)
    }

    /// Load creature class level stats from database
    pub async fn load_class_level_stats(&self) -> Result<Vec<ClassLevelStatsRow>> {
        let result = sqlx::query_as::<_, ClassLevelStatsRow>(
            r#"SELECT class, level, melee_damage, ranged_damage,
                      attack_power, ranged_attack_power,
                      health, base_health, mana, base_mana, armor
               FROM world.creature_classlevelstats"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature class level stats")?;
        Ok(result)
    }

    /// Load every creature_spells list (eight slots of eleven columns each).
    pub async fn load_creature_spells(&self) -> Result<Vec<CreatureSpellsListRow>> {
        use sqlx::Row;
        const COLUMNS: [&str; 11] = [
            "spellId",
            "probability",
            "castTarget",
            "targetParam1",
            "targetParam2",
            "castFlags",
            "delayInitialMin",
            "delayInitialMax",
            "delayRepeatMin",
            "delayRepeatMax",
            "scriptId",
        ];
        let mut select = String::from("SELECT entry::BIGINT AS entry");
        for slot in 1..=8 {
            for column in COLUMNS {
                select.push_str(&format!(", \"{column}_{slot}\"::BIGINT AS {column}_{slot}"));
            }
        }
        select.push_str(" FROM world.creature_spells");

        let rows = sqlx::query(&select)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load creature spell lists")?;
        let mut lists = Vec::with_capacity(rows.len());
        for row in rows {
            let get = |column: &str, slot: u32| -> Result<i64> {
                Ok(row.try_get::<i64, _>(format!("{column}_{slot}").to_lowercase().as_str())?)
            };
            let mut spells = Vec::new();
            for slot in 1..=8 {
                let spell_id = get("spellId", slot)?;
                if spell_id <= 0 {
                    continue;
                }
                spells.push(CreatureSpellRow {
                    spell_id: spell_id as u32,
                    probability: get("probability", slot)?.clamp(0, 255) as u8,
                    cast_target: get("castTarget", slot)?.clamp(0, 255) as u8,
                    target_param1: get("targetParam1", slot)? as u32,
                    target_param2: get("targetParam2", slot)? as u32,
                    cast_flags: get("castFlags", slot)?.clamp(0, u16::MAX as i64) as u16,
                    delay_initial_min: get("delayInitialMin", slot)? as u32,
                    delay_initial_max: get("delayInitialMax", slot)? as u32,
                    delay_repeat_min: get("delayRepeatMin", slot)? as u32,
                    delay_repeat_max: get("delayRepeatMax", slot)? as u32,
                    script_id: get("scriptId", slot)? as u32,
                });
            }
            lists.push(CreatureSpellsListRow {
                entry: row.try_get::<i64, _>("entry")? as u32,
                spells,
            });
        }
        Ok(lists)
    }

    /// Load all creature spawns from database
    pub async fn load_all_spawns(&self) -> Result<Vec<CreatureSpawnRow>> {
        sqlx::query_as::<_, CreatureSpawnRow>(
            r#"SELECT creature.guid, creature.id, creature.map::BIGINT AS map,
                      creature.position_x, creature.position_y, creature.position_z, creature.orientation,
                      creature.spawntimesecsmin, creature.spawntimesecsmax, creature.wander_distance,
                       creature.movement_type::BIGINT AS movement_type,
                       creature.spawn_flags::BIGINT AS spawn_flags,
                        COALESCE(game_event_creature.event, 0)::SMALLINT AS event,
                        COALESCE(pool_creature.pool_entry, 0)::BIGINT AS guid_pool_entry,
                        COALESCE(pool_creature_template.pool_entry, 0)::BIGINT AS entry_pool_entry,
                       creature.patch_min::BIGINT AS patch_min, creature.patch_max::BIGINT AS patch_max
                FROM world.creature
                LEFT OUTER JOIN world.game_event_creature ON creature.guid = game_event_creature.guid
                LEFT OUTER JOIN world.pool_creature ON creature.guid = pool_creature.guid
                LEFT OUTER JOIN world.pool_creature_template ON creature.id = pool_creature_template.id"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature spawns")
    }
}
