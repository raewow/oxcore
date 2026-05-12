use anyhow::Result;

use crate::config::Config;
use crate::db;
use super::migrate::collect_migrations;

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

        // Check migrations table exists
        let has_table: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM information_schema.TABLES
             WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'migrations'",
        )
        .fetch_one(&pool)
        .await?;

        if has_table.0 == 0 {
            println!("  Not initialized (run: cargo run --bin db -- migrate)");
            println!();
            continue;
        }

        // Base tables
        let base_dir = config.base_dir.join(db_name);
        let base_file_count = count_sql_files(&base_dir);
        let base_applied = db::base_tables_applied(&pool).await?;
        if base_applied {
            println!("  Base:    ok ({base_file_count} tables)");
        } else {
            println!("  Base:    not applied ({base_file_count} files pending)");
        }

        let applied = db::applied_migrations(&pool).await?;
        let applied_set: std::collections::HashSet<_> = applied.iter().cloned().collect();

        let mut migrations = collect_migrations(&config.migrations_dir, db_name)?;
        migrations.sort_by(|a, b| a.0.cmp(&b.0));

        let pending: Vec<_> = migrations
            .iter()
            .filter(|(id, _, _)| !applied_set.contains(id))
            .collect();

        println!("  Applied: {}", applied.len());
        println!("  Pending: {}", pending.len());
        for (id, name, _) in &pending {
            println!("    + {id}_{db_name}_{name}.sql");
        }

        println!();
    }

    Ok(())
}

fn count_sql_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |x| x == "sql"))
                .count()
        })
        .unwrap_or(0)
}
