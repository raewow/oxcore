//! Database access for the bnet server.
//!
//! Shares the auth database with `oxcore-auth` — the same `account` table, reached through the
//! shared [`AccountRepository`]. The bnet server only needs the account/credential surface, so
//! it holds just that repository rather than the fuller `Database` the auth server builds.

use std::sync::Arc;

use anyhow::{Context, Result};
use oxcore_db::database::auth::repositories::{AccountRepository, RealmRepository};
use sqlx::PgPool;

#[derive(Clone)]
pub struct Database {
    pub accounts: AccountRepository,
    pub realms: RealmRepository,
}

impl Database {
    pub async fn connect(login_database_url: &str) -> Result<Self> {
        let pool = Arc::new(
            PgPool::connect(login_database_url)
                .await
                .context("Failed to connect to auth database")?,
        );
        Ok(Self::from_pool(pool))
    }

    /// Build a database handle whose pool connects on first use rather than up front. Used by
    /// tests and by handlers that may never touch the database on a given connection.
    pub fn connect_lazy(login_database_url: &str) -> Result<Self> {
        let pool = Arc::new(
            PgPool::connect_lazy(login_database_url)
                .context("Failed to prepare lazy auth database pool")?,
        );
        Ok(Self::from_pool(pool))
    }

    fn from_pool(pool: Arc<PgPool>) -> Self {
        Self {
            accounts: AccountRepository::new(pool.clone()),
            realms: RealmRepository::new(pool),
        }
    }
}
