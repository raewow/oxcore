//! Guild command handlers for GM use
//!
//! Provides administrative guild management commands:
//! - create: Instantly create a guild (bypasses charter)
//! - disband: Disband a guild
//! - info: Show guild information
//! - addmember: Add player to guild
//! - removemember: Remove player from guild
//! - setrank: Set player's guild rank
//! - rename: Rename a guild

use anyhow::{anyhow, Result};
use crate::shared::common::AccountType;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};
use crate::shared::protocol::ObjectGuid;

/// Guild command info
pub fn guild_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "guild",
        help: "Guild management commands. Usage: .guild <create|disband|info|addmember|removemember|setrank|rename> [args]",
        min_security: AccountType::GameMaster,
    }
}

/// Main guild command handler with subcommands
pub async fn cmd_guild(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();

    if args.is_empty() {
        return Ok("Guild commands: create, disband, info, addmember, removemember, setrank, rename. Use .guild <subcommand> for details.".to_string());
    }

    // Parse subcommand
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let subcommand = parts[0].to_lowercase();
    let subargs = if parts.len() > 1 { parts[1] } else { "" };

    match subcommand.as_str() {
        "create" => cmd_guild_create(ctx, subargs).await,
        "disband" => cmd_guild_disband(ctx, subargs).await,
        "info" => cmd_guild_info(ctx, subargs).await,
        "addmember" => cmd_guild_addmember(ctx, subargs).await,
        "removemember" => cmd_guild_removemember(ctx, subargs).await,
        "setrank" => cmd_guild_setrank(ctx, subargs).await,
        "rename" => cmd_guild_rename(ctx, subargs).await,
        _ => Ok(format!(
            "Unknown guild subcommand '{}'. Use .guild for help.",
            subcommand
        )),
    }
}

/// Create a guild instantly (bypasses charter)
/// Usage: .guild create <guild_name> [player_name]
async fn cmd_guild_create(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();

    if args.is_empty() {
        return Ok("Usage: .guild create <guild_name> [player_name]".to_string());
    }

    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    let guild_name = parts[0];
    let target_player_name = if parts.len() > 1 {
        Some(parts[1])
    } else {
        None
    };

    // Check guild name length
    if guild_name.len() > 24 {
        return Ok("Guild name too long (max 24 characters)".to_string());
    }

    // Check if guild name already exists
    let guild_system = &ctx.world.systems.guild;
    if guild_system.has_guild_name(guild_name) {
        return Ok(format!("Guild '{}' already exists", guild_name));
    }

    // Determine leader (target player or self)
    let (leader_guid, leader_name) = if let Some(target_name) = target_player_name {
        // Find target player by name
        if let Some(guid) = ctx
            .world
            .managers
            .player_mgr
            .find_player_by_name(target_name)
        {
            // Check if target is already in a guild
            if guild_system.is_in_guild(guid) {
                return Ok(format!("Player '{}' is already in a guild", target_name));
            }

            // Get player name
            let name = ctx
                .world
                .managers
                .player_mgr
                .get_player_name(guid)
                .ok_or_else(|| anyhow!("Failed to get player name"))?;

            (guid, name)
        } else {
            return Ok(format!("Player '{}' not found", target_name));
        }
    } else {
        // Use command executor as leader
        let player_guid = ctx.player_guid;
        let player_name = ctx
            .world
            .managers
            .player_mgr
            .get_player_name(player_guid)
            .ok_or_else(|| anyhow!("Failed to get player name"))?;

        // Check if GM is already in a guild
        if guild_system.is_in_guild(player_guid) {
            return Ok("You are already in a guild. Leave your current guild first.".to_string());
        }

        (player_guid, player_name)
    };

    // Create guild using GuildSystem
    match guild_system
        .create_guild_from_petition(leader_guid, leader_name.clone(), guild_name.to_string())
        .await
    {
        Ok(()) => Ok(format!(
            "Guild '{}' created successfully with {} as guild master",
            guild_name, leader_name
        )),
        Err(e) => Ok(format!("Failed to create guild: {}", e)),
    }
}

/// Disband a guild
/// Usage: .guild disband <guild_name>
async fn cmd_guild_disband(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let guild_name = args.trim();

    if guild_name.is_empty() {
        return Ok("Usage: .guild disband <guild_name>".to_string());
    }

    let guild_system = &ctx.world.systems.guild;

    // Find guild by name
    let guild_id = if let Some(guild) = guild_system.get_guild_by_name(guild_name) {
        guild.guild_id
    } else {
        return Ok(format!("Guild '{}' not found", guild_name));
    };

    // Get leader GUID to authorize disband
    let leader_guid = guild_system
        .get_guild(guild_id)
        .map(|g| g.info.leader_guid)
        .ok_or_else(|| anyhow!("Guild not found"))?;

    // Disband guild
    match guild_system.disband_guild(leader_guid).await {
        Ok(()) => Ok(format!("Guild '{}' disbanded successfully", guild_name)),
        Err(e) => Ok(format!("Failed to disband guild: {}", e)),
    }
}

/// Show guild information
/// Usage: .guild info [guild_name]
async fn cmd_guild_info(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let guild_name = args.trim();

    let guild_system = &ctx.world.systems.guild;

    // Get guild (either from args or from player's current guild)
    let guild = if guild_name.is_empty() {
        // Show player's own guild info
        let player_guid = ctx.player_guid;

        match guild_system.get_player_guild(player_guid) {
            Some(state) => {
                if let Some(guild_id) = state.guild_id {
                    guild_system.get_guild(guild_id)
                } else {
                    return Ok("You are not in a guild. Specify a guild name.".to_string());
                }
            }
            None => return Ok("You are not in a guild. Specify a guild name.".to_string()),
        }
    } else {
        // Show specified guild info
        match guild_system.get_guild_by_name(guild_name) {
            Some(g) => Some(g),
            None => return Ok(format!("Guild '{}' not found", guild_name)),
        }
    };

    let guild = match guild {
        Some(g) => g,
        None => return Ok("Guild not found".to_string()),
    };

    // Get leader name
    let leader_name = if let Some(member) = guild.members.get(&guild.info.leader_guid) {
        member.name.clone()
    } else {
        "Unknown".to_string()
    };

    let mut info = format!(
        "Guild: {} (ID: {})\nLeader: {}\nMOTD: {}\nMembers: {}\nCreated: {}",
        guild.info.name,
        guild.guild_id,
        leader_name,
        guild.info.motd,
        guild.members.len(),
        guild.info.create_date
    );

    // Show ranks
    info.push_str("\n\nRanks:");
    for (i, rank) in guild.ranks.iter().enumerate() {
        info.push_str(&format!(
            "\n  {}: {} (rights: 0x{:08X})",
            i, rank.name, rank.rights
        ));
    }

    Ok(info)
}

/// Add a member to a guild
/// Usage: .guild addmember <player_name> <guild_name>
async fn cmd_guild_addmember(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();

    if parts.len() < 2 {
        return Ok("Usage: .guild addmember <player_name> <guild_name>".to_string());
    }

    let player_name = parts[0];
    let guild_name = parts[1];

    let guild_system = &ctx.world.systems.guild;

    // Find guild
    let guild_id = if let Some(guild) = guild_system.get_guild_by_name(guild_name) {
        guild.guild_id
    } else {
        return Ok(format!("Guild '{}' not found", guild_name));
    };

    // Find player
    let player_guid = if let Some(guid) = ctx.world.managers.player_mgr.find_player_by_name(player_name) {
        // Check if player is already in a guild
        if guild_system.is_in_guild(guid) {
            return Ok(format!("Player '{}' is already in a guild", player_name));
        }
        guid
    } else {
        return Ok(format!("Player '{}' not found", player_name));
    };

    // Get lowest rank ID for guild
    let guild = guild_system.get_guild(guild_id).unwrap();
    let lowest_rank = guild.get_lowest_rank_id();

    // Add member to guild with lowest rank
    match guild_system
        .add_member_directly(player_guid, player_name.to_string(), guild_id, lowest_rank)
        .await
    {
        Ok(()) => Ok(format!("Added '{}' to guild '{}'", player_name, guild_name)),
        Err(e) => Ok(format!("Failed to add member: {}", e)),
    }
}

/// Remove a member from their guild
/// Usage: .guild removemember <player_name>
async fn cmd_guild_removemember(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let player_name = args.trim();

    if player_name.is_empty() {
        return Ok("Usage: .guild removemember <player_name>".to_string());
    }

    let guild_system = &ctx.world.systems.guild;

    // Find player
    let player_guid = if let Some(guid) = ctx.world.managers.player_mgr.find_player_by_name(player_name) {
        guid
    } else {
        return Ok(format!("Player '{}' not found", player_name));
    };

    // Check if player is in a guild
    let guild_id = if let Some(state) = guild_system.get_player_guild(player_guid) {
        state.guild_id.ok_or_else(|| anyhow!("Player not in a guild"))?
    } else {
        return Ok(format!("Player '{}' is not in a guild", player_name));
    };

    // Get guild leader to authorize removal (GM bypasses permissions)
    let leader_guid = guild_system
        .get_guild(guild_id)
        .map(|g| g.info.leader_guid)
        .ok_or_else(|| anyhow!("Guild not found"))?;

    // Remove member (using leader GUID to bypass permission checks)
    match guild_system
        .remove_member(leader_guid, player_guid, player_name.to_string())
        .await
    {
        Ok(()) => Ok(format!("Removed '{}' from guild", player_name)),
        Err(e) => Ok(format!("Failed to remove member: {}", e)),
    }
}

/// Set a player's guild rank
/// Usage: .guild setrank <player_name> <rank_id>
async fn cmd_guild_setrank(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();

    if parts.len() < 2 {
        return Ok("Usage: .guild setrank <player_name> <rank_id>".to_string());
    }

    let player_name = parts[0];
    let rank_str = parts[1];

    let rank_id = match rank_str.parse::<u8>() {
        Ok(r) => r,
        Err(_) => return Ok("Invalid rank ID (must be a number 0-255)".to_string()),
    };

    let guild_system = &ctx.world.systems.guild;

    // Find player
    let player_guid = if let Some(guid) = ctx.world.managers.player_mgr.find_player_by_name(player_name) {
        guid
    } else {
        return Ok(format!("Player '{}' not found", player_name));
    };

    // Check if player is in a guild
    let guild = if let Some(state) = guild_system.get_player_guild(player_guid) {
        if let Some(guild_id) = state.guild_id {
            guild_system.get_guild(guild_id)
        } else {
            return Ok(format!("Player '{}' is not in a guild", player_name));
        }
    } else {
        return Ok(format!("Player '{}' is not in a guild", player_name));
    };

    let guild = match guild {
        Some(g) => g,
        None => return Ok(format!("Player '{}' is not in a guild", player_name)),
    };

    // Validate rank exists
    if rank_id as usize >= guild.ranks.len() {
        return Ok(format!(
            "Invalid rank ID. Guild has ranks 0-{}",
            guild.ranks.len() - 1
        ));
    }

    // Set member rank
    match guild_system
        .set_member_rank_directly(player_guid, rank_id)
        .await
    {
        Ok(()) => Ok(format!("Set '{}' to rank {}", player_name, rank_id)),
        Err(e) => Ok(format!("Failed to set rank: {}", e)),
    }
}

/// Rename a guild
/// Usage: .guild rename <old_name> <new_name>
async fn cmd_guild_rename(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();

    if parts.len() < 2 {
        return Ok("Usage: .guild rename <old_name> <new_name>".to_string());
    }

    let old_name = parts[0];
    let new_name = parts[1];

    // Check new name length
    if new_name.len() > 24 {
        return Ok("New guild name too long (max 24 characters)".to_string());
    }

    let guild_system = &ctx.world.systems.guild;

    // Check if new name already exists
    if guild_system.has_guild_name(new_name) {
        return Ok(format!("Guild name '{}' is already taken", new_name));
    }

    // Find old guild
    let guild_id = if let Some(guild) = guild_system.get_guild_by_name(old_name) {
        guild.guild_id
    } else {
        return Ok(format!("Guild '{}' not found", old_name));
    };

    // Rename guild
    match guild_system
        .rename_guild(guild_id, new_name.to_string())
        .await
    {
        Ok(()) => Ok(format!("Guild '{}' renamed to '{}'", old_name, new_name)),
        Err(e) => Ok(format!("Failed to rename guild: {}", e)),
    }
}
