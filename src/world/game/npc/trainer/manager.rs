//! Trainer Manager - state storage and database loading

use anyhow::Result;
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::info;

use super::types::TrainerSpell;
use crate::shared::database::world::repositories::TrainerRepository;

/// Manages trainer spell data loaded from the database
pub struct TrainerManager {
    world_db: Arc<MySqlPool>,
    /// Direct spells per creature entry (from npc_trainer)
    direct_spells: DashMap<u32, Vec<TrainerSpell>>,
    /// Template spells per template ID (from npc_trainer_template)
    template_spells: DashMap<u32, Vec<TrainerSpell>>,
    /// trainer_id per creature entry (registered from creature_template)
    creature_trainer_templates: DashMap<u32, u32>,
}

impl TrainerManager {
    pub fn new(world_db: Arc<MySqlPool>) -> Self {
        Self {
            world_db,
            direct_spells: DashMap::new(),
            template_spells: DashMap::new(),
            creature_trainer_templates: DashMap::new(),
        }
    }

    pub async fn load(&self) -> Result<()> {
        let repo = TrainerRepository::new(Arc::clone(&self.world_db));

        // Load direct spells (npc_trainer)
        let direct = repo.load_trainer_spells().await?;
        for row in &direct {
            self.direct_spells
                .entry(row.entry)
                .or_insert_with(Vec::new)
                .push(TrainerSpell {
                    spell_id: row.spell,
                    cost: row.spellcost,
                    req_skill: row.reqskill,
                    req_skill_value: row.reqskillvalue,
                    req_level: row.reqlevel,
                });
        }

        // Load template spells (npc_trainer_template)
        let templates = repo.load_trainer_template_spells().await?;
        for row in &templates {
            self.template_spells
                .entry(row.entry)
                .or_insert_with(Vec::new)
                .push(TrainerSpell {
                    spell_id: row.spell,
                    cost: row.spellcost,
                    req_skill: row.reqskill,
                    req_skill_value: row.reqskillvalue,
                    req_level: row.reqlevel,
                });
        }

        info!(
            "TrainerManager loaded: {} trainer entries, {} template entries",
            self.direct_spells.len(),
            self.template_spells.len()
        );

        Ok(())
    }

    /// Register a creature's trainer template ID (from creature_template.trainer_id)
    pub fn register_creature_trainer_template(&self, creature_entry: u32, trainer_id: u32) {
        if trainer_id > 0 {
            self.creature_trainer_templates
                .insert(creature_entry, trainer_id);
        }
    }

    /// Get all spells offered by a trainer (combines direct + template)
    pub fn get_trainer_spells(&self, entry: u32) -> Vec<TrainerSpell> {
        let mut spells = Vec::new();

        // Direct spells for this creature entry
        if let Some(direct) = self.direct_spells.get(&entry) {
            spells.extend(direct.iter().cloned());
        }

        // Template spells via trainer_id
        if let Some(template_id) = self.creature_trainer_templates.get(&entry) {
            if let Some(tmpl) = self.template_spells.get(&*template_id) {
                spells.extend(tmpl.iter().cloned());
            }
        }

        spells
    }

    /// Check if a creature entry has any trainer spells
    pub fn is_trainer(&self, entry: u32) -> bool {
        self.direct_spells.contains_key(&entry)
            || self.creature_trainer_templates.contains_key(&entry)
    }
}
