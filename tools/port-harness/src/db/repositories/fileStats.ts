import type Database from "better-sqlite3";
import { filePathMatches } from "../../files/paths.js";

export interface DbFileStats {
  file: string;
  symbol_count: number;
  discovered: number;
  documented: number;
  blocked: number;
  flow_count: number;
  by_status: Record<string, number>;
}

const STATUS_COLUMNS = [
  "discovered",
  "documented",
  "fixture_defined",
  "rust_planned",
  "rust_ported",
  "rust_compiled",
  "verified",
  "reviewed",
  "done",
  "blocked",
] as const;

export function getAllFileStats(db: Database.Database): DbFileStats[] {
  const rows = db
    .prepare(
      `SELECT cs.file,
              COUNT(*) as symbol_count,
              SUM(CASE WHEN mt.status = 'discovered' THEN 1 ELSE 0 END) as discovered,
              SUM(CASE WHEN mt.status NOT IN ('discovered') THEN 1 ELSE 0 END) as documented,
              SUM(CASE WHEN mt.status = 'blocked' THEN 1 ELSE 0 END) as blocked,
              COUNT(DISTINCT mt.flow_id) as flow_count,
              ${STATUS_COLUMNS.map((s) => `SUM(CASE WHEN mt.status = '${s}' THEN 1 ELSE 0 END) as status_${s}`).join(",\n              ")}
       FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id
       GROUP BY cs.file`,
    )
    .all() as Record<string, number | string>[];

  return rows.map((row) => {
    const by_status: Record<string, number> = {};
    for (const s of STATUS_COLUMNS) {
      by_status[s] = Number(row[`status_${s}`] ?? 0);
    }
    return {
      file: String(row.file),
      symbol_count: Number(row.symbol_count),
      discovered: Number(row.discovered ?? 0),
      documented: Number(row.documented ?? 0),
      blocked: Number(row.blocked ?? 0),
      flow_count: Number(row.flow_count ?? 0),
      by_status,
    };
  });
}

export function getTaskIdsForFile(
  db: Database.Database,
  filePath: string,
  status?: string,
): number[] {
  const rows = db
    .prepare(
      `SELECT mt.id, cs.file, mt.status FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id`,
    )
    .all() as { id: number; file: string; status: string }[];

  let filtered = rows.filter((r) => filePathMatches(r.file, filePath));

  if (status) {
    filtered = filtered.filter((r) => r.status === status);
  }

  return filtered.map((r) => r.id);
}
