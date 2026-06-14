import type Database from "better-sqlite3";
import type { ProviderName } from "../../config.js";

export function getSetting(db: Database.Database, key: string): string | null {
  const row = db
    .prepare("SELECT value FROM app_setting WHERE key = ?")
    .get(key) as { value: string } | undefined;
  return row?.value ?? null;
}

export function setSetting(db: Database.Database, key: string, value: string): void {
  db.prepare(
    "INSERT INTO app_setting (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
  ).run(key, value);
}

export function deleteSetting(db: Database.Database, key: string): void {
  db.prepare("DELETE FROM app_setting WHERE key = ?").run(key);
}

export function getActiveProviderOverride(db: Database.Database): {
  name?: ProviderName;
  model?: string;
} {
  const name = getSetting(db, "active_provider") as ProviderName | null;
  const model = getSetting(db, "active_model");
  return {
    ...(name ? { name } : {}),
    ...(model ? { model } : {}),
  };
}
