use super::types::{Loot, LootItem, LootTableEntry};
use anyhow::Context;
use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;
use std::sync::Arc;

/// Manages loot tables and active loot instances
pub struct LootManager {
    /// Loot tables by creature entry
    creature_loot_tables: DashMap<u32, Vec<LootTableEntry>>,
    /// Loot tables by gameobject loot ID
    gameobject_loot_tables: DashMap<u32, Vec<LootTableEntry>>,
    /// Active loot instances by source GUID
    active_loot: DashMap<ObjectGuid, Loot>,
}

/// Database row for creature_loot_template
#[derive(sqlx::FromRow, Debug, Clone)]
struct LootTableRow {
    pub entry: i64,
    pub item: i64,
    pub chance: f32,
    pub min_count: i32,
    pub max_count: i16,
    pub group_id: i16,
}

impl LootTableRow {
    fn into_entry(self) -> anyhow::Result<LootTableEntry> {
        let entry = u32::try_from(self.entry).context("loot entry is outside u32 range")?;
        let item = u32::try_from(self.item).context("loot item is outside u32 range")?;
        let max_count = u32::try_from(self.max_count)
            .context("loot max_count must not be negative")?
            .max(1);
        let group_id = u8::try_from(self.group_id).context("loot group_id is outside u8 range")?;

        Ok(LootTableEntry {
            entry,
            item,
            chance: self.chance.abs(),
            min_count: self.min_count.max(1) as u32,
            max_count,
            group_id,
            is_reference: self.min_count < 0,
            is_quest_drop: self.chance < 0.0,
        })
    }
}

impl LootManager {
    pub fn new() -> Self {
        Self {
            creature_loot_tables: DashMap::new(),
            gameobject_loot_tables: DashMap::new(),
            active_loot: DashMap::new(),
        }
    }

    /// Load loot tables from database
    pub async fn load_loot_tables(&self, world_db: &sqlx::PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query_as::<_, LootTableRow>(
            r#"SELECT entry, item, "ChanceOrQuestChance" AS chance,
                "mincountOrRef" AS min_count, maxcount AS max_count, groupid AS group_id
                FROM world.creature_loot_template"#,
        )
        .fetch_all(world_db)
        .await?;

        for row in rows {
            let entry = row.into_entry()?;

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

    /// Load gameobject loot tables from the world database.
    pub async fn load_gameobject_loot_tables(&self, world_db: &sqlx::PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query_as::<_, LootTableRow>(
            r#"SELECT entry, item, "ChanceOrQuestChance" AS chance,
                "mincountOrRef" AS min_count, maxcount AS max_count, groupid AS group_id
                FROM world.gameobject_loot_template"#,
        )
        .fetch_all(world_db)
        .await?;

        for row in rows {
            let entry = row.into_entry()?;
            self.gameobject_loot_tables
                .entry(entry.entry)
                .or_insert_with(Vec::new)
                .push(entry);
        }

        tracing::info!(
            "Loaded loot tables for {} gameobject loot IDs",
            self.gameobject_loot_tables.len()
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
                    loot.add_quest_item(LootItem {
                        slot: quest_slot,
                        item_id: entry.item,
                        count,
                        is_looted: false,
                        is_blocked: false,
                        is_counted: false,
                        roll_winner: None,
                    });
                    quest_slot += 1;
                } else {
                    loot.add_item(LootItem {
                        slot,
                        item_id: entry.item,
                        count,
                        is_looted: false,
                        is_blocked: false,
                        is_counted: false,
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

    /// Generate loot for a gameobject chest. Unlike creatures, chests do not
    /// generate a level-based gold drop.
    pub fn generate_gameobject_loot(&self, source_guid: ObjectGuid, loot_id: u32) -> bool {
        let Some(table) = self.gameobject_loot_tables.get(&loot_id) else {
            self.active_loot.insert(
                source_guid,
                Loot {
                    generated: true,
                    ..Loot::new()
                },
            );
            return true;
        };

        let mut loot = Loot::new();
        let mut slot: u8 = 0;
        let mut quest_slot: u8 = 0;
        for entry in table.iter() {
            if entry.is_reference || rand::random::<f32>() * 100.0 > entry.chance {
                continue;
            }

            let count = if entry.min_count == entry.max_count {
                entry.min_count
            } else {
                rand::thread_rng().gen_range(entry.min_count..=entry.max_count)
            };
            let item = LootItem {
                slot: if entry.is_quest_drop {
                    quest_slot
                } else {
                    slot
                },
                item_id: entry.item,
                count,
                is_looted: false,
                is_blocked: false,
                is_counted: false,
                roll_winner: None,
            };
            if entry.is_quest_drop {
                loot.add_quest_item(item);
                quest_slot += 1;
            } else {
                loot.add_item(item);
                slot += 1;
            }
        }
        loot.generated = true;
        self.active_loot.insert(source_guid, loot);
        true
    }

    /// Whether a gameobject loot table has a quest-only item needed by a player.
    pub fn player_needs_gameobject_quest_loot<F>(&self, loot_id: u32, mut needs_item: F) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.gameobject_loot_tables
            .get(&loot_id)
            .is_some_and(|table| {
                table
                    .iter()
                    .any(|entry| entry.is_quest_drop && needs_item(entry.item))
            })
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

    /// Resolve which quest items `player` may see, counting them once.
    ///
    /// A quest drop only becomes "unlooted loot" once a player who needs it opens the corpse.
    pub fn fill_quest_loot<F>(&self, source: ObjectGuid, player: ObjectGuid, allowed: F) -> Vec<u8>
    where
        F: FnMut(u32) -> bool,
    {
        self.active_loot
            .get_mut(&source)
            .map(|mut loot| loot.fill_quest_loot(player, allowed))
            .unwrap_or_default()
    }

    /// Look up an item by the slot the client sees, without marking it looted.
    ///
    /// Quest items are presented to the client with a slot offset equal to the number of
    /// normal items, so we first check normal items. If not found, we subtract the offset
    /// and check quest items — but only those already made visible to this player.
    pub fn peek_item(&self, source: ObjectGuid, slot: u8, player: ObjectGuid) -> Option<LootItem> {
        let loot = self.active_loot.get(&source)?;

        if let Some(item) = loot.get_item(slot) {
            return Some(item.clone());
        }

        let normal_count = loot.items.len() as u8;
        if slot < normal_count {
            return None;
        }

        let quest_slot = slot - normal_count;
        // Don't hand out a quest item the player was never shown
        if !loot
            .visible_quest_slots(player)
            .is_some_and(|slots| slots.contains(&quest_slot))
        {
            return None;
        }

        loot.get_quest_item(quest_slot).cloned()
    }

    /// Mark the item at the client-visible slot as looted, using the same slot mapping
    /// as [`Self::peek_item`].
    pub fn loot_item(&self, source: ObjectGuid, slot: u8, player: ObjectGuid) -> Option<LootItem> {
        let mut loot = self.active_loot.get_mut(&source)?;

        // Try normal items first
        if let Some(item) = loot.loot_item(slot) {
            return Some(item);
        }

        // Try quest items using offset slot
        let normal_count = loot.items.len() as u8;
        if slot < normal_count {
            return None;
        }

        let quest_slot = slot - normal_count;
        if !loot
            .visible_quest_slots(player)
            .is_some_and(|slots| slots.contains(&quest_slot))
        {
            return None;
        }

        loot.loot_quest_item(quest_slot)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gameobject_quest_loot_is_generated_as_quest_only() {
        let manager = LootManager::new();
        let loot_id = 10119;
        let source = ObjectGuid::new_gameobject(161557, 1);
        manager.gameobject_loot_tables.insert(
            loot_id,
            vec![LootTableEntry {
                entry: loot_id,
                item: 11119,
                chance: 100.0,
                min_count: 1,
                max_count: 1,
                group_id: 0,
                is_reference: false,
                is_quest_drop: true,
            }],
        );

        manager.generate_gameobject_loot(source, loot_id);

        let loot = manager.get_loot(source).expect("chest loot should exist");
        assert!(loot.items.is_empty());
        assert_eq!(loot.quest_items.len(), 1);
        assert_eq!(loot.quest_items[0].item_id, 11119);
        assert!(manager.player_needs_gameobject_quest_loot(loot_id, |item_id| item_id == 11119));
        assert!(!manager.player_needs_gameobject_quest_loot(loot_id, |_| false));
    }
}
