import Database from "better-sqlite3";
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { getPackageRoot } from "../config.js";

let dbInstance: Database.Database | null = null;

export function getDb(dbPath?: string): Database.Database {
  if (!dbInstance) {
    const path = dbPath ?? join(getPackageRoot(), "port_harness.db");
    dbInstance = new Database(path);
    dbInstance.pragma("journal_mode = WAL");
    dbInstance.pragma("foreign_keys = ON");
    runMigrations(dbInstance);
  }
  return dbInstance;
}

export function closeDb(): void {
  if (dbInstance) {
    dbInstance.close();
    dbInstance = null;
  }
}

function runMigrations(db: Database.Database): void {
  db.exec(`
    CREATE TABLE IF NOT EXISTS _migrations (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL UNIQUE,
      applied_at TEXT NOT NULL DEFAULT (datetime('now'))
    )
  `);

  const schemaDir = join(getPackageRoot(), "schema");
  if (!existsSync(schemaDir)) return;

  const files = readdirSync(schemaDir)
    .filter((f) => f.endsWith(".sql"))
    .sort();

  const applied = new Set(
    db.prepare("SELECT name FROM _migrations").all().map((r) => (r as { name: string }).name),
  );

  for (const file of files) {
    if (applied.has(file)) continue;
    const sql = readFileSync(join(schemaDir, file), "utf-8");
    db.exec(sql);
    db.prepare("INSERT INTO _migrations (name) VALUES (?)").run(file);
  }
}
