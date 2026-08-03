use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    /// Absolute path to this crate's PostgreSQL migrations.
    pub postgres_migrations_dir: PathBuf,
    pub postgres_url: String,
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    postgres: PostgresConfig,
}

#[derive(Debug, Deserialize)]
struct PostgresConfig {
    database_url: String,
}

impl RootConfig {
    fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read: {}", path.as_ref().display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse: {}", path.as_ref().display()))
    }
}

impl Config {
    pub fn load(config_path: Option<PathBuf>) -> Result<Self> {
        let path = config_path.unwrap_or_else(find_config_file);

        if !path.exists() {
            bail!(
                "Config file not found: {}\n\
                 \n\
                 Create one from the example:\n\
                   cp config.toml.example config.toml\n\
                 \n\
                 When running this tool on your host (not inside Docker), database URLs\n\
                 must use 127.0.0.1 to the PostgreSQL service:\n\
                    postgres://oxcore:oxcore@127.0.0.1:5432/oxcore\n\
                 \n\
                 Or pass a different file: cargo run --bin db -- -c /path/to/config.toml",
                path.display()
            );
        }

        let root = RootConfig::load(&path)
            .with_context(|| format!("Failed to load config: {}", path.display()))?;
        Ok(Config {
            postgres_migrations_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"),
            postgres_url: root.postgres.database_url,
        })
    }

    pub fn postgres_url(&self) -> Result<&str> {
        Ok(&self.postgres_url)
    }
}

fn find_config_file() -> PathBuf {
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
