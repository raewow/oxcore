//! Command Framework
//!
//! Defines the command handler functions and registry for console commands.
//! This is a generic framework that can work with any server context.

use crate::shared::common::AccountType;
use crate::shared::console::output::print_console;
use anyhow::Result;
use std::collections::HashMap;

/// Command context passed to command handlers
/// The Context type parameter allows any server-specific context (World, etc.)
pub struct CommandContext<'a, Context> {
    /// Reference to the server context (World, etc.)
    pub context: &'a Context,
    /// Security level of the command executor (always Console for stdin)
    pub security: AccountType,
}

/// Console command parsed from stdin
#[derive(Debug, Clone)]
pub struct ConsoleCommand {
    /// Command name (first word)
    pub name: String,
    /// Command arguments (remaining words)
    pub args: String,
}

impl ConsoleCommand {
    /// Parse a command string into a ConsoleCommand
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input.is_empty() {
            return Self {
                name: String::new(),
                args: String::new(),
            };
        }

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let name = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

        Self { name, args }
    }
}

/// Command metadata
pub struct CommandInfo {
    /// Command name
    pub name: &'static str,
    /// Help text describing the command
    pub help: &'static str,
    /// Minimum security level required to execute this command
    pub min_security: AccountType,
}

/// Command handler function type
/// Returns a boxed future that resolves to a Result<String>
/// Generic over Context to work with any server context
pub type CommandHandler<Context> = Box<
    dyn for<'a> Fn(
            &'a CommandContext<'_, Context>,
            &'a str,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String>> + Send + 'a>,
        > + Send
        + Sync,
>;

/// Registry entry for a command
struct CommandEntry<Context> {
    info: CommandInfo,
    handler: CommandHandler<Context>,
}

/// Registry of all available commands
/// Generic over Context to work with any server context
pub struct CommandRegistry<Context> {
    commands: HashMap<String, CommandEntry<Context>>,
}

impl<Context> CommandRegistry<Context> {
    /// Create a new command registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a command handler
    pub fn register<F>(&mut self, info: CommandInfo, handler: F)
    where
        F: for<'a> Fn(
                &'a CommandContext<'_, Context>,
                &'a str,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<String>> + Send + 'a>,
            > + Send
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

    /// Get command info by name
    pub fn get_info(&self, name: &str) -> Option<&CommandInfo> {
        self.commands.get(&name.to_lowercase()).map(|e| &e.info)
    }

    /// Get command handler by name
    fn get_handler(&self, name: &str) -> Option<&CommandHandler<Context>> {
        self.commands.get(&name.to_lowercase()).map(|e| &e.handler)
    }

    /// Get all registered command names
    pub fn command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// Execute a command
    pub async fn execute<'a>(
        &self,
        cmd: &ConsoleCommand,
        ctx: &CommandContext<'a, Context>,
    ) -> Result<String> {
        if cmd.name.is_empty() {
            return Ok(String::new());
        }

        match self.commands.get(&cmd.name) {
            Some(entry) => {
                // Check security level
                if ctx.security < entry.info.min_security {
                    return Ok(format!(
                        "Insufficient security level. Required: {:?}, Current: {:?}",
                        entry.info.min_security, ctx.security
                    ));
                }

                (entry.handler)(ctx, &cmd.args).await
            }
            None => Ok(format!(
                "Unknown command: {}. Type 'help' for available commands.",
                cmd.name
            )),
        }
    }

    /// Process pending console commands from the receiver
    /// This is a convenience method that combines command processing with the registry
    pub async fn process_commands(
        &self,
        console_rx: &tokio::sync::Mutex<tokio::sync::mpsc::Receiver<ConsoleCommand>>,
        context: &Context,
    ) -> Result<()> {
        let mut rx = console_rx.lock().await;

        while let Ok(cmd) = rx.try_recv() {
            let ctx = CommandContext {
                context,
                security: AccountType::Console,
            };

            match self.execute(&cmd, &ctx).await {
                Ok(result) => {
                    if !result.is_empty() {
                        print_console(&result);
                    }
                }
                Err(e) => {
                    tracing::error!("Console command execution error: {}", e);
                    print_console(&format!("Error: {}", e));
                }
            }
        }

        Ok(())
    }
}

impl<Context> Default for CommandRegistry<Context> {
    fn default() -> Self {
        Self::new()
    }
}
