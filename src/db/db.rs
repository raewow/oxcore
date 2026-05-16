use anyhow::{Context, Result};
use sqlx::mysql::MySqlConnectOptions;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn connect(url: &str) -> Result<MySqlPool> {
    MySqlPoolOptions::new()
        .max_connections(3)
        .connect(url)
        .await
        .with_context(|| format!("Failed to connect to {}", mask(url)))
}

/// Try to connect, returning None with a message if the DB doesn't exist yet.
pub async fn try_connect(url: &str) -> Option<MySqlPool> {
    match connect(url).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            println!("  Could not connect: {e}");
            None
        }
    }
}

pub async fn ensure_migrations_table(pool: &MySqlPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS `migrations` (
            `id` VARCHAR(20) NOT NULL,
            `name` VARCHAR(255) NOT NULL DEFAULT '',
            `applied_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (`id`)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
    )
    .execute(pool)
    .await
    .context("Failed to create migrations table")?;
    Ok(())
}

pub async fn applied_migrations(pool: &MySqlPool) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM migrations ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub async fn run_migration(pool: &MySqlPool, id: &str, name: &str, sql: &str) -> Result<()> {
    // Execute each statement in the file
    for stmt in split_statements(sql) {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .with_context(|| format!("Failed executing: {}...", &stmt[..stmt.len().min(80)]))?;
    }
    // Record it
    sqlx::query("INSERT INTO migrations (id, name) VALUES (?, ?)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .context("Failed to record migration")?;
    Ok(())
}

pub async fn base_tables_applied(pool: &MySqlPool) -> Result<bool> {
    // If any tables exist beyond 'migrations' itself, consider base applied
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.TABLES
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME != 'migrations'",
    )
    .fetch_one(pool)
    .await?;
    Ok(count.0 > 0)
}

pub async fn apply_base(pool: &MySqlPool, base_dir: &std::path::Path) -> Result<()> {
    if !base_dir.exists() {
        println!("  No base directory at {}", base_dir.display());
        return Ok(());
    }

    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(pool)
        .await?;
    sqlx::query("SET SQL_MODE = ''").execute(pool).await?;

    let mut files: Vec<_> = std::fs::read_dir(base_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "sql"))
        .collect();
    files.sort_by_key(|e| e.file_name());

    println!("  Applying {} base table file(s)...", files.len());
    for entry in files {
        let path = entry.path();
        let sql = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        for stmt in split_statements(&sql) {
            let _ = sqlx::query(stmt).execute(pool).await; // tolerate "already exists"
        }
    }

    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(pool)
        .await?;
    Ok(())
}

fn split_statements(sql: &str) -> Vec<&str> {
    sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with("--") && !s.starts_with("/*"))
        .collect()
}

fn mask(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(colon) = url[..at].rfind(':') {
            return format!("{}:***{}", &url[..colon], &url[at..]);
        }
    }
    url.to_string()
}
