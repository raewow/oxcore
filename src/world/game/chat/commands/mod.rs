//! In-game chat command system for world
//!
//! Provides an extensible command framework for GM and admin commands
//! accessed via chat messages starting with '.' or '!'.

pub mod context;
pub mod handlers;

pub use context::{ChatCommandContext, ChatCommandInfo};

use crate::shared::common::AccountType;
use anyhow::Result;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Handler function type for chat commands
/// Returns a boxed future that resolves to Result<String>
pub type ChatCommandHandler = Box<
    dyn for<'a> Fn(
            &'a ChatCommandContext<'_>,
            &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>
        + Send
        + Sync,
>;

/// Registry entry for a command
struct CommandEntry {
    info: ChatCommandInfo,
    handler: ChatCommandHandler,
}

/// Registry of all available chat commands
pub struct CommandRegistry {
    /// Top-level commands
    commands: HashMap<String, CommandEntry>,
    /// Subcommand groups (e.g., "modify" -> {"speed": handler, "hp": handler})
    subcommands: HashMap<String, HashMap<String, CommandEntry>>,
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            subcommands: HashMap::new(),
        }
    }

    /// Register a top-level command handler
    pub fn register<F>(&mut self, info: ChatCommandInfo, handler: F)
    where
        F: for<'a> Fn(
                &'a ChatCommandContext<'_>,
                &'a str,
            ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        let name = info.name.to_lowercase();
        self.commands.insert(
            name,
            CommandEntry {
                info,
                handler: Box::new(handler),
            },
        );
    }

    /// Register a subcommand (e.g., "modify" "speed")
    pub fn register_subcommand<F>(&mut self, parent: &str, info: ChatCommandInfo, handler: F)
    where
        F: for<'a> Fn(
                &'a ChatCommandContext<'_>,
                &'a str,
            ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        let parent_name = parent.to_lowercase();
        let cmd_name = info.name.to_lowercase();

        self.subcommands.entry(parent_name).or_default().insert(
            cmd_name,
            CommandEntry {
                info,
                handler: Box::new(handler),
            },
        );
    }

    /// Get command info by name
    pub fn get_info(&self, name: &str) -> Option<&ChatCommandInfo> {
        self.commands.get(&name.to_lowercase()).map(|e| &e.info)
    }

    /// Check if a command name is registered (top-level or subcommand group)
    pub fn exists(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.commands.contains_key(&lower) || self.subcommands.contains_key(&lower)
    }

    /// Get all registered command names
    pub fn command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get all subcommand names for a parent command
    pub fn subcommand_names(&self, parent: &str) -> Vec<String> {
        self.subcommands
            .get(&parent.to_lowercase())
            .map(|subs| {
                let mut names: Vec<String> = subs.keys().cloned().collect();
                names.sort();
                names
            })
            .unwrap_or_default()
    }

    /// Execute a command string
    pub async fn execute<'a>(
        &self,
        command_str: &str,
        ctx: &ChatCommandContext<'a>,
    ) -> Result<String> {
        let command_str = command_str.trim();
        if command_str.is_empty() {
            return Ok(String::new());
        }

        // Parse command and args
        let parts: Vec<&str> = command_str.splitn(2, ' ').collect();
        let cmd_name = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        // First check for subcommands
        if let Some(subcmds) = self.subcommands.get(&cmd_name) {
            // Parse subcommand name from args
            let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
            let sub_name = sub_parts
                .get(0)
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            let sub_args = sub_parts.get(1).map(|s| s.trim()).unwrap_or("");

            if let Some(entry) = subcmds.get(&sub_name) {
                // Check security level
                if ctx.security < entry.info.min_security {
                    return Ok(format!(
                        "Insufficient permission. Required: {:?}",
                        entry.info.min_security
                    ));
                }
                return (entry.handler)(ctx, sub_args).await;
            } else if !sub_name.is_empty() {
                // Unknown subcommand
                let available = self.subcommand_names(&cmd_name).join(", ");
                return Ok(format!(
                    "Unknown subcommand '{}'. Available: {}",
                    sub_name, available
                ));
            }
        }

        // Check for top-level command
        match self.commands.get(&cmd_name) {
            Some(entry) => {
                // Check security level
                if ctx.security < entry.info.min_security {
                    return Ok(format!(
                        "Insufficient permission. Required: {:?}",
                        entry.info.min_security
                    ));
                }
                (entry.handler)(ctx, args).await
            }
            None => Ok(format!(
                "Unknown command: {}. Type .help for available commands.",
                cmd_name
            )),
        }
    }

    /// Generate help text for commands
    pub fn get_help(&self, command: Option<&str>, security: AccountType) -> String {
        match command {
            Some(cmd) => {
                let cmd_lower = cmd.to_lowercase();

                // Check for specific command help
                if let Some(entry) = self.commands.get(&cmd_lower) {
                    if security >= entry.info.min_security {
                        return format!(".{} - {}", entry.info.name, entry.info.help);
                    } else {
                        return format!("No permission to view help for '{}'", cmd);
                    }
                }

                // Check for subcommand group help
                if let Some(subcmds) = self.subcommands.get(&cmd_lower) {
                    let mut output = format!("Subcommands for .{}:\n", cmd_lower);
                    for (name, entry) in subcmds {
                        if security >= entry.info.min_security {
                            output.push_str(&format!(
                                "  .{} {} - {}\n",
                                cmd_lower, name, entry.info.help
                            ));
                        }
                    }
                    return output;
                }

                format!("Unknown command: {}", cmd)
            }
            None => {
                // List all available commands
                let mut output = String::from("Available commands:\n");

                // Top-level commands
                let mut names = self.command_names();
                names.sort();
                for name in names {
                    if let Some(entry) = self.commands.get(&name) {
                        if security >= entry.info.min_security {
                            output.push_str(&format!("  .{} - {}\n", name, entry.info.help));
                        }
                    }
                }

                // Subcommand groups
                let mut sub_groups: Vec<&String> = self.subcommands.keys().collect();
                sub_groups.sort();
                for group in sub_groups {
                    if let Some(subcmds) = self.subcommands.get(group) {
                        // Check if user can see any subcommand
                        let visible: Vec<_> = subcmds
                            .iter()
                            .filter(|(_, e)| security >= e.info.min_security)
                            .collect();
                        if !visible.is_empty() {
                            output.push_str(&format!(
                                "  .{} <subcommand> - {} subcommands available\n",
                                group,
                                visible.len()
                            ));
                        }
                    }
                }

                output
            }
        }
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
