use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<PgPool>,
    pub characters: Arc<PgPool>,
    pub web: Arc<PgPool>,
    pub logs: Arc<PgPool>,
    pub secure_cookies: bool,
    pub public_origin: String,
}

impl AppState {
    pub async fn connect(config: &Config) -> Result<Self> {
        let auth = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&config.auth_database_url)
            .await
            .context("failed to connect to the auth database")?;
        let web = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&config.web_database_url)
            .await
            .context("failed to connect to the web database")?;
        let characters = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&config.character_database_url)
            .await
            .context("failed to connect to the characters database")?;
        let logs = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .connect(&config.logs_database_url)
            .await
            .context("failed to connect to the logs database")?;

        Ok(Self {
            auth: Arc::new(auth),
            characters: Arc::new(characters),
            web: Arc::new(web),
            logs: Arc::new(logs),
            secure_cookies: config.secure_cookies(),
            public_origin: config.public_base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn ready(&self) -> bool {
        sqlx::query("SELECT 1").execute(&*self.auth).await.is_ok()
            && sqlx::query("SELECT 1")
                .execute(&*self.characters)
                .await
                .is_ok()
            && sqlx::query("SELECT 1").execute(&*self.web).await.is_ok()
            && sqlx::query("SELECT 1").execute(&*self.logs).await.is_ok()
    }
}
