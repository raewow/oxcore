use anyhow::{Context, Result};
use tracing::{error, info, warn};

use crate::auth::config::Config;
use crate::auth::database::Database;

pub async fn initialize_database(config: &Config) -> Result<Database> {
    let database = Database::new(config)
        .await
        .context("Failed to initialize database connection pool")?;

    info!("Database connection pool initialized");

    database
        .ping()
        .await
        .context("Database connection test failed")?;

    info!("Database connection verified");

    if let Err(e) = database.cleanup_expired_bans().await {
        warn!("Failed to cleanup expired bans: {}", e);
    }

    Ok(database)
}
