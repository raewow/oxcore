// Database modules organized by database
pub mod auth;
pub mod characters;
pub mod logs;
pub mod world;

use anyhow::{Context, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use tracing::info;

use crate::world::config::Config;

/// Container for all database connection pools
#[derive(Clone)]
pub struct Databases {
    pub world: MySqlPool,
    pub character: MySqlPool,
    pub auth: MySqlPool,
    pub logs: MySqlPool,
}

impl Databases {
    /// Create all database connection pools from config
    pub async fn new(config: &Config) -> Result<Self> {
        Self::from_urls(
            &config.world_database_url,
            &config.character_database_url,
            &config.login_database_url,
            &config.logs_database_url,
        )
        .await
    }

    /// Create all database connection pools from URLs
    pub async fn from_urls(
        world_url: &str,
        character_url: &str,
        auth_url: &str,
        logs_url: &str,
    ) -> Result<Self> {
        info!("Connecting to databases...");
        info!("  World: {}", mask_database_url(world_url));
        info!("  Character: {}", mask_database_url(character_url));
        info!("  Auth: {}", mask_database_url(auth_url));
        info!("  Logs: {}", mask_database_url(logs_url));

        let world = MySqlPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(world_url)
            .await
            .context("Failed to connect to World database")?;
        let character = MySqlPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(character_url)
            .await
            .context("Failed to connect to Character database")?;
        let auth = MySqlPoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(auth_url)
            .await
            .context("Failed to connect to Auth database")?;
        let logs = MySqlPoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(logs_url)
            .await
            .context("Failed to connect to Logs database")?;

        info!("All database connection pools created");

        Ok(Databases {
            world,
            character,
            auth,
            logs,
        })
    }

    /// Ping all databases to verify connections
    pub async fn ping_all(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.world)
            .await
            .context("World database ping failed")?;

        sqlx::query("SELECT 1")
            .execute(&self.character)
            .await
            .context("Character database ping failed")?;

        sqlx::query("SELECT 1")
            .execute(&self.auth)
            .await
            .context("Auth database ping failed")?;

        sqlx::query("SELECT 1")
            .execute(&self.logs)
            .await
            .context("Logs database ping failed")?;

        Ok(())
    }

    /// Get a reference to the world database pool
    pub fn world(&self) -> &MySqlPool {
        &self.world
    }

    /// Get a reference to the character database pool
    pub fn character(&self) -> &MySqlPool {
        &self.character
    }

    /// Get a reference to the auth database pool
    pub fn auth(&self) -> &MySqlPool {
        &self.auth
    }

    /// Get a reference to the logs database pool
    pub fn logs(&self) -> &MySqlPool {
        &self.logs
    }
}

/// Helper function to mask password in database URL for logging
fn mask_database_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(slash_pos) = url[..at_pos].rfind(':') {
            let user_part = &url[..slash_pos];
            let rest = &url[at_pos..];
            format!("{}:***{}", user_part, rest)
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

// Re-export commonly used types for convenience

// Auth database types
pub use auth::{
    AccountAccessRow, AccountBannedRow, AccountRepository, AccountRow, AllowedClientRow,
    IpBanRepository, IpBannedRow, RealmCharactersRow, RealmRepository, RealmRow,
};

// Characters database types
pub use characters::{
    CharacterRepository, CharacterRow, GroupRepository, GroupRow, GuildRepository, GuildRow,
    ItemInstanceRow, ItemRepository, MailRepository, MailRow, ReputationRepository,
    SocialRepository,
};
