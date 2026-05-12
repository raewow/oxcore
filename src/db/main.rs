use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod db;

use config::Config;

#[derive(Parser)]
#[command(name = "db", about = "Database tool for rcore")]
struct Cli {
    /// Path to configuration file (default: config.toml)
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run pending migrations on all databases
    Migrate,
    /// Show migration status for all databases
    Status,
    /// Create a new migration file
    New {
        /// Database: world, auth, characters, logs
        db: String,
        /// Migration name (snake_case description)
        name: String,
    },
    /// Show usage and examples
    Help,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.config)?;

    match cli.command {
        Command::Help => { commands::help::run(); return Ok(()); }
        Command::Migrate => commands::migrate::run(&config).await?,
        Command::Status => commands::status::run(&config).await?,
        Command::New { db, name } => commands::new::run(&db, &name, &config.migrations_dir)?,
    }

    Ok(())
}
