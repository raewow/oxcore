use super::types::{Loot, LootItem, LootTableEntry};
use crate::shared::protocol::ObjectGuid;
use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;

/// Manages loot tables and active loot instances
pub struct LootManager {
    /// Loot tables by creature entry
    creature_loot_tables: DashMap<u32, Vec<LootTableEntry>>,
    /// Active loot instances by source GUID
    active_loot: DashMap<ObjectGuid, Loot>,
}

/// Database row for creature_loot_template
#[derive(sqlx::FromRow, Debug, Clone)]
struct LootTableRow {
    pub entry: u32,
    pub item: u32,
    pub chance: f32,
    pub min_count: i32,
    pub max_count: u8,
    pub group_id: u8,
}

impl LootManager {
    pub fn new() -> Self {
        Self {
            creature_loot_tables: DashMap::new(),
            active_loot: DashMap::new(),
        }
    }

    /// Load loot tables from database
    pub async fn load_loot_tables(&self, world_db: &sqlx::MySqlPool) -> anyhow::Result<()> {
        let rows = sqlx::query_as::<_, LootTableRow>(
            r#"SELECT entry, item, ChanceOrQuestChance as chance,
               CAST(mincountOrRef AS SIGNED) as min_count, maxcount as max_count, groupid as group_id
               FROM creature_loot_template"#
        )
        .fetch_all(world_db)
        .await?;

        for row in rows {
            let entry = LootTableEntry {
                entry: row.entry,
                item: row.item,
                // Negative ChanceOrQuestChance means quest-only drop; store the magnitude as chance
                chance: row.chance.abs(),
                min_count: row.min_count.max(1) as u32,
                max_count: (row.max_count as u32).max(1),
                group_id: row.group_id,
                is_reference: row.min_count < 0, // Negative min_count indicates reference
                is_quest_drop: row.chance < 0.0,
            };

            self.creature_loot_tables
                .entry(entry.entry)
                .or_insert_with(Vec::new)
                .push(entry);
        }

        tracing::info!(
            "Loaded loot tables for {} creature entries",
            self.creature_loot_tables.len()
        );
        Ok(())
    }

    /// Generate loot for a creature
    pub fn generate_creature_loot(
        &self,
        source_guid: ObjectGuid,
        creature_entry: u32,
        creature_level: u8,
        allowed_looters: Vec<ObjectGuid>,
    ) -> bool {
        let Some(table) = self.creature_loot_tables.get(&creature_entry) else {
            // No loot table - still create empty loot for gold
            let mut loot = Loot::new();
            loot.gold = calculate_gold_drop(creature_level);
            loot.allowed_looters = allowed_looters;
            loot.generated = true;
            self.active_loot.insert(source_guid, loot);
            return true;
        };

        let mut loot = Loot::new();
        let mut slot: u8 = 0;
        let mut quest_slot: u8 = 0;

        // Process loot table entries
        for entry in table.iter() {
            // Skip reference entries for now (Phase 7 basic implementation)
            if entry.is_reference {
                continue;
            }

            // Roll for item
            let roll: f32 = rand::random::<f32>() * 100.0;
            if roll <= entry.chance {
                // Won the roll
                let count = if entry.min_count == entry.max_count {
                    entry.min_count
                } else {
                    rand::thread_rng().gen_range(entry.min_count..=entry.max_count)
                };

                if entry.is_quest_drop {
                    // Quest drops go into a separate list; filtered per-player when showing loot
                    loot.quest_items.push(LootItem {
                        slot: quest_slot,
                        item_id: entry.item,
                        count,
                        is_looted: false,
                        is_blocked: false,
                        roll_winner: None,
                    });
                    quest_slot += 1;
                } else {
                    loot.items.push(LootItem {
                        slot,
                        item_id: entry.item,
                        count,
                        is_looted: false,
                        is_blocked: false,
                        roll_winner: None,
                    });
                    slot += 1;
                }
            }
        }

        // Generate gold
        loot.gold = calculate_gold_drop(creature_level);
        loot.allowed_looters = allowed_looters;
        loot.generated = true;

        self.active_loot.insert(source_guid, loot);
        true
    }

    /// Check if loot exists for a source
    pub fn has_loot(&self, source: ObjectGuid) -> bool {
        self.active_loot.contains_key(&source)
    }

    /// Get loot for a source
    pub fn get_loot(
        &self,
        source: ObjectGuid,
    ) -> Option<dashmap::mapref::one::Ref<'_, ObjectGuid, Loot>> {
        self.active_loot.get(&source)
    }

    /// Get mutable loot
    pub fn get_loot_mut(
        &self,
        source: ObjectGuid,
    ) -> Option<dashmap::mapref::one::RefMut<'_, ObjectGuid, Loot>> {
        self.active_loot.get_mut(&source)
    }

    /// Remove loot when corpse despawns
    pub fn remove_loot(&self, source: ObjectGuid) {
        self.active_loot.remove(&source);
    }

    /// Set looting state
    pub fn set_looting(&self, source: ObjectGuid, player: ObjectGuid) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.being_looted = true;
            loot.looting_player = Some(player);
        }
    }

    /// Clear looting state
    pub fn clear_looting(&self, source: ObjectGuid) {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.being_looted = false;
            loot.looting_player = None;
        }
    }

    /// Take gold from loot
    pub fn take_gold(&self, source: ObjectGuid) -> u32 {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            loot.take_gold()
        } else {
            0
        }
    }

    /// Loot an item by slot.
    ///
    /// Quest items are presented to the client with a slot offset equal to the number of
    /// normal items, so we first check normal items. If not found, we subtract the offset
    /// and check quest items.
    pub fn loot_item(&self, source: ObjectGuid, slot: u8) -> Option<LootItem> {
        if let Some(mut loot) = self.active_loot.get_mut(&source) {
            // Try normal items first
            if let Some(item) = loot.loot_item(slot) {
                return Some(item);
            }
            // Try quest items using offset slot
            let normal_count = loot.items.len() as u8;
            if slot >= normal_count {
                let quest_slot = slot - normal_count;
                return loot.loot_quest_item(quest_slot);
            }
            None
        } else {
            None
        }
    }

    /// Check if loot is empty
    pub fn is_loot_empty(&self, source: ObjectGuid) -> bool {
        if let Some(loot) = self.active_loot.get(&source) {
            loot.is_empty()
        } else {
            true
        }
    }

    /// Execute closure with mutable loot access
    pub fn with_loot_mut<F, R>(&self, source: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut Loot) -> R,
    {
        self.active_loot
            .get_mut(&source)
            .map(|mut loot| f(&mut *loot))
    }
}

/// Calculate gold drop based on creature level
fn calculate_gold_drop(level: u8) -> u32 {
    // Simple formula: base + level * multiplier + random variance
    let base = 5u32;
    let multiplier = 3u32;
    let variance = rand::thread_rng().gen_range(0..=level as u32);

    base + (level as u32 * multiplier) + variance
}

impl Default for LootManager {
    fn default() -> Self {
        Self::new()
    }
}
