//! Item Manager - owns item templates and loads them from database

use anyhow::{Context, Result};
use dashmap::DashMap;
use sqlx::{MySqlPool, Row};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::info;

use super::item::{ItemRequiredTarget, ItemTargetType};

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
    pub buy_count: u32,
    pub buy_price: u32,
    pub sell_price: u32,
    pub bag_family: u32,
    pub container_slots: u8,
    pub start_quest: u32,
    pub stat_type: [u8; 10],
    pub stat_value: [i16; 10],
    pub delay: u16,
    pub ammo_type: u8,
    pub dmg_min: [f32; 5],
    pub dmg_max: [f32; 5],
    pub dmg_type: [u8; 5],
    pub block: u32,
    pub armor: i16,
    pub holy_res: i16,
    pub fire_res: i16,
    pub nature_res: i16,
    pub frost_res: i16,
    pub shadow_res: i16,
    pub arcane_res: i16,
    // Spell fields for item usage
    pub spell_id: [u32; 5],
    pub spell_trigger: [u32; 5],
    pub spell_charges: [i32; 5],
    pub spell_cooldown: [i32; 5],
    pub spell_category: [u32; 5],
    pub spell_category_cooldown: [i32; 5],
}

impl Default for ItemTemplate {
    fn default() -> Self {
        Self {
            entry: 0,
            name: String::new(),
            display_id: 0,
            quality: 0,
            item_level: 0,
            required_level: 0,
            item_class: 0,
            item_subclass: 0,
            inventory_type: 0,
            max_count: 0,
            stackable: 1,
            max_durability: 0,
            buy_count: 0,
            buy_price: 0,
            sell_price: 0,
            bag_family: 0,
            container_slots: 0,
            start_quest: 0,
            stat_type: [0; 10],
            stat_value: [0; 10],
            delay: 0,
            ammo_type: 0,
            dmg_min: [0.0; 5],
            dmg_max: [0.0; 5],
            dmg_type: [0; 5],
            block: 0,
            armor: 0,
            holy_res: 0,
            fire_res: 0,
            nature_res: 0,
            frost_res: 0,
            shadow_res: 0,
            arcane_res: 0,
            spell_id: [0; 5],
            spell_trigger: [0; 5],
            spell_charges: [0; 5],
            spell_cooldown: [0; 5],
            spell_category: [0; 5],
            spell_category_cooldown: [0; 5],
        }
    }
}

impl ItemTemplate {
    /// Get the maximum stack size for this item
    pub fn get_max_stack_size(&self) -> u32 {
        self.stackable
    }

    /// Get allowed equipment slots for this item based on inventory type
    /// Returns up to 4 possible slots (NULL_SLOT for unused entries)
    pub fn get_allowed_equip_slots(&self, class_id: u8, can_dual_wield: bool) -> [u8; 4] {
        let mut slots = [255u8; 4]; // NULL_SLOT = 255

        match self.inventory_type {
            1 => slots[0] = 0,  // INVTYPE_HEAD -> EQUIPMENT_SLOT_HEAD
            2 => slots[0] = 1,  // INVTYPE_NECK -> EQUIPMENT_SLOT_NECK
            3 => slots[0] = 2,  // INVTYPE_SHOULDERS -> EQUIPMENT_SLOT_SHOULDERS
            4 => slots[0] = 3,  // INVTYPE_BODY -> EQUIPMENT_SLOT_BODY
            5 => slots[0] = 4,  // INVTYPE_CHEST -> EQUIPMENT_SLOT_CHEST
            6 => slots[0] = 4,  // INVTYPE_ROBE -> EQUIPMENT_SLOT_CHEST
            7 => slots[0] = 5,  // INVTYPE_WAIST -> EQUIPMENT_SLOT_WAIST
            8 => slots[0] = 6,  // INVTYPE_LEGS -> EQUIPMENT_SLOT_LEGS
            9 => slots[0] = 7,  // INVTYPE_FEET -> EQUIPMENT_SLOT_FEET
            10 => slots[0] = 8, // INVTYPE_WRISTS -> EQUIPMENT_SLOT_WRISTS
            11 => {
                // INVTYPE_FINGER
                slots[0] = 10; // EQUIPMENT_SLOT_FINGER1
                slots[1] = 11; // EQUIPMENT_SLOT_FINGER2
            }
            12 => {
                // INVTYPE_TRINKET
                slots[0] = 12; // EQUIPMENT_SLOT_TRINKET1
                slots[1] = 13; // EQUIPMENT_SLOT_TRINKET2
            }
            13 => {
                // INVTYPE_WEAPON
                slots[0] = 15; // EQUIPMENT_SLOT_MAINHAND
                if can_dual_wield {
                    slots[1] = 16; // EQUIPMENT_SLOT_OFFHAND
                }
            }
            14 => slots[0] = 16, // INVTYPE_SHIELD -> EQUIPMENT_SLOT_OFFHAND
            15 => slots[0] = 17, // INVTYPE_RANGED -> EQUIPMENT_SLOT_RANGED
            16 => slots[0] = 14, // INVTYPE_CLOAK -> EQUIPMENT_SLOT_BACK
            17 => slots[0] = 15, // INVTYPE_2HWEAPON -> EQUIPMENT_SLOT_MAINHAND
            18 => {
                // INVTYPE_BAG
                slots[0] = 19; // INVENTORY_SLOT_BAG_START
                slots[1] = 20;
                slots[2] = 21;
                slots[3] = 22;
            }
            19 => slots[0] = 18, // INVTYPE_TABARD -> EQUIPMENT_SLOT_TABARD
            20 => slots[0] = 4,  // INVTYPE_ROBE -> EQUIPMENT_SLOT_CHEST
            21 => slots[0] = 15, // INVTYPE_WEAPONMAINHAND -> EQUIPMENT_SLOT_MAINHAND
            22 => slots[0] = 16, // INVTYPE_WEAPONOFFHAND -> EQUIPMENT_SLOT_OFFHAND
            23 => slots[0] = 16, // INVTYPE_HOLDABLE -> EQUIPMENT_SLOT_OFFHAND
            25 => slots[0] = 17, // INVTYPE_THROWN -> EQUIPMENT_SLOT_RANGED
            26 => slots[0] = 17, // INVTYPE_RANGEDRIGHT -> EQUIPMENT_SLOT_RANGED
            28 => {
                // INVTYPE_RELIC
                // Class-specific relic slots
                match self.item_subclass {
                    2 => {
                        // ITEM_SUBCLASS_ARMOR_LIBRAM
                        if class_id == 2 {
                            // CLASS_PALADIN
                            slots[0] = 17; // EQUIPMENT_SLOT_RANGED
                        }
                    }
                    3 => {
                        // ITEM_SUBCLASS_ARMOR_IDOL
                        if class_id == 11 {
                            // CLASS_DRUID
                            slots[0] = 17; // EQUIPMENT_SLOT_RANGED
                        }
                    }
                    4 => {
                        // ITEM_SUBCLASS_ARMOR_TOTEM
                        if class_id == 7 {
                            // CLASS_SHAMAN
                            slots[0] = 17; // EQUIPMENT_SLOT_RANGED
                        }
                    }
                    5 => {
                        // ITEM_SUBCLASS_ARMOR_MISC
                        if class_id == 9 {
                            // CLASS_WARLOCK
                            slots[0] = 17; // EQUIPMENT_SLOT_RANGED
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        slots
    }

    /// Get the proficiency skill required to equip this item
    pub fn get_proficiency_skill(&self) -> u32 {
        const ITEM_CLASS_WEAPON: u32 = 2;
        const ITEM_CLASS_ARMOR: u32 = 4;

        const ITEM_WEAPON_SKILLS: [u32; 21] = [
            44,  // SKILL_AXES (subclass 0)
            172, // SKILL_2H_AXES (subclass 1)
            45,  // SKILL_BOWS (subclass 2)
            46,  // SKILL_GUNS (subclass 3)
            54,  // SKILL_MACES (subclass 4)
            160, // SKILL_2H_MACES (subclass 5)
            229, // SKILL_POLEARMS (subclass 6)
            43,  // SKILL_SWORDS (subclass 7)
            55,  // SKILL_2H_SWORDS (subclass 8)
            0,   // unused (subclass 9)
            136, // SKILL_STAVES (subclass 10)
            0,   // unused (subclass 11)
            0,   // unused (subclass 12)
            162, // SKILL_UNARMED (subclass 13)
            0,   // unused (subclass 14)
            173, // SKILL_DAGGERS (subclass 15)
            176, // SKILL_THROWN (subclass 16)
            0,   // SKILL_ASSASSINATION (subclass 17) - not in constants
            226, // SKILL_CROSSBOWS (subclass 18)
            228, // SKILL_WANDS (subclass 19)
            356, // SKILL_FISHING (subclass 20)
        ];

        const ITEM_ARMOR_SKILLS: [u32; 10] = [
            0,   // subclass 0 (misc)
            415, // SKILL_CLOTH (subclass 1)
            414, // SKILL_LEATHER (subclass 2)
            413, // SKILL_MAIL (subclass 3)
            293, // SKILL_PLATE_MAIL (subclass 4)
            0,   // subclass 5 (buckler)
            433, // SKILL_SHIELD (subclass 6)
            0,   // subclass 7 (libram)
            0,   // subclass 8 (idol)
            0,   // subclass 9 (totem)
        ];

        match self.item_class {
            ITEM_CLASS_WEAPON => {
                if self.item_subclass < ITEM_WEAPON_SKILLS.len() as u32 {
                    ITEM_WEAPON_SKILLS[self.item_subclass as usize]
                } else {
                    0
                }
            }
            ITEM_CLASS_ARMOR => {
                if self.item_subclass < ITEM_ARMOR_SKILLS.len() as u32 {
                    ITEM_ARMOR_SKILLS[self.item_subclass as usize]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// Get the proficiency spell ID that teaches this item's proficiency
    pub fn get_proficiency_spell(&self) -> u32 {
        const ITEM_CLASS_WEAPON: u32 = 2;
        const ITEM_CLASS_ARMOR: u32 = 4;

        match self.item_class {
            ITEM_CLASS_WEAPON => {
                match self.item_subclass {
                    0 => 196,   // Axe
                    1 => 197,   // 2H Axe
                    2 => 264,   // Bow
                    3 => 266,   // Gun
                    4 => 198,   // Mace
                    5 => 199,   // 2H Mace
                    6 => 200,   // Polearm
                    7 => 201,   // Sword
                    8 => 202,   // 2H Sword
                    10 => 227,  // Staff
                    15 => 1180, // Dagger
                    16 => 2567, // Thrown
                    17 => 3386, // Spear
                    18 => 5011, // Crossbow
                    19 => 5009, // Wand
                    _ => 0,
                }
            }
            ITEM_CLASS_ARMOR => {
                match self.item_subclass {
                    1 => 9078, // Cloth
                    2 => 9077, // Leather
                    3 => 8737, // Mail
                    4 => 750,  // Plate
                    6 => 9116, // Shield
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    /// Check if this item template fits the spell's equipment requirements
    pub fn is_fit_to_spell_requirements(&self, spell: &oxcore_dbc::structures::SpellEntry) -> bool {
        // Check item class
        if spell.equipped_item_class != -1 {
            // Exception for Enchant Cloak - Minor Agility (spell ID 13419)
            if spell.id == 13419 && self.inventory_type == 16 {
                // INVTYPE_CLOAK
                return true;
            }

            if spell.equipped_item_class != self.item_class as i32 && spell.id != 13419 {
                return false;
            }

            // Check subclass mask
            if spell.equipped_item_sub_class_mask != 0 {
                if (spell.equipped_item_sub_class_mask & (1 << self.item_subclass)) == 0 {
                    return false;
                }
            }
        }

        // Check inventory type mask
        if spell.equipped_item_inventory_type_mask != 0 {
            if (spell.equipped_item_inventory_type_mask & (1 << self.inventory_type)) == 0 {
                return false;
            }
        }

        true
    }

    /// Check if an enchantment makes the item soulbound
    /// Note: This requires SpellItemEnchantment DBC data which is not yet loaded
    pub fn is_bound_by_enchant(&self, _enchantments: &[(u32, u32, u32)]) -> bool {
        // TODO: Check SpellItemEnchantmentEntry for ENCHANTMENT_CAN_SOULBOUND flag
        // when enchantment DBC data is available
        false
    }
}

/// Manages item templates and provides database loading
pub struct ItemManager {
    templates: DashMap<u32, Arc<ItemTemplate>>,
    /// item entry -> required-target rules (loaded from `item_required_target`)
    required_targets: DashMap<u32, Vec<ItemRequiredTarget>>,
    next_guid: AtomicU32,
}

impl ItemManager {
    pub fn new() -> Self {
        Self {
            templates: DashMap::new(),
            required_targets: DashMap::new(),
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
                     buy_count, buy_price, sell_price, container_slots, bag_family, start_quest,
                     stat_type1, stat_type2, stat_type3, stat_type4, stat_type5,
                     stat_type6, stat_type7, stat_type8, stat_type9, stat_type10,
                     stat_value1, stat_value2, stat_value3, stat_value4, stat_value5,
                     stat_value6, stat_value7, stat_value8, stat_value9, stat_value10,
                     delay, ammo_type, dmg_min1, dmg_min2, dmg_min3, dmg_min4, dmg_min5,
                     dmg_max1, dmg_max2, dmg_max3, dmg_max4, dmg_max5,
                     dmg_type1, dmg_type2, dmg_type3, dmg_type4, dmg_type5, block,
                     armor, holy_res, fire_res, nature_res, frost_res, shadow_res, arcane_res,
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
            let buy_count: u32 = row.try_get("buy_count")?;
            let buy_price: u32 = row.try_get("buy_price")?;
            let sell_price: u32 = row.try_get("sell_price")?;
            let bag_family = u32::try_from(row.try_get::<i32, _>("bag_family")?)
                .context("item_template.bag_family must not be negative")?;
            let container_slots: u8 = row.try_get("container_slots")?;
            let start_quest: u32 = row.try_get("start_quest")?;
            let stat_type = [
                row.try_get("stat_type1")?,
                row.try_get("stat_type2")?,
                row.try_get("stat_type3")?,
                row.try_get("stat_type4")?,
                row.try_get("stat_type5")?,
                row.try_get("stat_type6")?,
                row.try_get("stat_type7")?,
                row.try_get("stat_type8")?,
                row.try_get("stat_type9")?,
                row.try_get("stat_type10")?,
            ];
            let stat_value = [
                row.try_get("stat_value1")?,
                row.try_get("stat_value2")?,
                row.try_get("stat_value3")?,
                row.try_get("stat_value4")?,
                row.try_get("stat_value5")?,
                row.try_get("stat_value6")?,
                row.try_get("stat_value7")?,
                row.try_get("stat_value8")?,
                row.try_get("stat_value9")?,
                row.try_get("stat_value10")?,
            ];
            let delay: u16 = row.try_get("delay")?;
            let ammo_type: u8 = row.try_get("ammo_type")?;
            let dmg_min = [
                row.try_get("dmg_min1")?,
                row.try_get("dmg_min2")?,
                row.try_get("dmg_min3")?,
                row.try_get("dmg_min4")?,
                row.try_get("dmg_min5")?,
            ];
            let dmg_max = [
                row.try_get("dmg_max1")?,
                row.try_get("dmg_max2")?,
                row.try_get("dmg_max3")?,
                row.try_get("dmg_max4")?,
                row.try_get("dmg_max5")?,
            ];
            let dmg_type = [
                row.try_get("dmg_type1")?,
                row.try_get("dmg_type2")?,
                row.try_get("dmg_type3")?,
                row.try_get("dmg_type4")?,
                row.try_get("dmg_type5")?,
            ];
            let block: u32 = row.try_get("block")?;
            let armor: i16 = row.try_get("armor")?;
            let holy_res: i16 = row.try_get("holy_res")?;
            let fire_res: i16 = row.try_get("fire_res")?;
            let nature_res: i16 = row.try_get("nature_res")?;
            let frost_res: i16 = row.try_get("frost_res")?;
            let shadow_res: i16 = row.try_get("shadow_res")?;
            let arcane_res: i16 = row.try_get("arcane_res")?;

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

            // Validate stackable value
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
                buy_count,
                buy_price,
                sell_price,
                bag_family,
                container_slots,
                start_quest,
                stat_type,
                stat_value,
                delay,
                ammo_type,
                dmg_min,
                dmg_max,
                dmg_type,
                block,
                armor,
                holy_res,
                fire_res,
                nature_res,
                frost_res,
                shadow_res,
                arcane_res,
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

    /// Register a required-target rule for an item entry.
    pub fn add_required_target(&self, entry: u32, target: ItemRequiredTarget) {
        self.required_targets.entry(entry).or_default().push(target);
    }

    /// Load all item required-target rules from the `item_required_target` table.
    pub async fn load_item_required_targets(&self, pool: &sqlx::MySqlPool) -> Result<()> {
        let rows = sqlx::query("SELECT entry, `type`, target_entry FROM item_required_target")
            .fetch_all(pool)
            .await
            .context("Failed to load item_required_target")?;

        let rows_len = rows.len();
        let mut skipped = 0;

        for row in rows {
            let entry: u32 = row.try_get("entry")?;
            let raw_type: u8 = row.try_get("type")?;
            let target_entry: u32 = row.try_get("target_entry")?;

            let Some(target_type) = ItemTargetType::from_db(raw_type) else {
                tracing::warn!(
                    "Item (Entry: {}) has unknown item_required_target type {}, skipped.",
                    entry,
                    raw_type
                );
                skipped += 1;
                continue;
            };

            self.add_required_target(entry, ItemRequiredTarget::new(target_type, target_entry));
        }

        info!("Loaded {} item required-target rules", rows_len - skipped);
        Ok(())
    }

    /// Check whether `target` is a valid use-target for the given item entry.
    ///
    /// `target` is `None` when no unit is targeted; otherwise it is
    /// `(target_is_unit, target_entry, target_alive)`. Items with no
    /// required-target rules are always usable.
    pub fn is_target_valid_for_item_use(
        &self,
        item_entry: u32,
        target: Option<(bool, u32, bool)>,
    ) -> bool {
        let Some(rules) = self.required_targets.get(&item_entry) else {
            return true;
        };

        if rules.is_empty() {
            return true;
        }

        let Some((is_unit, entry, alive)) = target else {
            return false;
        };

        rules
            .iter()
            .any(|rule| rule.is_fit_to_requirements(is_unit, entry, alive))
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
