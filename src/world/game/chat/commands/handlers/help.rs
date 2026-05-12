//! Help command handler
//!
//! Provides help and command listing functionality.

use anyhow::Result;

use crate::shared::common::AccountType;
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};

/// Help command - shows available commands and usage
pub async fn cmd_help(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();
    let command = if args.is_empty() { None } else { Some(args) };
    Ok(ctx.world.systems.chat.get_command_help(command, ctx.security))
}

pub fn help_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "help",
        help: "Show available commands and usage",
        min_security: AccountType::Player,
    }
}
