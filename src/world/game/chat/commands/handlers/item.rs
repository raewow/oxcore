//! Item command handlers
//!
//! Commands for item lookup and manipulation.

use anyhow::{anyhow, Result};

use crate::shared::common::AccountType;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};
use crate::world::game::inventory::AddItemResult;

/// Lookup item command - search for items by name or ID
pub async fn cmd_lookup_item(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let search = args.trim();

    if search.is_empty() {
        return Ok("Usage: .lookup item <name or ID>".to_string());
    }

    // Try to parse as item ID first
    if let Ok(item_id) = search.parse::<u32>() {
        if let Some(template) = ctx.world.systems.item_mgr.get_template(item_id) {
            return Ok(format!(
                "Item {}: {} (Quality: {}, ItemLevel: {}, ReqLevel: {})",
                template.entry,
                template.name,
                template.quality,
                template.item_level,
                template.required_level
            ));
        } else {
            return Ok(format!("Item ID {} not found", item_id));
        }
    }

    // Search by name pattern using in-memory templates
    let mut results = ctx.world.systems.item_mgr.search_templates(search);

    if results.is_empty() {
        return Ok(format!("No items found matching '{}'", search));
    }

    // Limit to 20 results
    results.truncate(20);

    let mut output = format!("Found {} items matching '{}':\n", results.len(), search);
    for template in results {
        output.push_str(&format!(
            "  [{}] {} (Q:{}, IL:{}, RL:{})\n",
            template.entry,
            template.name,
            template.quality,
            template.item_level,
            template.required_level
        ));
    }

    Ok(output)
}

pub fn lookup_item_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "item",
        help: "Search for items by name or ID. Usage: .lookup item <name or ID>",
        min_security: AccountType::GameMaster,
    }
}

/// Add item command - add items to player inventory
pub async fn cmd_additem(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let mut parts = args.trim().split_whitespace();

    let item_id = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| anyhow!("Usage: .additem <item_id> [quantity]"))?;

    let quantity = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    if quantity == 0 {
        return Ok("Quantity must be greater than 0".to_string());
    }

    // Verify item exists
    let template = match ctx.world.systems.item_mgr.get_template(item_id) {
        Some(t) => t,
        None => return Ok(format!("Item {} not found in database", item_id)),
    };

    // Add to inventory
    let result = ctx
        .world
        .systems
        .inventory
        .add_item(ctx.player_guid, item_id, quantity)
        .await;

    match result {
        AddItemResult::Success {
            items_created,
            items_modified,
        } => {
            let new_items = items_created.len();
            let updated = items_modified.len();

            let msg = if new_items > 0 && updated > 0 {
                format!(
                    "Added {} x {} (created {} stacks, updated {} stacks)",
                    quantity, template.name, new_items, updated
                )
            } else if new_items > 0 {
                format!(
                    "Added {} x {} (created {} stacks)",
                    quantity, template.name, new_items
                )
            } else {
                format!(
                    "Added {} x {} (updated {} stacks)",
                    quantity, template.name, updated
                )
            };

            Ok(msg)
        }
        AddItemResult::InventoryFull => Ok(format!(
            "Cannot add {} x {}: Inventory is full!",
            quantity, template.name
        )),
        AddItemResult::InvalidItem => Ok(format!("Item {} does not exist", item_id)),
        AddItemResult::PlayerNotLoaded => Ok("Player not loaded in world".to_string()),
        AddItemResult::DatabaseError(e) => Ok(format!("Database error: {}", e)),
    }
}

pub fn additem_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "additem",
        help: "Add items to your inventory. Usage: .additem <item_id> [quantity]",
        min_security: AccountType::GameMaster,
    }
}
