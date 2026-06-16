//! Trainer repository for database access
//!
//! Handles loading of trainer spells from npc_trainer and npc_trainer_template tables.

use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

/// Row from npc_trainer table (direct per-creature spells)
pub struct TrainerSpellRow {
    pub entry: u32,
    pub spell: u32,
    pub spellcost: u32,
    pub reqskill: u16,
    pub reqskillvalue: u16,
    pub reqlevel: u8,
}

/// Row from npc_trainer_template table (shared spell lists via trainer_id)
pub struct TrainerTemplateSpellRow {
    pub entry: u32,
    pub spell: u32,
    pub spellcost: u32,
    pub reqskill: u16,
    pub reqskillvalue: u16,
    pub reqlevel: u8,
}

pub struct TrainerRepository {
    pool: Arc<MySqlPool>,
}

impl TrainerRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load all spells from npc_trainer (direct per-creature)
    pub async fn load_trainer_spells(&self) -> Result<Vec<TrainerSpellRow>> {
        let rows = sqlx::query(
            r#"SELECT entry, spell, spellcost, reqskill, reqskillvalue, reqlevel
               FROM npc_trainer
               WHERE build_min <= 5875 AND build_max >= 5875
               ORDER BY entry, spell"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load npc_trainer")?;

        let mut result = Vec::new();
        for row in rows {
            use sqlx::Row;
            result.push(TrainerSpellRow {
                entry: row.get("entry"),
                spell: row.get::<u16, _>("spell") as u32,
                spellcost: row.get("spellcost"),
                reqskill: row.get("reqskill"),
                reqskillvalue: row.get("reqskillvalue"),
                reqlevel: row.get("reqlevel"),
            });
        }
        Ok(result)
    }

    /// Load all spells from npc_trainer_template (shared lists)
    pub async fn load_trainer_template_spells(&self) -> Result<Vec<TrainerTemplateSpellRow>> {
        let rows = sqlx::query(
            r#"SELECT entry, spell, spellcost, reqskill, reqskillvalue, reqlevel
               FROM npc_trainer_template
               WHERE build_min <= 5875 AND build_max >= 5875
               ORDER BY entry, spell"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load npc_trainer_template")?;

        let mut result = Vec::new();
        for row in rows {
            use sqlx::Row;
            result.push(TrainerTemplateSpellRow {
                entry: row.get("entry"),
                spell: row.get::<u16, _>("spell") as u32,
                spellcost: row.get("spellcost"),
                reqskill: row.get("reqskill"),
                reqskillvalue: row.get("reqskillvalue"),
                reqlevel: row.get("reqlevel"),
            });
        }
        Ok(result)
    }
}
