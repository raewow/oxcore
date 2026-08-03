use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
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
    /// Manage PostgreSQL application schemas and migrations
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
        Command::Pg { command } => commands::postgres::run(&config, command).await?,
    }

    Ok(())
}
