//! Quest Manager - state storage and database loading
//!
//! Manages quest templates and quest relations (creature/GO to quest mappings).
//! All data is stored in DashMaps for concurrent access.

use anyhow::Result;
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::info;

use super::types::{QuestRelation, QuestTemplate};
use crate::shared::database::world::repositories::QuestTemplateRepository;

/// Manages quest data (state storage + database loading)
#[derive(Debug)]
pub struct QuestManager {
    /// Database pool for loading
    world_db: Arc<MySqlPool>,
    /// Quest templates by ID
    quest_templates: DashMap<u32, Arc<QuestTemplate>>,
    /// Quest starters by creature entry
    creature_quest_relations: DashMap<u32, Vec<u32>>,
    /// Quest enders by creature entry
    creature_involved_relations: DashMap<u32, Vec<u32>>,
    /// Quest starters by GameObject entry
    go_quest_relations: DashMap<u32, Vec<u32>>,
    /// Quest enders by GameObject entry
    go_involved_relations: DashMap<u32, Vec<u32>>,
}

impl QuestManager {
    /// Create a new quest manager with database pool
    pub fn new(world_db: Arc<MySqlPool>) -> Self {
        Self {
            world_db,
            quest_templates: DashMap::new(),
            creature_quest_relations: DashMap::new(),
            creature_involved_relations: DashMap::new(),
            go_quest_relations: DashMap::new(),
            go_involved_relations: DashMap::new(),
        }
    }

    /// Load all quest data from the database
    pub async fn load(&self) -> Result<()> {
        QuestTemplateRepository::load(self, &self.world_db).await?;

        info!(
            "QuestManager loaded: {} quest templates, {} creature quest starters, {} creature quest enders",
            self.quest_template_count(),
            self.creature_starter_count(),
            self.creature_ender_count()
        );

        Ok(())
    }

    /// Get quest template by ID
    pub fn get_quest_template(&self, quest_id: u32) -> Option<Arc<QuestTemplate>> {
        self.quest_templates.get(&quest_id).map(|q| q.clone())
    }

    /// Get all quest template IDs
    pub fn get_all_quest_ids(&self) -> Vec<u32> {
        self.quest_templates
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Get quests this creature can give (starter quests)
    pub fn get_creature_quest_relations(&self, entry: u32) -> Vec<u32> {
        self.creature_quest_relations
            .get(&entry)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get quests this creature can complete (ender quests)
    pub fn get_creature_involved_relations(&self, entry: u32) -> Vec<u32> {
        self.creature_involved_relations
            .get(&entry)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get quests this GameObject can give
    pub fn get_go_quest_relations(&self, entry: u32) -> Vec<u32> {
        self.go_quest_relations
            .get(&entry)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get quests this GameObject can complete
    pub fn get_go_involved_relations(&self, entry: u32) -> Vec<u32> {
        self.go_involved_relations
            .get(&entry)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Check if a quest template exists
    pub fn has_quest_template(&self, quest_id: u32) -> bool {
        self.quest_templates.contains_key(&quest_id)
    }

    /// Get the number of loaded quest templates
    pub fn quest_template_count(&self) -> usize {
        self.quest_templates.len()
    }

    /// Get the number of creature quest starters
    pub fn creature_starter_count(&self) -> usize {
        self.creature_quest_relations.len()
    }

    /// Get the number of creature quest enders
    pub fn creature_ender_count(&self) -> usize {
        self.creature_involved_relations.len()
    }

    /// Add quest template (used by repository during loading)
    pub fn add_quest_template(&self, template: QuestTemplate) {
        let id = template.id;
        self.quest_templates.insert(id, Arc::new(template));
    }

    /// Add creature quest starter
    pub fn add_creature_quest_starter(&self, entry: u32, quest_id: u32) {
        self.creature_quest_relations
            .entry(entry)
            .or_insert_with(Vec::new)
            .push(quest_id);
    }

    /// Add creature quest ender
    pub fn add_creature_quest_ender(&self, entry: u32, quest_id: u32) {
        self.creature_involved_relations
            .entry(entry)
            .or_insert_with(Vec::new)
            .push(quest_id);
    }

    /// Add GameObject quest starter
    pub fn add_go_quest_starter(&self, entry: u32, quest_id: u32) {
        self.go_quest_relations
            .entry(entry)
            .or_insert_with(Vec::new)
            .push(quest_id);
    }

    /// Add GameObject quest ender
    pub fn add_go_quest_ender(&self, entry: u32, quest_id: u32) {
        self.go_involved_relations
            .entry(entry)
            .or_insert_with(Vec::new)
            .push(quest_id);
    }

    /// Clear all quest data (used during reloads)
    pub fn clear(&self) {
        self.quest_templates.clear();
        self.creature_quest_relations.clear();
        self.creature_involved_relations.clear();
        self.go_quest_relations.clear();
        self.go_involved_relations.clear();
    }

    /// Get all quests that a creature is involved with (both starter and ender)
    pub fn get_creature_all_quests(&self, entry: u32) -> Vec<u32> {
        let mut quests = Vec::new();

        if let Some(starters) = self.creature_quest_relations.get(&entry) {
            quests.extend(starters.iter().copied());
        }

        if let Some(enders) = self.creature_involved_relations.get(&entry) {
            for quest_id in enders.iter() {
                if !quests.contains(quest_id) {
                    quests.push(*quest_id);
                }
            }
        }

        quests
    }
}
