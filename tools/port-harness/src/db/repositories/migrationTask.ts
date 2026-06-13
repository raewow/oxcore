import type Database from "better-sqlite3";
import type { MigrationTask, TaskStatus, TaskWithDetails } from "../../models/index.js";

export function upsertTask(
  db: Database.Database,
  sourceSymbolId: number,
  fields: Partial<{
    flow_id: number | null;
    target_rust_file: string;
    status: TaskStatus;
    notes: string;
    rust_symbol_name: string;
  }> = {},
): number {
  const existing = db
    .prepare("SELECT id FROM migration_task WHERE source_symbol_id = ?")
    .get(sourceSymbolId) as { id: number } | undefined;

  if (existing) {
    const sets: string[] = ["updated_at = datetime('now')"];
    const values: unknown[] = [];
    for (const [key, val] of Object.entries(fields)) {
      if (val !== undefined) {
        sets.push(`${key} = ?`);
        values.push(val);
      }
    }
    values.push(existing.id);
    db.prepare(`UPDATE migration_task SET ${sets.join(", ")} WHERE id = ?`).run(...values);
    return existing.id;
  }

  const result = db
    .prepare(
      `INSERT INTO migration_task (source_symbol_id, flow_id, target_rust_file, status, notes, rust_symbol_name)
       VALUES (?, ?, ?, ?, ?, ?)`,
    )
    .run(
      sourceSymbolId,
      fields.flow_id ?? null,
      fields.target_rust_file ?? null,
      fields.status ?? "discovered",
      fields.notes ?? null,
      fields.rust_symbol_name ?? null,
    );
  return Number(result.lastInsertRowid);
}

export function getTaskById(db: Database.Database, id: number): MigrationTask | undefined {
  return db.prepare("SELECT * FROM migration_task WHERE id = ?").get(id) as
    | MigrationTask
    | undefined;
}

export function getTaskBySymbolId(
  db: Database.Database,
  symbolId: number,
): MigrationTask | undefined {
  return db
    .prepare("SELECT * FROM migration_task WHERE source_symbol_id = ?")
    .get(symbolId) as MigrationTask | undefined;
}

export function updateTaskStatus(db: Database.Database, id: number, status: TaskStatus): void {
  db.prepare(
    `UPDATE migration_task SET status = ?, updated_at = datetime('now') WHERE id = ?`,
  ).run(status, id);
}

export function bulkUpdateTasks(
  db: Database.Database,
  ids: number[],
  fields: Partial<{
    status: TaskStatus;
    notes: string;
    target_rust_file: string;
    flow_id: number;
  }>,
): void {
  const sets: string[] = ["updated_at = datetime('now')"];
  const values: unknown[] = [];
  for (const [key, val] of Object.entries(fields)) {
    if (val !== undefined) {
      sets.push(`${key} = ?`);
      values.push(val);
    }
  }
  const placeholders = ids.map(() => "?").join(",");
  db.prepare(
    `UPDATE migration_task SET ${sets.join(", ")} WHERE id IN (${placeholders})`,
  ).run(...values, ...ids);
}

export interface TaskFilter {
  file?: string;
  status?: TaskStatus;
  flow?: string;
  q?: string;
  missingDocs?: boolean;
  blocked?: boolean;
  limit?: number;
  offset?: number;
}

export function listTasksWithDetails(
  db: Database.Database,
  filter: TaskFilter = {},
): { tasks: TaskWithDetails[]; total: number } {
  const conditions: string[] = ["1=1"];
  const params: unknown[] = [];

  if (filter.file) {
    conditions.push("cs.file LIKE ?");
    params.push(`%${filter.file}%`);
  }
  if (filter.status) {
    conditions.push("mt.status = ?");
    params.push(filter.status);
  }
  if (filter.flow) {
    conditions.push("bf.name = ?");
    params.push(filter.flow);
  }
  if (filter.q) {
    conditions.push("(cs.name LIKE ? OR mt.notes LIKE ?)");
    params.push(`%${filter.q}%`, `%${filter.q}%`);
  }
  if (filter.missingDocs) {
    conditions.push("mt.status = 'discovered'");
  }
  if (filter.blocked) {
    conditions.push("mt.status = 'blocked'");
  }

  const where = conditions.join(" AND ");
  const countRow = db
    .prepare(
      `SELECT COUNT(*) as c FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id
       LEFT JOIN business_flow bf ON bf.id = mt.flow_id
       WHERE ${where}`,
    )
    .get(...params) as { c: number };

  const limit = filter.limit ?? 100;
  const offset = filter.offset ?? 0;

  const tasks = db
    .prepare(
      `SELECT mt.*, cs.name as symbol_name, cs.file as symbol_file,
              cs.start_line, cs.end_line, bf.name as flow_name, bf.risk_level,
              (SELECT COUNT(*) FROM behaviour_claim bc WHERE bc.symbol_id = cs.id) as claim_count,
              (SELECT COUNT(*) FROM test_fixture tf WHERE tf.symbol_id = cs.id) as fixture_count
       FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id
       LEFT JOIN business_flow bf ON bf.id = mt.flow_id
       WHERE ${where}
       ORDER BY cs.file, cs.start_line
       LIMIT ? OFFSET ?`,
    )
    .all(...params, limit, offset) as TaskWithDetails[];

  return { tasks, total: countRow.c };
}

export function getStatusCounts(
  db: Database.Database,
): { status: string; count: number }[] {
  return db
    .prepare("SELECT status, COUNT(*) as count FROM migration_task GROUP BY status")
    .all() as { status: string; count: number }[];
}

export function getFileProgress(
  db: Database.Database,
): { file: string; total: number; documented: number }[] {
  return db
    .prepare(
      `SELECT cs.file, COUNT(*) as total,
              SUM(CASE WHEN mt.status NOT IN ('discovered') THEN 1 ELSE 0 END) as documented
       FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id
       GROUP BY cs.file`,
    )
    .all() as { file: string; total: number; documented: number }[];
}
