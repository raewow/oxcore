use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<MySqlPool>,
    pub web: Arc<MySqlPool>,
    pub secure_cookies: bool,
}

impl AppState {
    pub async fn connect(config: &Config) -> Result<Self> {
        let auth = MySqlPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&config.auth_database_url)
            .await
            .context("failed to connect to the auth database")?;
        let web = MySqlPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&config.web_database_url)
            .await
            .context("failed to connect to the web database")?;

        Ok(Self {
            auth: Arc::new(auth),
            web: Arc::new(web),
            secure_cookies: config.secure_cookies(),
        })
    }

    pub async fn ready(&self) -> bool {
        sqlx::query("SELECT 1").execute(&*self.auth).await.is_ok()
            && sqlx::query("SELECT 1").execute(&*self.web).await.is_ok()
    }
}
