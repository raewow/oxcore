import type Database from "better-sqlite3";
import { filePathMatches } from "../../files/paths.js";

export interface DbFileStats {
  file: string;
  symbol_count: number;
  discovered: number;
  documented: number;
  blocked: number;
  flow_count: number;
}

export function getAllFileStats(db: Database.Database): DbFileStats[] {
  return db
    .prepare(
      `SELECT cs.file,
              COUNT(*) as symbol_count,
              SUM(CASE WHEN mt.status = 'discovered' THEN 1 ELSE 0 END) as discovered,
              SUM(CASE WHEN mt.status NOT IN ('discovered') THEN 1 ELSE 0 END) as documented,
              SUM(CASE WHEN mt.status = 'blocked' THEN 1 ELSE 0 END) as blocked,
              COUNT(DISTINCT mt.flow_id) as flow_count
       FROM migration_task mt
       JOIN code_symbol cs ON cs.id = mt.source_symbol_id
       GROUP BY cs.file`,
    )
    .all() as DbFileStats[];
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
