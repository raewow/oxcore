use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub world_url: String,
    pub character_url: String,
    pub auth_url: String,
    /// Optional PostgreSQL URL used only by the `db pg` migration commands.
    pub postgres_url: Option<String>,
    /// Absolute path to sql/base/
    pub base_dir: PathBuf,
    /// Absolute path to sql/migrations/
    pub migrations_dir: PathBuf,
    /// Absolute path to sql/postgres/migrations/
    pub postgres_migrations_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    world: WorldConfig,
    postgres: Option<PostgresConfig>,
}

#[derive(Debug, Deserialize)]
struct PostgresConfig {
    database_url: String,
}

#[derive(Debug, Deserialize)]
struct WorldConfig {
    world_database_url: String,
    character_database_url: String,
    login_database_url: String,
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
            postgres_url: root.postgres.map(|postgres| postgres.database_url),
            base_dir: sql_dir.join("base"),
            migrations_dir: sql_dir.join("migrations"),
            postgres_migrations_dir: sql_dir.join("postgres/migrations"),
        })
    }

    pub fn postgres_url(&self) -> Result<&str> {
        self.postgres_url.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "PostgreSQL is not configured. Add [postgres] database_url to config.toml."
            )
        })
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
