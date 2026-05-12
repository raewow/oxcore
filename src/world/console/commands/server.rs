//! Server Management Commands for world

use crate::shared::common::AccountType;
use crate::shared::console::command::{CommandContext, CommandInfo};
use crate::shared::console::output::print_console;
use crate::world::World;
use anyhow::Result;
use std::time::Duration;

/// Help command - shows available commands or help for a specific command
pub async fn cmd_help(ctx: &CommandContext<'_, World>, args: &str) -> Result<String> {
    let registry = ctx.context.get_command_registry().await;

    if args.trim().is_empty() {
        // Show all commands
        let mut output = String::from("Available commands:\n");
        let names = registry.command_names();
        for name in names {
            if let Some(info) = registry.get_info(&name) {
                output.push_str(&format!("  {} - {}\n", name, info.help));
            }
        }
        Ok(output)
    } else {
        // Show help for specific command
        let cmd_name = args.trim().to_lowercase();
        match registry.get_info(&cmd_name) {
            Some(info) => Ok(format!("{} - {}", cmd_name, info.help)),
            None => Ok(format!("Unknown command: {}", cmd_name)),
        }
    }
}

/// Info command - shows server information
pub async fn cmd_info(ctx: &CommandContext<'_, World>, _args: &str) -> Result<String> {
    let mut output = String::from("Server Information:\n");
    output.push_str(&format!(
        "  World update interval: {:?}\n",
        ctx.context.update_interval
    ));
    output.push_str(&format!("  Realm ID: {}\n", ctx.context.get_realm_id()));
    output.push_str(&format!("  Running: {}\n", ctx.context.is_running()));

    Ok(output)
}

/// Shutdown command - gracefully shuts down server
pub async fn cmd_shutdown(ctx: &CommandContext<'_, World>, args: &str) -> Result<String> {
    let delay_secs = if args.trim().is_empty() {
        // Immediate shutdown
        0
    } else {
        // Try to parse as seconds
        match args.trim().parse::<u64>() {
            Ok(secs) => secs,
            Err(_) => {
                return Ok(format!(
                    "Invalid duration: '{}'. Use format like '15' for 15 seconds.",
                    args.trim()
                ));
            }
        }
    };

    if delay_secs == 0 {
        // Instant shutdown
        print_console("Shutting down server immediately...");
        ctx.context.stop();
        Ok("Shutdown initiated".to_string())
    } else {
        // Simple timed shutdown (for now, just announce and shut down)
        print_console(&format!(
            "Server will shutdown in {} seconds...",
            delay_secs
        ));
        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs));

        print_console("Shutting down server now...");
        ctx.context.stop();
        Ok("Shutdown complete".to_string())
    }
}

/// Get command info for help command
pub fn help_info() -> CommandInfo {
    CommandInfo {
        name: "help",
        help: "Show available commands or help for a specific command. Usage: help [command]",
        min_security: AccountType::Player,
    }
}

/// Get command info for info command
pub fn info_info() -> CommandInfo {
    CommandInfo {
        name: "info",
        help: "Show server information",
        min_security: AccountType::Player,
    }
}

/// Get command info for shutdown command
pub fn shutdown_info() -> CommandInfo {
    CommandInfo {
        name: "shutdown",
        help: "Gracefully shutdown the server. Usage: shutdown [seconds]",
        min_security: AccountType::Administrator,
    }
}
