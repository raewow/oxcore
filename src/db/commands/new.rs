use anyhow::{bail, Result};
use chrono::Utc;
use std::path::Path;

const VALID_DBS: &[&str] = &["world", "auth", "characters", "logs"];

pub fn run(db: &str, name: &str, migrations_dir: &Path) -> Result<()> {
    if !VALID_DBS.contains(&db) {
        bail!("Unknown database '{}'. Valid options: {}", db, VALID_DBS.join(", "));
    }

    // Normalise name: lowercase, spaces → underscores
    let name = name.to_lowercase().replace(' ', "_");

    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let filename = format!("{timestamp}_{db}_{name}.sql");

    std::fs::create_dir_all(migrations_dir)?;
    let path = migrations_dir.join(&filename);

    if path.exists() {
        bail!("File already exists: {}", path.display());
    }

    let content = format!(
        "-- Migration: {db} / {name}\n\
         -- Created: {timestamp}\n\
         \n\
         -- Write your SQL here\n"
    );
    std::fs::write(&path, content)?;

    println!("Created: {}", path.display());
    Ok(())
}
