import type Database from "better-sqlite3";

export interface TaskAuditSummary {
  task_id: number;
  audited_at: string;
  implementation_status: "complete" | "partial" | "missing" | "incorrect";
  passed: boolean;
  coverage: {
    claims_covered: number;
    claims_total: number;
    branches_covered?: number;
    branches_total?: number;
  };
  summary: string;
  issues: { severity: string; message: string; claim_ref?: string }[];
  missing_behaviours: string[];
  planning_notes: string[];
  rust_locations: { file: string; symbol: string; start_line?: number; end_line?: number }[];
}

export interface FlowAuditSummary {
  total: number;
  audited: number;
  complete: number;
  partial: number;
  missing: number;
  incorrect: number;
  reviewed: number;
  passed: number;
}

export function getLatestAuditsForTasks(
  db: Database.Database,
  taskIds: number[],
): Map<number, TaskAuditSummary> {
  const result = new Map<number, TaskAuditSummary>();
  if (!taskIds.length) return result;

  const placeholders = taskIds.map(() => "?").join(",");
  const rows = db
    .prepare(
      `SELECT ar.task_id, ar.created_at, ar.output_json
       FROM agent_run ar
       INNER JOIN (
         SELECT task_id, MAX(created_at) AS max_created
         FROM agent_run
         WHERE stage = 'audit-rust' AND task_id IN (${placeholders})
         GROUP BY task_id
       ) latest ON ar.task_id = latest.task_id AND ar.created_at = latest.max_created
       WHERE ar.stage = 'audit-rust'`,
    )
    .all(...taskIds) as { task_id: number; created_at: string; output_json: string }[];

  for (const row of rows) {
    try {
      const parsed = JSON.parse(row.output_json) as Omit<TaskAuditSummary, "task_id" | "audited_at">;
      result.set(row.task_id, {
        task_id: row.task_id,
        audited_at: row.created_at,
        implementation_status: parsed.implementation_status,
        passed: parsed.passed,
        coverage: parsed.coverage,
        summary: parsed.summary,
        issues: parsed.issues ?? [],
        missing_behaviours: parsed.missing_behaviours ?? [],
        planning_notes: parsed.planning_notes ?? [],
        rust_locations: parsed.rust_locations ?? [],
      });
    } catch {
      // skip malformed rows
    }
  }

  return result;
}

export function summarizeFlowAudits(
  taskIds: number[],
  audits: Map<number, TaskAuditSummary>,
  taskStatuses: Map<number, string>,
): FlowAuditSummary {
  let complete = 0;
  let partial = 0;
  let missing = 0;
  let incorrect = 0;
  let passed = 0;
  let reviewed = 0;

  for (const taskId of taskIds) {
    const audit = audits.get(taskId);
    if (taskStatuses.get(taskId) === "reviewed") reviewed++;
    if (!audit) continue;

    if (audit.passed) passed++;
    switch (audit.implementation_status) {
      case "complete":
        complete++;
        break;
      case "partial":
        partial++;
        break;
      case "missing":
        missing++;
        break;
      case "incorrect":
        incorrect++;
        break;
    }
  }

  return {
    total: taskIds.length,
    audited: audits.size,
    complete,
    partial,
    missing,
    incorrect,
    reviewed,
    passed,
  };
}

export type FlowNextAction =
  | "audit"
  | "plan"
  | "port"
  | "review"
  | "done";

export function recommendNextAction(
  audit: TaskAuditSummary | undefined,
  taskStatus: string,
): FlowNextAction {
  if (taskStatus === "reviewed" || taskStatus === "done") return "done";
  if (!audit) return "audit";
  if (audit.passed && audit.implementation_status === "complete") return "review";
  if (taskStatus === "rust_planned") return "port";
  return "plan";
}
