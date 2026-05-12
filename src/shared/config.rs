use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct RootConfig {
    pub auth: crate::auth::config::Config,
    pub world: crate::world::config::Config,
}

impl RootConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read: {}", path.as_ref().display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse: {}", path.as_ref().display()))
    }
}

pub fn find_config_file() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join("config.toml");
            if path.exists() {
                return path;
            }
        }
    }
    PathBuf::from("config.toml")
}
