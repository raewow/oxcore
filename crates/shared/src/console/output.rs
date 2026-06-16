//! Console Output Formatting
//!
//! Provides formatted output for console commands.

use tracing::info;

/// Print a message to the console
/// Uses tracing::info! for consistent logging
pub fn print_console(message: &str) {
    info!("{}", message);
}

/// Print an error message to the console
pub fn print_error(message: &str) {
    tracing::error!("{}", message);
}

/// Print a success message to the console
pub fn print_success(message: &str) {
    tracing::info!("{}", message);
}
