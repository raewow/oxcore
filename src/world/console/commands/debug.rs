//! Debug Commands for world

use crate::shared::common::AccountType;
use crate::shared::console::command::{CommandContext, CommandInfo};
use crate::world::World;
use anyhow::Result;

/// Stats command - shows server statistics
pub async fn cmd_stats(ctx: &CommandContext<'_, World>, _args: &str) -> Result<String> {
    let sessions = ctx.context.session_mgr.get_all_sessions();

    let mut output = String::from("Server Statistics:\n");
    output.push_str(&format!("  Total sessions: {}\n", sessions.len()));

    // Calculate online player count (if player_mgr is ready)
    let online_count = ctx.context.managers.player_mgr.get_online_count().await;
    output.push_str(&format!("  Online players: {}\n", online_count));

    output.push_str(&format!(
        "  World update interval: {:?}\n",
        ctx.context.update_interval
    ));
    output.push_str(&format!("  Realm ID: {}\n", ctx.context.get_realm_id()));
    output.push_str(&format!("  Running: {}\n", ctx.context.is_running()));

    Ok(output)
}

/// Get command info for stats command
pub fn stats_info() -> CommandInfo {
    CommandInfo {
        name: "stats",
        help: "Show server statistics",
        min_security: AccountType::Moderator,
    }
}
