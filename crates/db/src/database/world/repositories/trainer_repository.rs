//! Trainer repository for database access
//!
//! Handles loading of trainer spells from npc_trainer and npc_trainer_template tables.

use anyhow::{Context, Result};
use sqlx::PgPool;
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
    pool: Arc<PgPool>,
}

impl TrainerRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Load all spells from npc_trainer (direct per-creature)
    pub async fn load_trainer_spells(&self) -> Result<Vec<TrainerSpellRow>> {
        let rows = sqlx::query(
            r#"SELECT entry, spell::BIGINT AS spell, spellcost,
               reqskill::SMALLINT AS reqskill, reqskillvalue::SMALLINT AS reqskillvalue, reqlevel
               FROM world.npc_trainer
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
                entry: row.get::<i64, _>("entry").try_into()?,
                spell: row.get::<i64, _>("spell").try_into()?,
                spellcost: row.get::<i64, _>("spellcost").try_into()?,
                reqskill: row.get::<i16, _>("reqskill").try_into()?,
                reqskillvalue: row.get::<i16, _>("reqskillvalue").try_into()?,
                reqlevel: row.get::<i16, _>("reqlevel").try_into()?,
            });
        }
        Ok(result)
    }

    /// Load all spells from npc_trainer_template (shared lists)
    pub async fn load_trainer_template_spells(&self) -> Result<Vec<TrainerTemplateSpellRow>> {
        let rows = sqlx::query(
            r#"SELECT entry, spell::BIGINT AS spell, spellcost,
               reqskill::SMALLINT AS reqskill, reqskillvalue::SMALLINT AS reqskillvalue, reqlevel
               FROM world.npc_trainer_template
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
                entry: row.get::<i64, _>("entry").try_into()?,
                spell: row.get::<i64, _>("spell").try_into()?,
                spellcost: row.get::<i64, _>("spellcost").try_into()?,
                reqskill: row.get::<i16, _>("reqskill").try_into()?,
                reqskillvalue: row.get::<i16, _>("reqskillvalue").try_into()?,
                reqlevel: row.get::<i16, _>("reqlevel").try_into()?,
            });
        }
        Ok(result)
    }
}
