//! Spell command handlers
//!
//! Commands for spell lookup and casting.

use anyhow::Result;

use crate::shared::common::AccountType;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};

/// Lookup spell command - search for spells by name or ID
pub async fn cmd_lookup_spell(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let search = args.trim();

    if search.is_empty() {
        return Ok("Usage: .lookup spell <name or ID>".to_string());
    }

    // Try to parse as spell ID first
    if let Ok(spell_id) = search.parse::<u32>() {
        if let Some(spell) = ctx.world.managers.spell_mgr.get(spell_id) {
            return Ok(format!(
                "Spell {}: {} (Level: {}, School: {}, Mana: {})",
                spell.id, spell.name, spell.spell_level, spell.school, spell.mana_cost
            ));
        } else {
            return Ok(format!("Spell ID {} not found", spell_id));
        }
    }

    // Search by name
    let mut results = ctx.world.managers.spell_mgr.search_by_name(search);

    if results.is_empty() {
        return Ok(format!("No spells found matching '{}'", search));
    }

    let total = results.len();
    results.truncate(20);

    let mut output = format!(
        "Found {} spells matching '{}' (showing {}):\n",
        total,
        search,
        results.len()
    );
    for spell in &results {
        output.push_str(&format!(
            "  [{}] {} (Lvl:{}, School:{}, Mana:{})\n",
            spell.id, spell.name, spell.spell_level, spell.school, spell.mana_cost
        ));
    }

    Ok(output)
}

pub fn lookup_spell_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "spell",
        help: "Search for spells by name or ID. Usage: .lookup spell <name or ID>",
        min_security: AccountType::GameMaster,
    }
}

/// Cast command - cast a spell by ID on yourself or your target
pub async fn cmd_cast(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok("Usage: .cast <spell_id>".to_string());
    }

    let spell_id = match args.split_whitespace().next().unwrap().parse::<u32>() {
        Ok(id) => id,
        Err(_) => return Ok("Invalid spell ID. Usage: .cast <spell_id>".to_string()),
    };

    // Verify spell exists
    let spell = match ctx.world.managers.spell_mgr.get(spell_id) {
        Some(s) => s,
        None => return Ok(format!("Spell {} not found", spell_id)),
    };

    let spell_name = spell.name.clone();

    // Use target if selected, otherwise self-cast
    let target_guid = ctx.target.unwrap_or(ctx.player_guid);

    let result = ctx
        .world
        .systems
        .spells
        .cast_spell(
            ctx.player_guid,
            spell_id,
            Some(target_guid),
            true, // triggered = true (GM cast, bypass checks)
            ctx.world,
        )
        .await?;

    Ok(format!(
        "Cast {} ({}) on {} -> {:?}",
        spell_name,
        spell_id,
        if target_guid == ctx.player_guid {
            "self".to_string()
        } else {
            format!("{:?}", target_guid)
        },
        result
    ))
}

pub fn cast_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "cast",
        help: "Cast a spell by ID. Targets selection or self. Usage: .cast <spell_id>",
        min_security: AccountType::GameMaster,
    }
}
