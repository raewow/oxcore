//! Terminal UI for Console Input
//!
//! Provides simple console input using standard stdin.
//! This approach works reliably with async logging.

use super::command::ConsoleCommand;
use std::io::{self, BufRead, Write};
use tokio::sync::{broadcast, mpsc};
use tracing::error;

/// Console UI handler
pub struct ConsoleUI;

impl ConsoleUI {
    pub fn new() -> Self {
        Self
    }

    /// Run the console UI
    pub async fn run(
        &self,
        tx: mpsc::Sender<ConsoleCommand>,
        mut shutdown: broadcast::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Use blocking stdin in a separate task
        let tx_clone = tx.clone();
        let readline_task = tokio::task::spawn_blocking(move || {
            let stdin = io::stdin();
            let mut stdout = io::stdout();

            // Print initial prompt
            println!();
            println!("Type 'help' for available commands.");
            print!("server> ");
            let _ = stdout.flush();

            for line in stdin.lock().lines() {
                match line {
                    Ok(input) => {
                        let cmd = ConsoleCommand::parse(&input);
                        if !cmd.name.is_empty() {
                            if tx_clone.try_send(cmd).is_err() {
                                eprintln!("Warning: Command channel full");
                            }
                        }
                        // Print next prompt
                        print!("server> ");
                        let _ = stdout.flush();
                    }
                    Err(e) => {
                        error!("Stdin read error: {}", e);
                        break;
                    }
                }
            }
        });

        tokio::select! {
            _ = readline_task => {
                // Readline task finished
            }
            _ = shutdown.recv() => {
                // Shutdown signal received
            }
        }

        Ok(())
    }
}

impl Default for ConsoleUI {
    fn default() -> Self {
        Self::new()
    }
}
