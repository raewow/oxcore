use anyhow::Result;
use std::io::Write;

use crate::config::Config;
use crate::db;

const DATABASES: &[(&str, fn(&Config) -> &str)] = &[
    ("world", |c| &c.world_url),
    ("auth", |c| &c.auth_url),
    ("characters", |c| &c.character_url),
    ("logs", |c| &c.logs_url),
];

pub async fn run(config: &Config, yes: bool) -> Result<()> {
    if !yes && !confirm()? {
        println!("Aborted.");
        return Ok(());
    }

    for (db_name, url_fn) in DATABASES {
        let url = url_fn(config);
        println!("[{db_name}] Dropping and recreating...");
        db::reset_database(url).await?;
    }
    println!();

    super::migrate::run(config).await
}

fn confirm() -> Result<bool> {
    println!("This will DROP and recreate the following databases, destroying all data:");
    for (db_name, _) in DATABASES {
        println!("  - {db_name}");
    }
    print!("Continue? [y/N] ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}
