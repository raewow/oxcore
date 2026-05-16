//! Item Manager - owns item templates and loads them from database

use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::{MySqlPool, Row};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::info;

/// Item template from database
#[derive(Debug, Clone)]
pub struct ItemTemplate {
    pub entry: u32,
    pub name: String,
    pub display_id: u32,
    pub quality: u8,
    pub item_level: u32,
    pub required_level: u32,
    pub item_class: u32,
    pub item_subclass: u32,
    pub inventory_type: u8,
    pub max_count: u32, // Maximum copies player can have (0 = unlimited)
    pub stackable: u32, // Maximum stack size per slot
    pub max_durability: u32,
    pub buy_price: u32,
    pub sell_price: u32,
    pub container_slots: u8,
    pub start_quest: u32,
    // Spell fields for item usage
    pub spell_id: [u32; 5],
    pub spell_trigger: [u32; 5],
    pub spell_charges: [i32; 5],
    pub spell_cooldown: [i32; 5],
    pub spell_category: [u32; 5],
    pub spell_category_cooldown: [i32; 5],
}

impl ItemTemplate {
    /// Get the maximum stack size for this item
    /// Matches MaNGOS GetMaxStackSize() behavior
    pub fn get_max_stack_size(&self) -> u32 {
        self.stackable
    }
}

/// Manages item templates and provides database loading
pub struct ItemManager {
    templates: DashMap<u32, Arc<ItemTemplate>>,
    next_guid: AtomicU32,
}

impl ItemManager {
    pub fn new() -> Self {
        Self {
            templates: DashMap::new(),
            next_guid: AtomicU32::new(0),
        }
    }

    /// Generate the next available item GUID
    ///
    /// Uses atomic fetch_add for thread-safe sequential generation
    pub fn generate_guid(&self) -> u32 {
        self.next_guid.fetch_add(1, Ordering::SeqCst)
    }

    /// Set the initial GUID counter (called during world initialization)
    ///
    /// Should be set to the highest existing GUID from the database
    /// to avoid conflicts with existing items
    pub fn set_initial_guid(&self, guid: u32) {
        self.next_guid.store(guid, Ordering::SeqCst);
    }

    /// Get the current GUID counter value (for debugging)
    pub fn current_guid(&self) -> u32 {
        self.next_guid.load(Ordering::SeqCst)
    }

    /// Get an item template by entry
    pub fn get_template(&self, entry: u32) -> Option<Arc<ItemTemplate>> {
        self.templates.get(&entry).map(|r| Arc::clone(&r))
    }

    /// Search for item templates by name (case-insensitive)
    pub fn search_templates(&self, query: &str) -> Vec<Arc<ItemTemplate>> {
        let query_lower = query.to_lowercase();
        self.templates
            .iter()
            .filter_map(|entry| {
                let template = entry.value();
                if template.name.to_lowercase().contains(&query_lower) {
                    Some(Arc::clone(template))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Add a template
    pub fn add_template(&self, template: ItemTemplate) {
        self.templates.insert(template.entry, Arc::new(template));
    }

    /// Load all item templates from database
    pub async fn load_item_templates(&self, pool: &sqlx::MySqlPool) -> Result<()> {
        let rows = sqlx::query(
            "SELECT entry, name, display_id, quality, item_level, required_level,
                      inventory_type, `class`, subclass, max_count, stackable, max_durability,
                     buy_price, sell_price, container_slots, start_quest,
                     spellid_1, spellid_2, spellid_3, spellid_4, spellid_5,
                     spelltrigger_1, spelltrigger_2, spelltrigger_3, spelltrigger_4, spelltrigger_5,
                     spellcharges_1, spellcharges_2, spellcharges_3, spellcharges_4, spellcharges_5,
                     spellcooldown_1, spellcooldown_2, spellcooldown_3, spellcooldown_4, spellcooldown_5,
                     spellcategory_1, spellcategory_2, spellcategory_3, spellcategory_4, spellcategory_5,
                     spellcategorycooldown_1, spellcategorycooldown_2, spellcategorycooldown_3,
                     spellcategorycooldown_4, spellcategorycooldown_5
               FROM item_template WHERE patch = 0",
        )
        .fetch_all(pool)
        .await
        .context("Failed to load item templates")?;

        let rows_len = rows.len();
        let mut invalid_stackable_count = 0;

        for row in rows {
            let entry: u32 = row.try_get("entry")?;
            let name: String = row.try_get("name")?;
            let display_id: u32 = row.try_get("display_id")?;
            let quality: u8 = row.try_get("quality")?;
            let item_level: u32 = row.try_get("item_level")?;
            let required_level: u32 = row.try_get("required_level")?;
            let inventory_type: u8 = row.try_get("inventory_type")?;
            let item_class: u32 = row.try_get("class")?;
            let item_subclass: u32 = row.try_get("subclass")?;
            let max_count: u32 = row.try_get("max_count")?;
            let mut stackable: u32 = row.try_get("stackable")?;
            let max_durability: u32 = row.try_get("max_durability")?;
            let buy_price: u32 = row.try_get("buy_price")?;
            let sell_price: u32 = row.try_get("sell_price")?;
            let container_slots: u8 = row.try_get("container_slots")?;
            let start_quest: u32 = row.try_get("start_quest")?;

            // Read spell data (default to 0 for all fields)
            let spell_id = [
                row.try_get("spellid_1").unwrap_or(0),
                row.try_get("spellid_2").unwrap_or(0),
                row.try_get("spellid_3").unwrap_or(0),
                row.try_get("spellid_4").unwrap_or(0),
                row.try_get("spellid_5").unwrap_or(0),
            ];
            let spell_trigger = [
                row.try_get("spelltrigger_1").unwrap_or(0),
                row.try_get("spelltrigger_2").unwrap_or(0),
                row.try_get("spelltrigger_3").unwrap_or(0),
                row.try_get("spelltrigger_4").unwrap_or(0),
                row.try_get("spelltrigger_5").unwrap_or(0),
            ];
            let spell_charges = [
                row.try_get("spellcharges_1").unwrap_or(0),
                row.try_get("spellcharges_2").unwrap_or(0),
                row.try_get("spellcharges_3").unwrap_or(0),
                row.try_get("spellcharges_4").unwrap_or(0),
                row.try_get("spellcharges_5").unwrap_or(0),
            ];
            let spell_cooldown = [
                row.try_get("spellcooldown_1").unwrap_or(-1),
                row.try_get("spellcooldown_2").unwrap_or(-1),
                row.try_get("spellcooldown_3").unwrap_or(-1),
                row.try_get("spellcooldown_4").unwrap_or(-1),
                row.try_get("spellcooldown_5").unwrap_or(-1),
            ];
            let spell_category = [
                row.try_get("spellcategory_1").unwrap_or(0),
                row.try_get("spellcategory_2").unwrap_or(0),
                row.try_get("spellcategory_3").unwrap_or(0),
                row.try_get("spellcategory_4").unwrap_or(0),
                row.try_get("spellcategory_5").unwrap_or(0),
            ];
            let spell_category_cooldown = [
                row.try_get("spellcategorycooldown_1").unwrap_or(-1),
                row.try_get("spellcategorycooldown_2").unwrap_or(-1),
                row.try_get("spellcategorycooldown_3").unwrap_or(-1),
                row.try_get("spellcategorycooldown_4").unwrap_or(-1),
                row.try_get("spellcategorycooldown_5").unwrap_or(-1),
            ];

            // Validate stackable value (matches MaNGOS ObjectMgr.cpp behavior)
            if stackable == 0 {
                tracing::warn!(
                    "Item (Entry: {}) has wrong value in stackable (0), replace by default 1.",
                    entry
                );
                stackable = 1;
                invalid_stackable_count += 1;
            } else if stackable > 255 {
                tracing::warn!(
                    "Item (Entry: {}) has too large value in stackable ({}), replace by hardcoded upper limit (255).",
                    entry, stackable
                );
                stackable = 255;
                invalid_stackable_count += 1;
            }

            let template = ItemTemplate {
                entry,
                name,
                display_id,
                quality,
                item_level,
                required_level,
                item_class,
                item_subclass,
                inventory_type,
                max_count,
                stackable,
                max_durability,
                buy_price,
                sell_price,
                container_slots,
                start_quest,
                spell_id,
                spell_trigger,
                spell_charges,
                spell_cooldown,
                spell_category,
                spell_category_cooldown,
            };

            self.add_template(template);
        }

        info!("Loaded {} item templates", rows_len);
        if invalid_stackable_count > 0 {
            tracing::warn!(
                "Fixed {} item templates with invalid stackable values",
                invalid_stackable_count
            );
        }

        Ok(())
    }

    /// Number of loaded templates
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Initialize the GUID generator from the database
    ///
    /// Queries the maximum existing item GUID from both item_instance and character_inventory
    /// tables to ensure we don't generate duplicate GUIDs. Sets the next GUID to max + 1.
    pub async fn init_guid_generator(&self, pool: &sqlx::MySqlPool) -> Result<()> {
        // Get max item GUID from both item_instance and character_inventory tables
        // We need to check both because items might exist in character_inventory without item_instance entries
        // GREATEST returns BIGINT, so we need to cast it to UNSIGNED INT
        let max_item_guid: Option<u32> = match sqlx::query_scalar::<_, Option<u64>>(
            "SELECT CAST(GREATEST(COALESCE((SELECT MAX(guid) FROM item_instance), 0), COALESCE((SELECT MAX(item_guid) FROM character_inventory), 0)) AS UNSIGNED) as max_guid"
        )
        .fetch_optional(pool)
        .await
        {
            Ok(Some(Some(guid))) => {
                if guid > u32::MAX as u64 {
                    tracing::warn!("GUID value {} exceeds u32::MAX, clamping to {}", guid, u32::MAX);
                    Some(u32::MAX)
                } else {
                    Some(guid as u32)
                }
            },
            Ok(Some(None)) | Ok(None) => None,
            Err(e) => {
                // Fallback: try just item_instance
                tracing::warn!("Could not query max item GUID with GREATEST, trying simpler query: {}", e);
                match sqlx::query_scalar::<_, Option<u64>>(
                    "SELECT CAST(MAX(guid) AS UNSIGNED) FROM item_instance"
                )
                .fetch_optional(pool)
                .await
                {
                    Ok(Some(Some(guid))) => {
                        if guid > u32::MAX as u64 {
                            Some(u32::MAX)
                        } else {
                            Some(guid as u32)
                        }
                    },
                    Ok(Some(None)) | Ok(None) | Err(_) => None,
                }
            }
        };

        let item_start = max_item_guid.map(|g| g + 1).unwrap_or(1);
        self.set_initial_guid(item_start);

        tracing::debug!(
            "Initialized item GUID generator - starting at {}",
            item_start
        );

        Ok(())
    }
}

impl Default for ItemManager {
    fn default() -> Self {
        Self::new()
    }
}
