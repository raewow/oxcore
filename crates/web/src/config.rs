use std::net::{IpAddr, SocketAddr};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RootConfig {
    pub web: Option<Config>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind_ip: IpAddr,
    pub port: u16,
    pub public_base_url: String,
    pub auth_database_url: String,
    pub character_database_url: String,
    pub web_database_url: String,
    pub logs_database_url: String,
}

impl Config {
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind_ip, self.port)
    }

    pub fn validate(&self) -> Result<()> {
        let url = self.public_base_url.trim_end_matches('/');
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            anyhow::bail!("web.public_base_url must start with http:// or https://");
        }

        url.parse::<axum::http::Uri>()
            .context("web.public_base_url must be a valid URI")?;
        if !self.auth_database_url.starts_with("mysql://") {
            anyhow::bail!("web.auth_database_url must be a MySQL connection URL");
        }
        if !self.web_database_url.starts_with("mysql://") {
            anyhow::bail!("web.web_database_url must be a MySQL connection URL");
        }
        if !self.character_database_url.starts_with("mysql://") {
            anyhow::bail!("web.character_database_url must be a MySQL connection URL");
        }
        if !self.logs_database_url.starts_with("mysql://") {
            anyhow::bail!("web.logs_database_url must be a MySQL connection URL");
        }
        Ok(())
    }

    pub fn secure_cookies(&self) -> bool {
        self.public_base_url.starts_with("https://")
    }
}
