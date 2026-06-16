//! Console Command System
//!
//! Provides MaNGOS-style console command functionality for server administration.
//! Commands are read from stdin and executed in the server update loop.
//! This is a generic framework that can work with any server context.

pub mod command;
pub mod input;
pub mod output;
pub mod ui;

pub use command::{CommandContext, CommandHandler, CommandInfo, CommandRegistry, ConsoleCommand};
pub use input::run_console_input;
pub use output::{print_console, print_error, print_success};
