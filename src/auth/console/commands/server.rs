//! Server management commands for the auth server.

use crate::auth::context::AuthServer;
use crate::shared::common::AccountType;
use crate::shared::console::command::{CommandContext, CommandInfo};
use crate::shared::console::output::print_console;
use anyhow::Result;

pub async fn cmd_help(ctx: &CommandContext<'_, AuthServer>, args: &str) -> Result<String> {
    let registry = ctx.context.get_command_registry().await;

    if args.trim().is_empty() {
        let mut output = String::from("Available commands:\n");
        for name in registry.command_names() {
            if let Some(info) = registry.get_info(&name) {
                output.push_str(&format!("  {} - {}\n", name, info.help));
            }
        }
        Ok(output)
    } else {
        let cmd_name = args.trim().to_lowercase();
        match registry.get_info(&cmd_name) {
            Some(info) => Ok(format!("{} - {}", cmd_name, info.help)),
            None => Ok(format!("Unknown command: {}", cmd_name)),
        }
    }
}

pub async fn cmd_info(ctx: &CommandContext<'_, AuthServer>, _args: &str) -> Result<String> {
    let snapshot = ctx.context.metrics.snapshot();
    let mut output = String::from("Auth Server Information:\n");
    output.push_str(&format!("  Bind: {}:{}\n", ctx.context.config.bind_ip, ctx.context.config.realm_server_port));
    output.push_str(&format!(
        "  Connections: {} total, {} active\n",
        snapshot.connections_total, snapshot.connections_active
    ));
    output.push_str(&format!("  Running: {}\n", ctx.context.is_running()));
    Ok(output)
}

pub async fn cmd_shutdown(ctx: &CommandContext<'_, AuthServer>, args: &str) -> Result<String> {
    let delay_secs = if args.trim().is_empty() {
        0
    } else {
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
        print_console("Shutting down auth server immediately...");
        ctx.context.stop();
        Ok("Shutdown initiated".to_string())
    } else {
        print_console(&format!(
            "Auth server will shutdown in {} seconds...",
            delay_secs
        ));
        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
        print_console("Shutting down auth server now...");
        ctx.context.stop();
        Ok("Shutdown complete".to_string())
    }
}

pub fn help_info() -> CommandInfo {
    CommandInfo {
        name: "help",
        help: "Show available commands or help for a specific command. Usage: help [command]",
        min_security: AccountType::Player,
    }
}

pub fn info_info() -> CommandInfo {
    CommandInfo {
        name: "info",
        help: "Show auth server information",
        min_security: AccountType::Player,
    }
}

pub fn shutdown_info() -> CommandInfo {
    CommandInfo {
        name: "shutdown",
        help: "Gracefully shutdown the auth server. Usage: shutdown [seconds]",
        min_security: AccountType::Administrator,
    }
}
