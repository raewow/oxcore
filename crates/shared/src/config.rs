use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

pub fn load_toml<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let contents = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read: {}", path.as_ref().display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("Failed to parse: {}", path.as_ref().display()))
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
