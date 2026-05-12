//! Starting items and action buttons for new character creation.
//!
//! These functions run during CMSG_CHAR_CREATE before the player is online,
//! so they insert directly into the database rather than going through the
//! online InventorySystem (which requires cache and broadcaster).

use crate::shared::database::world::repositories::player_create_info_repository::{
    PlayerCreateInfoActionRow, PlayerCreateInfoItemRow,
};
use crate::world::game::items::manager::ItemManager;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::collections::HashSet;
use tracing::{debug, warn, info};

/// Inventory slot constants
const INVENTORY_SLOT_ITEM_START: u8 = 23;
const INVENTORY_SLOT_ITEM_END: u8 = 39;

/// Create starting items for a newly created character.
///
/// Inserts item_instance and character_inventory rows in a single transaction.
/// Equipment-type items are placed in their matching equipment slot (0-18),
/// remaining items go into inventory bag slots (23-38).
///
/// Returns the number of items successfully created.
pub async fn give_starting_items(
    character_db: &MySqlPool,
    item_mgr: &ItemManager,
    character_guid: u32,
    starting_items: &[PlayerCreateInfoItemRow],
) -> Result<usize> {
    let mut used_equipment_slots: HashSet<u8> = HashSet::new();
    let mut next_inventory_slot: u8 = INVENTORY_SLOT_ITEM_START;
    let mut created_count: usize = 0;

    let mut tx = character_db
        .begin().await
        .context("Failed to begin transaction for starting items")?;

    for item_info in starting_items {
        let template = match item_mgr.get_template(item_info.itemid) {
            Some(t) => t,
            None => {
                warn!(
                    "Starting item template {} not found, skipping",
                    item_info.itemid
                );
                continue;
            }
        };

        let item_guid = item_mgr.generate_guid();

        // Determine target slot: equipment slot if possible, otherwise inventory
        let target_slot =
            match get_equipment_slot_for_item(template.inventory_type, &mut used_equipment_slots) {
                Some(equip_slot) => equip_slot,
                None => {
                    if next_inventory_slot >= INVENTORY_SLOT_ITEM_END {
                        warn!(
                            "No inventory space for starting item {}, skipping",
                            item_info.itemid
                        );
                        continue;
                    }
                    let slot = next_inventory_slot;
                    next_inventory_slot += 1;
                    slot
                }
            };

        let count = item_info.amount as u32;
        let durability = template.max_durability as u16;

        // Insert item_instance
        sqlx::query(
            r#"INSERT INTO item_instance
               (guid, item_id, owner_guid, creator_guid, gift_creator_guid, count, duration,
                charges, flags, enchantments, random_property_id, durability, text, generated_loot)
               VALUES (?, ?, ?, 0, 0, ?, 0, '0 0 0 0 0 ', 0, '0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ', 0, ?, 0, 0)"#,
        )
        .bind(item_guid)
        .bind(item_info.itemid)
        .bind(character_guid)
        .bind(count)
        .bind(durability)
        .execute(&mut *tx).await
        .with_context(|| {
            format!(
                "Failed to create item_instance for item {} guid {}",
                item_info.itemid, item_guid
            )
        })?;

        // Insert character_inventory
        sqlx::query(
            r#"INSERT INTO character_inventory (guid, bag, slot, item_guid, item_id)
               VALUES (?, 0, ?, ?, ?)"#,
        )
        .bind(character_guid)
        .bind(target_slot)
        .bind(item_guid)
        .bind(item_info.itemid)
        .execute(&mut *tx).await
        .with_context(|| {
            format!(
                "Failed to create character_inventory for item {} slot {}",
                item_info.itemid, target_slot
            )
        })?;

        debug!(
            "Starting item: item_id={}, guid={}, slot={}",
            item_info.itemid, item_guid, target_slot
        );

        created_count += 1;
    }

    tx.commit().await
        .context("Failed to commit starting items transaction")?;

    debug!(
        "Created {} starting items for character {}",
        created_count,
        character_guid
    );
    Ok(created_count)
}

/// Create starting action buttons for a newly created character.
///
/// Inserts rows into character_action in a single transaction.
pub async fn give_starting_actions(
    character_db: &MySqlPool,
    character_guid: u32,
    starting_actions: &[PlayerCreateInfoActionRow],
) -> Result<()> {
    let mut tx = character_db
        .begin().await
        .context("Failed to begin transaction for starting actions")?;

    for action in starting_actions {
        info!("[CHAR_CREATE] Inserting action button: slot={}, action={}, type={} for character {}", 
              action.button, action.action, action.action_type, character_guid);
        sqlx::query(
            r#"INSERT INTO character_action (guid, button, action, type)
               VALUES (?, ?, ?, ?)"#,
        )
        .bind(character_guid)
        .bind(action.button as u8)
        .bind(action.action)
        .bind(action.action_type as u8)
        .execute(&mut *tx).await
        .with_context(|| {
            format!(
                "Failed to create action button {} for character {}",
                action.button, character_guid
            )
        })?;
    }

    tx.commit().await
        .context("Failed to commit starting actions transaction")?;

    info!(
        "[CHAR_CREATE] Successfully created {} starting action buttons for character {}",
        starting_actions.len(),
        character_guid
    );
    Ok(())
}

/// Get equipment slot (0-18) for an item based on its inventory_type.
/// Returns None if item should go in inventory slot instead.
/// Based on MaNGOS ItemPrototype::GetAllowedEquipSlots.
fn get_equipment_slot_for_item(
    inventory_type: u8,
    used_slots: &mut HashSet<u8>,
) -> Option<u8> {
    let slot = match inventory_type {
        1 => Some(0),   // INVTYPE_HEAD -> EQUIPMENT_SLOT_HEAD
        2 => Some(1),   // INVTYPE_NECK -> EQUIPMENT_SLOT_NECK
        3 => Some(2),   // INVTYPE_SHOULDERS -> EQUIPMENT_SLOT_SHOULDERS
        4 => Some(3),   // INVTYPE_BODY -> EQUIPMENT_SLOT_BODY (shirt)
        5 => Some(4),   // INVTYPE_CHEST -> EQUIPMENT_SLOT_CHEST
        6 => Some(4),   // INVTYPE_ROBE -> EQUIPMENT_SLOT_CHEST
        7 => Some(5),   // INVTYPE_WAIST -> EQUIPMENT_SLOT_WAIST
        8 => Some(6),   // INVTYPE_LEGS -> EQUIPMENT_SLOT_LEGS
        9 => Some(7),   // INVTYPE_FEET -> EQUIPMENT_SLOT_FEET
        10 => Some(8),  // INVTYPE_WRISTS -> EQUIPMENT_SLOT_WRISTS
        11 => {
            // INVTYPE_FINGER -> FINGER1 or FINGER2
            if !used_slots.contains(&10) {
                Some(10)
            } else if !used_slots.contains(&11) {
                Some(11)
            } else {
                None
            }
        }
        12 => {
            // INVTYPE_TRINKET -> TRINKET1 or TRINKET2
            if !used_slots.contains(&12) {
                Some(12)
            } else if !used_slots.contains(&13) {
                Some(13)
            } else {
                None
            }
        }
        13 => Some(15),  // INVTYPE_WEAPON -> EQUIPMENT_SLOT_MAINHAND
        14 => Some(16),  // INVTYPE_SHIELD -> EQUIPMENT_SLOT_OFFHAND
        15 => Some(17),  // INVTYPE_RANGED -> EQUIPMENT_SLOT_RANGED
        16 => Some(14),  // INVTYPE_CLOAK -> EQUIPMENT_SLOT_BACK
        17 => Some(15),  // INVTYPE_2HWEAPON -> EQUIPMENT_SLOT_MAINHAND
        19 => Some(18),  // INVTYPE_TABARD -> EQUIPMENT_SLOT_TABARD
        20 => Some(4),   // INVTYPE_ROBE -> EQUIPMENT_SLOT_CHEST
        21 => Some(15),  // INVTYPE_WEAPONMAINHAND -> EQUIPMENT_SLOT_MAINHAND
        22 => Some(16),  // INVTYPE_WEAPONOFFHAND -> EQUIPMENT_SLOT_OFFHAND
        23 => Some(16),  // INVTYPE_HOLDABLE -> EQUIPMENT_SLOT_OFFHAND
        25 => Some(17),  // INVTYPE_THROWN -> EQUIPMENT_SLOT_RANGED
        26 => Some(17),  // INVTYPE_RANGEDRIGHT -> EQUIPMENT_SLOT_RANGED
        _ => None,       // Other types (bags, ammo, etc.) go in inventory
    };

    // For single-slot items, check if slot is available
    // For multi-slot items (rings/trinkets), the match arm already handled slot selection
    if let Some(s) = slot {
        // Try to insert the slot - returns true if it was NOT already present
        if used_slots.insert(s) {
            return Some(s);
        }
        // Slot already used - item goes to inventory
    }

    None
}
