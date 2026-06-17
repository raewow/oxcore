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
        .with_context(|| {
            format!(
                "Failed to connect to MySQL server at {}",
                mask(&parts.server_url)
            )
        })?;

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
    for stmt in split_statements(sql) {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .with_context(|| format!("Failed executing: {}...", &stmt[..stmt.len().min(80)]))?;
    }
    sqlx::query("INSERT INTO migrations (id, name) VALUES (?, ?)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .context("Failed to record migration")?;
    Ok(())
}

pub async fn base_tables_applied(pool: &MySqlPool) -> Result<bool> {
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
        let mut failed = 0usize;
        for stmt in split_statements(&sql) {
            if sqlx::query(stmt).execute(pool).await.is_err() {
                failed += 1;
            }
        }
        if failed > 0 {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            println!("    WARNING: {failed} statement(s) failed in {name}");
        }
    }

    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(pool)
        .await?;
    Ok(())
}

/// Split a SQL script into individual statements on top-level `;`.
///
/// `;` characters inside single-quoted string literals or backtick-quoted
/// identifiers are ignored, so mysqldump INSERTs whose text columns contain
/// semicolons (e.g. quest Details/Objectives) are kept intact. Backslash
/// escapes inside strings are honoured.
fn split_statements(sql: &str) -> Vec<&str> {
    let bytes = sql.as_bytes();
    let mut statements = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_backtick = false;
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i];
        if in_single {
            match c {
                // Backslash escapes the next byte (\\, \', \n, ...).
                b'\\' => {
                    i += 2;
                    continue;
                }
                b'\'' => in_single = false,
                _ => {}
            }
        } else if in_backtick {
            if c == b'`' {
                in_backtick = false;
            }
        } else {
            match c {
                b'\'' => in_single = true,
                b'`' => in_backtick = true,
                b';' => {
                    statements.push(&sql[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        i += 1;
    }

    if start < sql.len() {
        statements.push(&sql[start..]);
    }

    statements
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with("--") && !s.starts_with("/*"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::split_statements;

    #[test]
    fn keeps_semicolons_inside_string_literals() {
        // Mirrors a mysqldump quest_template INSERT: semicolons live inside the
        // quoted text columns and must not split the statement.
        let sql = "INSERT INTO `quest_template` VALUES \
            (1,'Go to the site; then return','Details; more'),\
            (2,'Another; quest','Objectives; here');\n\
            INSERT INTO `quest_template` VALUES (3,'x','y');";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 2, "expected 2 statements, got {stmts:?}");
        assert!(stmts[0].contains("site; then return"));
        assert!(stmts[1].starts_with("INSERT INTO `quest_template` VALUES (3"));
    }

    #[test]
    fn handles_escaped_quotes_and_backticks() {
        // \' is an escaped quote (string continues); a ; right after must stay inside.
        let sql =
            "INSERT INTO `t` VALUES ('it\\'s a test; really','ok');CREATE TABLE `a;b` (x int);";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 2, "got {stmts:?}");
        assert!(stmts[0].contains("it\\'s a test; really"));
        assert!(stmts[1].contains("`a;b`"));
    }

    #[test]
    fn drops_comments_and_empty() {
        let sql = "-- a comment\n/*!40101 SET X */;\nSELECT 1;;";
        let stmts = split_statements(sql);
        assert_eq!(stmts, vec!["SELECT 1"]);
    }
}

fn mask(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(colon) = url[..at].rfind(':') {
            return format!("{}:***{}", &url[..colon], &url[at..]);
        }
    }
    url.to_string()
}
