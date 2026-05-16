//! Player command handlers
//!
//! Commands that affect player state and progression.

use anyhow::Result;

use crate::shared::common::AccountType;
use crate::shared::game::experience::XpSource;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};
use crate::world::game::inventory::GoldResult;

/// Add XP command - adds experience points to the player
pub async fn cmd_addxp(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    // Parse amount from args
    let xp_amount = args
        .trim()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(100); // Default if no amount specified

    // Add XP via experience system
    let result = ctx
        .world
        .systems
        .experience
        .add_xp(ctx.player_guid, xp_amount, XpSource::Quest, None, 1.0)
        .await;

    match result {
        Ok((xp_gained, leveled_up, new_level)) => {
            if leveled_up {
                Ok(format!(
                    "Added {} XP. Leveled up to level {}!",
                    xp_gained, new_level
                ))
            } else {
                Ok(format!("Added {} XP.", xp_gained))
            }
        }
        Err(e) => Ok(format!("Failed to add XP: {}", e)),
    }
}

pub fn addxp_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "addxp",
        help: "Add experience points. Usage: .addxp <amount>",
        min_security: AccountType::GameMaster,
    }
}

/// Add gold command - adds copper to the player (1 gold = 10000 copper)
pub async fn cmd_addgold(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok(
            "Usage: .addgold <amount> [g|s|c]  (e.g. .addgold 10g or .addgold 500)".to_string(),
        );
    }

    // Parse optional unit suffix: g = gold (10000c), s = silver (100c), c/none = copper
    let (num_str, multiplier) = if let Some(stripped) = args.strip_suffix('g') {
        (stripped, 10000u32)
    } else if let Some(stripped) = args.strip_suffix('s') {
        (stripped, 100u32)
    } else if let Some(stripped) = args.strip_suffix('c') {
        (stripped, 1u32)
    } else {
        (args, 1u32)
    };

    let amount: u32 = match num_str.trim().parse::<u32>() {
        Ok(v) => v,
        Err(_) => return Ok("Invalid amount. Usage: .addgold <amount> [g|s|c]".to_string()),
    };

    let copper = match amount.checked_mul(multiplier) {
        Some(v) => v,
        None => return Ok("Amount too large.".to_string()),
    };

    match ctx
        .world
        .systems
        .inventory
        .add_gold(ctx.player_guid, copper)
    {
        GoldResult::Success { new_balance } => {
            let g = new_balance / 10000;
            let s = (new_balance % 10000) / 100;
            let c = new_balance % 100;
            Ok(format!(
                "Added {} copper. Balance: {}g {}s {}c",
                copper, g, s, c
            ))
        }
        GoldResult::CapExceeded => Ok("Cannot add gold: would exceed the gold cap.".to_string()),
        GoldResult::PlayerNotLoaded => Ok("Player inventory not loaded.".to_string()),
        _ => Ok("Failed to add gold.".to_string()),
    }
}

pub fn addgold_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "addgold",
        help: "Add gold to your character. Usage: .addgold <amount>[g|s|c]  e.g. .addgold 10g",
        min_security: AccountType::GameMaster,
    }
}
