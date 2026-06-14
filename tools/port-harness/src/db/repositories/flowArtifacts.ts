import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type Database from "better-sqlite3";
import { getPackageRoot } from "../../config.js";

export function getLatestAgentRunsForTasks(
  db: Database.Database,
  taskIds: number[],
  stage: string,
): Map<number, { created_at: string; output_json: string }> {
  const result = new Map<number, { created_at: string; output_json: string }>();
  if (!taskIds.length) return result;

  const placeholders = taskIds.map(() => "?").join(",");
  const rows = db
    .prepare(
      `SELECT ar.task_id, ar.created_at, ar.output_json
       FROM agent_run ar
       INNER JOIN (
         SELECT task_id, MAX(created_at) AS max_created
         FROM agent_run
         WHERE stage = ? AND task_id IN (${placeholders})
         GROUP BY task_id
       ) latest ON ar.task_id = latest.task_id AND ar.created_at = latest.max_created
       WHERE ar.stage = ?`,
    )
    .all(stage, ...taskIds, stage) as {
    task_id: number;
    created_at: string;
    output_json: string;
  }[];

  for (const row of rows) {
    result.set(row.task_id, { created_at: row.created_at, output_json: row.output_json });
  }
  return result;
}

export interface TaskPlanSummary {
  target_rust_file: string;
  rust_symbol_name: string;
  structs: string[];
  enums: string[];
  notes: string;
  planned_at: string | null;
}

export interface TaskPortDraft {
  rust_code: string;
  todos: string[];
  ported_at: string | null;
  file_path: string | null;
}

export function parsePlanRun(
  run: { created_at: string; output_json: string } | undefined,
  taskFallback?: {
    target_rust_file: string | null;
    rust_symbol_name: string | null;
    notes: string | null;
    status?: string;
  },
): TaskPlanSummary | null {
  if (run) {
    try {
      const parsed = JSON.parse(run.output_json) as {
        target_rust_file?: string;
        rust_symbol_name?: string;
        structs?: string[];
        enums?: string[];
        notes?: string;
      };
      return {
        target_rust_file: parsed.target_rust_file ?? "",
        rust_symbol_name: parsed.rust_symbol_name ?? "",
        structs: parsed.structs ?? [],
        enums: parsed.enums ?? [],
        notes: parsed.notes ?? "",
        planned_at: run.created_at,
      };
    } catch {
      // fall through
    }
  }

  const planned =
    taskFallback?.status === "rust_planned" || taskFallback?.status === "rust_ported";
  if (
    planned &&
    (taskFallback?.target_rust_file || taskFallback?.rust_symbol_name)
  ) {
    return {
      target_rust_file: taskFallback.target_rust_file ?? "",
      rust_symbol_name: taskFallback.rust_symbol_name ?? "",
      structs: [],
      enums: [],
      notes: taskFallback.notes ?? "",
      planned_at: null,
    };
  }

  return null;
}

export function parsePortRun(
  run: { created_at: string; output_json: string } | undefined,
  symbolName: string,
): TaskPortDraft | null {
  if (run) {
    try {
      const parsed = JSON.parse(run.output_json) as {
        rust_code?: string;
        todos?: string[];
      };
      if (parsed.rust_code) {
        return {
          rust_code: parsed.rust_code,
          todos: parsed.todos ?? [],
          ported_at: run.created_at,
          file_path: portDraftRelPath(symbolName),
        };
      }
    } catch {
      // fall through
    }
  }

  const fromFile = readPortDraftFile(symbolName);
  if (fromFile) {
    return {
      rust_code: fromFile,
      todos: [],
      ported_at: null,
      file_path: portDraftRelPath(symbolName),
    };
  }

  return null;
}

export function portDraftRelPath(symbolName: string): string {
  return `tools/port-harness/docs/ports/${symbolName.replace(/::/g, "_")}.rs.txt`;
}

export function planDocRelPath(symbolName: string): string {
  return `tools/port-harness/docs/plans/${symbolName.replace(/::/g, "_")}.md`;
}

export function readPortDraftFile(symbolName: string): string | null {
  const path = join(getPackageRoot(), "docs/ports", `${symbolName.replace(/::/g, "_")}.rs.txt`);
  if (!existsSync(path)) return null;
  try {
    return readFileSync(path, "utf-8");
  } catch {
    return null;
  }
}

export function readPlanDocFile(symbolName: string): string | null {
  const path = join(getPackageRoot(), "docs/plans", `${symbolName.replace(/::/g, "_")}.md`);
  if (!existsSync(path)) return null;
  try {
    return readFileSync(path, "utf-8");
  } catch {
    return null;
  }
}

export function auditDocRelPath(symbolName: string): string {
  return `tools/port-harness/docs/audits/${symbolName.replace(/::/g, "_")}.md`;
}

export function readAuditDocFile(symbolName: string): string | null {
  const path = join(getPackageRoot(), "docs/audits", `${symbolName.replace(/::/g, "_")}.md`);
  if (!existsSync(path)) return null;
  try {
    return readFileSync(path, "utf-8");
  } catch {
    return null;
  }
}
