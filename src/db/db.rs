use anyhow::{Context, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn connect(url: &str) -> Result<MySqlPool> {
    MySqlPoolOptions::new()
        .max_connections(3)
        .connect(url)
        .await
        .with_context(|| format!("Failed to connect to {}", mask(url)))
}

/// Try to connect, creating the database first if needed.
pub async fn try_connect(url: &str) -> Option<MySqlPool> {
    if let Err(e) = ensure_database(url).await {
        print_connection_error(url, &e);
        return None;
    }

    match connect(url).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            print_connection_error(url, &e);
            None
        }
    }
}

async fn ensure_database(url: &str) -> Result<()> {
    let parts = parse_mysql_url(url).context("Invalid MySQL connection URL")?;

    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&parts.server_url)
        .await
        .with_context(|| format!("Failed to connect to MySQL server at {}", mask(&parts.server_url)))?;

    let db = parts.database.replace('`', "``");
    let sql = format!(
        "CREATE DATABASE IF NOT EXISTS `{db}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
    );
    sqlx::query(&sql)
        .execute(&pool)
        .await
        .with_context(|| format!("Failed to create database '{}'", parts.database))?;

    Ok(())
}

struct MysqlUrlParts {
    server_url: String,
    database: String,
}

fn parse_mysql_url(url: &str) -> Option<MysqlUrlParts> {
    let rest = url.strip_prefix("mysql://")?;
    let (user_pass, host_db) = rest.split_once('@')?;
    let (host_port, database) = host_db.rsplit_once('/')?;
    if database.is_empty() {
        return None;
    }

    Some(MysqlUrlParts {
        server_url: format!("mysql://{user_pass}@{host_port}"),
        database: database.to_string(),
    })
}

fn print_connection_error(url: &str, err: &anyhow::Error) {
    println!("  Could not connect to {}", mask(url));
    println!("  {}", lowest_error_message(err));

    if let Some(host) = parse_mysql_host(url) {
        if host == "mysql" {
            println!();
            println!("  The hostname 'mysql' only works inside the Docker/Podman network.");
            println!("  This tool runs on your host, so use 127.0.0.1 instead:");
            if let Some(example) = local_dev_url(url) {
                println!("    {example}");
            }
            println!();
            println!("  Start MySQL with: podman compose up -d");
            println!("  Password is 'root' (see docker-compose.yml).");
        }
    }
}

fn lowest_error_message(err: &anyhow::Error) -> String {
    err.chain()
        .last()
        .map(|e| e.to_string())
        .unwrap_or_else(|| err.to_string())
}

fn parse_mysql_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("mysql://")?;
    let after_at = rest.split('@').nth(1)?;
    after_at.split('/').next()?.split(':').next()
}

fn local_dev_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("mysql://")?;
    let (user_pass, host_db) = rest.split_once('@')?;
    let (host_port, db) = host_db.split_once('/')?;

    let user = user_pass.split(':').next()?;
    let port = host_port.split(':').nth(1).unwrap_or("3306");

    Some(format!("mysql://{user}:root@127.0.0.1:{port}/{db}"))
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
