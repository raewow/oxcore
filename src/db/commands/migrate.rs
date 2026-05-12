use anyhow::Result;

use crate::config::Config;
use crate::db;

const DATABASES: &[(&str, fn(&Config) -> &str)] = &[
    ("world", |c| &c.world_url),
    ("auth", |c| &c.auth_url),
    ("characters", |c| &c.character_url),
    ("logs", |c| &c.logs_url),
];

pub async fn run(config: &Config) -> Result<()> {
    for (db_name, url_fn) in DATABASES {
        let url = url_fn(config);
        println!("[{db_name}]");

        let Some(pool) = db::try_connect(url).await else {
            continue;
        };

        db::ensure_migrations_table(&pool).await?;

        // Apply base tables if the DB is empty
        if !db::base_tables_applied(&pool).await? {
            let base_dir = config.base_dir.join(db_name);
            db::apply_base(&pool, &base_dir).await?;
        }

        // Find and apply pending migrations
        let applied = db::applied_migrations(&pool).await?;
        let applied_set: std::collections::HashSet<_> = applied.iter().collect();

        let mut migrations = collect_migrations(&config.migrations_dir, db_name)?;
        migrations.sort_by(|a, b| a.0.cmp(&b.0)); // sort by id

        let pending: Vec<_> = migrations
            .iter()
            .filter(|(id, _, _)| !applied_set.contains(id))
            .collect();

        if pending.is_empty() {
            println!("  No pending migrations");
        } else {
            println!("  Applying {} migration(s)...", pending.len());
            for (id, name, path) in pending {
                println!("    {id}_{db_name}_{name}.sql");
                let sql = std::fs::read_to_string(path)?;
                db::run_migration(&pool, id, name, &sql).await?;
            }
        }

        println!();
    }

    Ok(())
}

/// Returns (id, name, path) for all migrations matching this db
pub fn collect_migrations(
    migrations_dir: &std::path::Path,
    db_name: &str,
) -> Result<Vec<(String, String, std::path::PathBuf)>> {
    let mut result = Vec::new();

    if !migrations_dir.exists() {
        return Ok(result);
    }

    let prefix = format!("_{db_name}_");
    for entry in std::fs::read_dir(migrations_dir)? {
        let entry = entry?;
        let fname = entry.file_name();
        let fname = fname.to_string_lossy();

        if !fname.ends_with(".sql") || !fname.contains(&prefix) {
            continue;
        }

        // filename format: YYYYMMDDHHMMSS_<db>_<name>.sql
        if let Some(id) = fname.get(..14) {
            if id.chars().all(|c| c.is_ascii_digit()) {
                // name = everything after "YYYYMMDDHHMMSS_<db>_", before ".sql"
                let after_db = &fname[14 + 1 + db_name.len() + 1..fname.len() - 4];
                result.push((
                    id.to_string(),
                    after_db.to_string(),
                    entry.path(),
                ));
            }
        }
    }

    Ok(result)
}
