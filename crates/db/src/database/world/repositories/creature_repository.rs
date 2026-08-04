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
                       base_attack_time, static_flags1, flags_extra, type::BIGINT AS creature_type,
                       gossip_menu_id, vendor_id, trainer_id, trainer_type::BIGINT AS trainer_type,
                       "rank"::BIGINT AS rank, spell_id1::BIGINT AS spell_id1,
                       spell_id2::BIGINT AS spell_id2, spell_id3::BIGINT AS spell_id3,
                       spell_id4::BIGINT AS spell_id4, gold_min, gold_max
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

    /// Load all creature spawns from database
    pub async fn load_all_spawns(&self) -> Result<Vec<CreatureSpawnRow>> {
        sqlx::query_as::<_, CreatureSpawnRow>(
            r#"SELECT creature.guid, creature.id, creature.map::BIGINT AS map,
                      creature.position_x, creature.position_y, creature.position_z, creature.orientation,
                      creature.spawntimesecsmin, creature.spawntimesecsmax, creature.wander_distance,
                       creature.movement_type::BIGINT AS movement_type,
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
