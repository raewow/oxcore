//! Database access for the bnet server.
//!
//! Shares the auth database with `oxcore-auth` — the same `account` table, reached through the
//! shared [`AccountRepository`]. The bnet server only needs the account/credential surface, so
//! it holds just that repository rather than the fuller `Database` the auth server builds.

use std::sync::Arc;

use anyhow::{Context, Result};
use oxcore_shared::database::auth::repositories::AccountRepository;
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct Database {
    pub accounts: AccountRepository,
}

impl Database {
    pub async fn connect(login_database_url: &str) -> Result<Self> {
        let pool = Arc::new(
            MySqlPool::connect(login_database_url)
                .await
                .context("Failed to connect to auth database")?,
        );
        Ok(Self {
            accounts: AccountRepository::new(pool),
        })
    }
}
