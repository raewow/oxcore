mod config;
mod server;

use anyhow::{Context, Result};
use oxcore_shared::config::{find_config_file, load_toml};
use tracing_subscriber::EnvFilter;

use crate::config::RootConfig;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("oxcore_web=info,tower_http=info")),
        )
        .with_target(false)
        .compact()
        .init();

    let config_path = std::env::args()
        .nth(1)
        .map(Into::into)
        .unwrap_or_else(find_config_file);
    let root: RootConfig = load_toml(&config_path).with_context(|| {
        format!(
            "failed to load configuration from {}",
            config_path.display()
        )
    })?;
    let config = root.web.context("[web] config section missing")?;

    server::serve(config).await
}
