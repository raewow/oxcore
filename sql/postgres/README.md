# PostgreSQL Migrations

`db pg migrate` initializes the `auth`, `world`, `characters`, `logs`, and `web` schemas in the
`oxcore` PostgreSQL database. Place schema-specific migrations in:

```text
sql/postgres/migrations/<schema>/YYYYMMDDHHMMSS_<name>.sql
```

The directories are created automatically by `db pg new <schema> <name>`. PostgreSQL migrations
are intentionally separate from the MySQL dumps and migrations under `sql/base/` and
`sql/migrations/`; do not run MySQL SQL through the PostgreSQL command lane.

Current coverage: the `logs` schema has its initial PostgreSQL migration. Runtime log pools and
queries still target MySQL until their dedicated query-port batch is complete.
