//! Creature repository for database access

use super::super::models::creature::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct CreatureRepository {
    pool: Arc<MySqlPool>,
}

impl CreatureRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load all creature templates from database
    pub async fn load_all_templates(&self) -> Result<Vec<CreatureTemplateRow>> {
        let result = sqlx::query_as::<_, CreatureTemplateRow>(
            r#"SELECT entry, name, subname, level_min, level_max, faction, npc_flags,
                      display_id1, display_id2, display_id3, display_id4, display_scale1,
                      health_multiplier, mana_multiplier, armor_multiplier, damage_multiplier,
                      damage_variance, unit_class,
                      base_attack_time, static_flags1, flags_extra, `type` as creature_type,
                      gossip_menu_id, vendor_id, trainer_id, trainer_type,
                      `rank`, spell_id1, spell_id2, spell_id3, spell_id4
               FROM creature_template
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
            r#"SELECT `class`, level, melee_damage, ranged_damage,
                      attack_power, ranged_attack_power,
                      health, base_health, mana, base_mana, armor
               FROM creature_classlevelstats"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature class level stats")?;
        Ok(result)
    }

    /// Load all creature spawns from database
    pub async fn load_all_spawns(&self) -> Result<Vec<CreatureSpawnRow>> {
        sqlx::query_as::<_, CreatureSpawnRow>(
            r#"SELECT creature.guid, creature.id, creature.map,
                      creature.position_x, creature.position_y, creature.position_z, creature.orientation,
                      creature.spawntimesecsmin, creature.spawntimesecsmax, creature.wander_distance,
                      creature.movement_type,
                      COALESCE(game_event_creature.event, CAST(0 AS SIGNED)) as event,
                      COALESCE(pool_creature.pool_entry, CAST(0 AS UNSIGNED)) as guid_pool_entry,
                      COALESCE(pool_creature_template.pool_entry, CAST(0 AS UNSIGNED)) as entry_pool_entry,
                      creature.patch_min, creature.patch_max
               FROM creature
               LEFT OUTER JOIN game_event_creature ON creature.guid = game_event_creature.guid
               LEFT OUTER JOIN pool_creature ON creature.guid = pool_creature.guid
               LEFT OUTER JOIN pool_creature_template ON creature.id = pool_creature_template.id"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load creature spawns")
    }
}
