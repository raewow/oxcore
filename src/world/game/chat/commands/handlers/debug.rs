//! Debug command handlers
//!
//! Simple diagnostic and debugging commands.

use anyhow::{anyhow, Result};

use crate::shared::common::AccountType;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};

/// Ping command - simple connectivity test
pub async fn cmd_ping(_ctx: &ChatCommandContext<'_>, _args: &str) -> Result<String> {
    Ok("pong".to_string())
}

pub fn ping_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "ping",
        help: "Simple connectivity test - responds with 'pong'",
        min_security: AccountType::Player,
    }
}

/// Pos command - shows player position
pub async fn cmd_pos(ctx: &ChatCommandContext<'_>, _args: &str) -> Result<String> {
    let player = ctx
        .world
        .managers
        .player_mgr
        .get_player(ctx.player_guid)
        .ok_or_else(|| anyhow!("Player not found"))?;

    let pos = player.movement.position;
    let map_id = player.map_id;

    Ok(format!(
        "Map {} ({:.2}, {:.2}, {:.2}, o={:.3})",
        map_id, pos.x, pos.y, pos.z, pos.o
    ))
}

pub fn pos_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "pos",
        help: "Shows your current position and map",
        min_security: AccountType::Player,
    }
}

pub fn where_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "where",
        help: "Shows your current position (alias for .pos)",
        min_security: AccountType::Player,
    }
}

pub fn coords_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "coords",
        help: "Shows your current position (alias for .pos)",
        min_security: AccountType::Player,
    }
}
