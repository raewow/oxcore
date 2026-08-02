use anyhow::{bail, Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::{Path, PathBuf};

pub const SCHEMAS: &[&str] = &["auth", "world", "characters", "logs", "web"];

pub async fn connect(url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(3)
        .connect(url)
        .await
        .with_context(|| format!("Failed to connect to {}", mask(url)))
}

/// Create the application namespaces and the shared PostgreSQL migration ledger.
pub async fn ensure_bootstrap(pool: &PgPool) -> Result<()> {
    for schema in SCHEMAS {
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {schema}"))
            .execute(pool)
            .await
            .with_context(|| format!("Failed to create PostgreSQL schema '{schema}'"))?;
    }

    sqlx::query("CREATE SCHEMA IF NOT EXISTS oxcore_meta")
        .execute(pool)
        .await
        .context("Failed to create PostgreSQL metadata schema")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS oxcore_meta.migrations (
            schema_name TEXT NOT NULL,
            id VARCHAR(20) NOT NULL,
            name TEXT NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (schema_name, id)
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create PostgreSQL migrations table")?;
    Ok(())
}

pub async fn migrations_initialized(pool: &PgPool) -> Result<bool> {
    sqlx::query_scalar("SELECT to_regclass('oxcore_meta.migrations') IS NOT NULL")
        .fetch_one(pool)
        .await
        .context("Failed to inspect PostgreSQL migration metadata")
}

pub async fn applied_migrations(pool: &PgPool, schema: &str) -> Result<Vec<String>> {
    validate_schema(schema)?;
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM oxcore_meta.migrations WHERE schema_name = $1 ORDER BY id")
            .bind(schema)
            .fetch_all(pool)
            .await
            .with_context(|| {
                format!("Failed to read applied PostgreSQL migrations for '{schema}'")
            })?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Apply a complete PostgreSQL migration script and record it atomically.
pub async fn run_migration(
    pool: &PgPool,
    schema: &str,
    id: &str,
    name: &str,
    sql: &str,
) -> Result<()> {
    validate_schema(schema)?;
    let mut transaction = pool.begin().await?;
    sqlx::query(&format!("SET LOCAL search_path TO {schema}, oxcore_meta"))
        .execute(&mut *transaction)
        .await?;
    sqlx::raw_sql(sql)
        .execute(&mut *transaction)
        .await
        .with_context(|| {
            format!("Failed applying PostgreSQL migration {id}_{name} in '{schema}'")
        })?;
    sqlx::query("INSERT INTO oxcore_meta.migrations (schema_name, id, name) VALUES ($1, $2, $3)")
        .bind(schema)
        .bind(id)
        .bind(name)
        .execute(&mut *transaction)
        .await
        .with_context(|| format!("Failed recording PostgreSQL migration {id}_{name}"))?;
    transaction.commit().await?;
    Ok(())
}

/// Clear application schemas and their migration history without touching the database or role.
pub async fn reset_application_schemas(pool: &PgPool) -> Result<()> {
    ensure_bootstrap(pool).await?;
    for schema in SCHEMAS {
        sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
            .execute(pool)
            .await
            .with_context(|| format!("Failed to drop PostgreSQL schema '{schema}'"))?;
    }
    sqlx::query("DELETE FROM oxcore_meta.migrations")
        .execute(pool)
        .await
        .context("Failed to clear PostgreSQL migration history")?;
    ensure_bootstrap(pool).await
}

pub fn collect_migrations(migrations_dir: &Path, schema: &str) -> Result<Vec<Migration>> {
    validate_schema(schema)?;
    let dir = migrations_dir.join(schema);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut migrations = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| {
        format!(
            "Failed to read PostgreSQL migration directory {}",
            dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "sql") {
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some((id, name)) = parse_migration_name(&file_name) else {
            bail!(
                "Invalid PostgreSQL migration filename '{}'. Expected YYYYMMDDHHMMSS_name.sql",
                file_name
            );
        };
        migrations.push(Migration {
            id: id.to_string(),
            name: name.to_string(),
            path,
        });
    }
    migrations.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(migrations)
}

pub fn create_migration(migrations_dir: &Path, schema: &str, name: &str) -> Result<PathBuf> {
    validate_schema(schema)?;
    let name = name.to_lowercase().replace(' ', "_");
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        bail!("Migration name must contain only lowercase letters, numbers, and underscores");
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let directory = migrations_dir.join(schema);
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{timestamp}_{name}.sql"));
    if path.exists() {
        bail!("File already exists: {}", path.display());
    }
    std::fs::write(
        &path,
        format!(
            "-- PostgreSQL migration: {schema} / {name}\n-- Created: {timestamp}\n\n-- Write your SQL here\n"
        ),
    )?;
    Ok(path)
}

pub fn validate_schema(schema: &str) -> Result<()> {
    if SCHEMAS.contains(&schema) {
        Ok(())
    } else {
        bail!(
            "Unknown PostgreSQL schema '{schema}'. Valid options: {}",
            SCHEMAS.join(", ")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

fn parse_migration_name(file_name: &str) -> Option<(&str, &str)> {
    let stem = file_name.strip_suffix(".sql")?;
    let (id, name) = stem.split_once('_')?;
    (id.len() == 14 && id.bytes().all(|byte| byte.is_ascii_digit()) && !name.is_empty())
        .then_some((id, name))
}

fn mask(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(colon) = url[..at].rfind(':') {
            return format!("{}:***{}", &url[..colon], &url[at..]);
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_migration_name, validate_schema};

    #[test]
    fn parses_postgres_migration_filename() {
        assert_eq!(
            parse_migration_name("20260802093000_create_accounts.sql"),
            Some(("20260802093000", "create_accounts"))
        );
    }

    #[test]
    fn rejects_invalid_schema_and_filename() {
        assert!(validate_schema("unknown").is_err());
        assert!(parse_migration_name("invalid.sql").is_none());
    }
}
