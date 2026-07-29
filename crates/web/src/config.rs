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
        Ok(())
    }
}
