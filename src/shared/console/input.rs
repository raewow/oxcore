//! Console Input Task
//!
//! Reads commands from stdin and sends them to the command processor.
//! Uses a terminal UI to keep input visible at the bottom.

use super::command::ConsoleCommand;
use super::ui::ConsoleUI;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::info;

/// Run the console input task
/// Uses terminal UI to keep input prompt visible at the bottom
pub async fn run_console_input(
    tx: mpsc::Sender<ConsoleCommand>,
    shutdown: broadcast::Receiver<()>,
) {
    info!("Console input task started. Type 'help' for available commands.");

    let ui = ConsoleUI::new();
    if let Err(e) = ui.run(tx, shutdown).await {
        tracing::error!("Console UI error: {}", e);
    }
}
