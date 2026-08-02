use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod db;
mod postgres;

use config::Config;

#[derive(Parser)]
#[command(
    name = "db",
    about = "Database tool for oxcore",
    after_help = commands::help::AFTER_HELP
)]
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
        /// Database: world, auth, or characters
        db: String,
        /// Migration name (snake_case description)
        name: String,
    },
    /// Drop and recreate all databases, then re-run migrate
    Fresh {
        /// Skip the confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// PostgreSQL foundation migrations (does not affect the MySQL runtime databases)
    Pg {
        #[command(subcommand)]
        command: commands::postgres::Command,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.config)?;

    match cli.command {
        Command::Migrate => commands::migrate::run(&config).await?,
        Command::Status => commands::status::run(&config).await?,
        Command::New { db, name } => commands::new::run(&db, &name, &config.migrations_dir)?,
        Command::Fresh { yes } => commands::fresh::run(&config, yes).await?,
        Command::Pg { command } => commands::postgres::run(&config, command).await?,
    }

    Ok(())
}
