use anyhow::{Context, Result};
use clap::Subcommand;
use std::io::Write;

use crate::config::Config;
use crate::postgres;

#[derive(Subcommand)]
pub enum Command {
    /// Bootstrap PostgreSQL namespaces and apply pending migrations
    Migrate,
    /// Show PostgreSQL migration status for every application schema
    Status,
    /// Create a PostgreSQL migration in sql/postgres/migrations/<schema>/
    New { schema: String, name: String },
    /// Drop and recreate application schemas, then re-run PostgreSQL migrations
    Fresh {
        /// Skip the confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn run(config: &Config, command: Command) -> Result<()> {
    match command {
        Command::Migrate => migrate(config).await,
        Command::Status => status(config).await,
        Command::New { schema, name } => new(config, &schema, &name),
        Command::Fresh { yes } => fresh(config, yes).await,
    }
}

async fn migrate(config: &Config) -> Result<()> {
    let pool = postgres::connect(config.postgres_url()?).await?;
    postgres::ensure_bootstrap(&pool).await?;

    for schema in postgres::SCHEMAS {
        println!("[{schema}]");
        let applied = postgres::applied_migrations(&pool, schema).await?;
        let migrations = postgres::collect_migrations(&config.postgres_migrations_dir, schema)?;
        let pending: Vec<_> = migrations
            .iter()
            .filter(|migration| !applied.contains(&migration.id))
            .collect();
        if pending.is_empty() {
            println!("  No pending migrations");
        } else {
            println!("  Applying {} migration(s)...", pending.len());
            for migration in pending {
                println!("    {}_{}.sql", migration.id, migration.name);
                let sql = std::fs::read_to_string(&migration.path).with_context(|| {
                    format!(
                        "Failed to read PostgreSQL migration {}",
                        migration.path.display()
                    )
                })?;
                postgres::run_migration(&pool, schema, &migration.id, &migration.name, &sql)
                    .await?;
            }
        }
        println!();
    }
    Ok(())
}

async fn status(config: &Config) -> Result<()> {
    let pool = postgres::connect(config.postgres_url()?).await?;
    if !postgres::migrations_initialized(&pool).await? {
        println!(
            "PostgreSQL is not initialized (run: cargo run -p oxcore-db --bin db -- pg migrate)"
        );
        return Ok(());
    }

    for schema in postgres::SCHEMAS {
        let applied = postgres::applied_migrations(&pool, schema).await?;
        let migrations = postgres::collect_migrations(&config.postgres_migrations_dir, schema)?;
        let pending: Vec<_> = migrations
            .iter()
            .filter(|migration| !applied.contains(&migration.id))
            .collect();
        println!("[{schema}]");
        println!("  Applied: {}", applied.len());
        println!("  Pending: {}", pending.len());
        for migration in pending {
            println!("    + {}_{}.sql", migration.id, migration.name);
        }
        println!();
    }
    Ok(())
}

fn new(config: &Config, schema: &str, name: &str) -> Result<()> {
    let path = postgres::create_migration(&config.postgres_migrations_dir, schema, name)?;
    println!("Created: {}", path.display());
    Ok(())
}

async fn fresh(config: &Config, yes: bool) -> Result<()> {
    if !yes && !confirm()? {
        println!("Aborted.");
        return Ok(());
    }
    let pool = postgres::connect(config.postgres_url()?).await?;
    println!("Dropping and recreating PostgreSQL application schemas...");
    postgres::reset_application_schemas(&pool).await?;
    migrate(config).await
}

fn confirm() -> Result<bool> {
    println!(
        "This will DROP and recreate the following PostgreSQL schemas, destroying their data:"
    );
    for schema in postgres::SCHEMAS {
        println!("  - {schema}");
    }
    print!("Continue? [y/N] ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}
