use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use wow_server::shared::config::{find_config_file, RootConfig};

#[derive(Debug)]
pub struct Config {
    pub world_url: String,
    pub character_url: String,
    pub auth_url: String,
    pub logs_url: String,
    /// Absolute path to sql/base/
    pub base_dir: PathBuf,
    /// Absolute path to sql/migrations/
    pub migrations_dir: PathBuf,
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
                 must use 127.0.0.1 instead of the 'mysql' service hostname:\n\
                   mysql://root:root@127.0.0.1:3306/world\n\
                 \n\
                 Or pass a different file: cargo run --bin db -- -c /path/to/config.toml",
                path.display()
            );
        }

        let root = RootConfig::load(&path)
            .with_context(|| format!("Failed to load config: {}", path.display()))?;
        let w = root.world;

        let sql_dir = find_sql_dir();

        Ok(Config {
            world_url: w.world_database_url,
            character_url: w.character_database_url,
            auth_url: w.login_database_url,
            logs_url: w.logs_database_url,
            base_dir: sql_dir.join("base"),
            migrations_dir: sql_dir.join("migrations"),
        })
    }
}

/// Walk upward from CWD to find the sql/ directory (sits at repo root).
fn find_sql_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd.as_path();
        loop {
            let candidate = dir.join("sql");
            if candidate.exists() {
                return candidate;
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
        }
    }
    PathBuf::from("sql")
}
