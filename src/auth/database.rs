use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::info;

use crate::auth::config::Config;
use crate::shared::database::auth::repositories::*;

#[derive(Clone)]
pub struct Database {
    pool: Arc<MySqlPool>,
    pub accounts: AccountRepository,
    pub ip_bans: IpBanRepository,
    pub realms: RealmRepository,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Connecting to database...");

        let pool = Arc::new(
            MySqlPool::connect(&config.login_database_url)
                .await
                .context("Failed to connect to database")?,
        );

        info!("Database connection pool created");

        Ok(Database {
            accounts: AccountRepository::new(pool.clone()),
            ip_bans: IpBanRepository::new(pool.clone()),
            realms: RealmRepository::new(pool.clone()),
            pool,
        })
    }

    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&*self.pool)
            .await
            .context("Database ping failed")?;

        Ok(())
    }

    pub async fn cleanup_expired_bans(&self) -> Result<()> {
        self.accounts.deactivate_expired_bans().await?;
        self.ip_bans.delete_expired_bans().await?;
        info!("Expired bans cleaned up");
        Ok(())
    }
}
