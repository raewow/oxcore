# PostgreSQL Migrations

`db pg migrate` initializes the `auth`, `world`, `characters`, `logs`, and `web` schemas in the
`oxcore` PostgreSQL database. Place schema-specific migrations in:

```text
crates/db/migrations/<schema>/YYYYMMDDHHMMSS_<name>.sql
```

The directories are created automatically by `db pg new <schema> <name>`.

Each schema has exactly one `base_tables` migration. Checked-in reference data uses ordered
`base_data_1`, `base_data_2`, and subsequent files; the world base data is split on complete SQL
statement boundaries.

All runtime services use PostgreSQL. `sql/migrations/` and `sql/base/` are historical MySQL
reference material. PostgreSQL migrations include the current base data, so a fresh database needs
no separate importer:

```sh
# Reset application schemas, then apply all PostgreSQL schema and base-data migrations.
podman compose up -d postgres
cargo run -p oxcore-db --bin db -- pg fresh --yes
```

Each migration runs in the database command's transaction with its schema search path. Base-data
migrations use schema-qualified inserts, convert MySQL zero/`NaN` game-event date sentinels to
PostgreSQL's year-1 sentinel, and synchronize owned identity/serial sequences. Apply them to a
fresh database; they do not truncate tables or resolve duplicate keys.
