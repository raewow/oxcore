import type Database from "better-sqlite3";
import {
  getLatestAuditsForTasks,
  recommendNextAction,
  type FlowAuditSummary,
  type TaskAuditSummary,
} from "./flowAudits.js";
import { getLatestAgentRunsForTasks } from "./flowArtifacts.js";

export type FlowProgressStage = "empty" | "audit" | "plan" | "port" | "review" | "done";

export interface FlowProgressSummary extends FlowAuditSummary {
  planned: number;
  ported: number;
  needs_audit: number;
  needs_plan: number;
  needs_port: number;
  needs_review: number;
  done: number;
  stage: FlowProgressStage;
  percent: number;
}

export interface FlowListEntry {
  id: number;
  name: string;
  description: string | null;
  source_file: string | null;
  risk_level: string;
  symbol_count: number;
  progress: FlowProgressSummary;
}

interface FlowTaskRow {
  id: number;
  flow_id: number;
  status: string;
}

function taskPipelineScore(
  audit: TaskAuditSummary | undefined,
  status: string,
  hasPlan: boolean,
  hasPort: boolean,
): number {
  if (status === "reviewed" || status === "done") return 4;
  if (hasPort || status === "rust_ported") return 3;
  if (hasPlan || status === "rust_planned") return 2;
  if (audit) return 1;
  return 0;
}

export function summarizeFlowProgress(
  taskIds: number[],
  audits: Map<number, TaskAuditSummary>,
  taskStatuses: Map<number, string>,
  plansByTask: Map<number, unknown>,
  portsByTask: Map<number, unknown>,
): FlowProgressSummary {
  const auditSummary = summarizeFromAudits(taskIds, audits, taskStatuses);

  let planned = 0;
  let ported = 0;
  let needsAudit = 0;
  let needsPlan = 0;
  let needsPort = 0;
  let needsReview = 0;
  let done = 0;
  let scoreSum = 0;

  for (const taskId of taskIds) {
    const status = taskStatuses.get(taskId) ?? "";
    const audit = audits.get(taskId);
    const hasPlan = plansByTask.has(taskId) || status === "rust_planned" || status === "rust_ported";
    const hasPort = portsByTask.has(taskId) || status === "rust_ported";

    if (hasPlan) planned++;
    if (hasPort) ported++;

    const next = recommendNextAction(audit, status);
    switch (next) {
      case "audit":
        needsAudit++;
        break;
      case "plan":
        needsPlan++;
        break;
      case "port":
        needsPort++;
        break;
      case "review":
        needsReview++;
        break;
      case "done":
        done++;
        break;
    }

    scoreSum += taskPipelineScore(audit, status, hasPlan, hasPort);
  }

  const total = taskIds.length;
  const stage = deriveFlowStage(total, needsAudit, needsPlan, needsPort, needsReview, done);
  const percent = total ? Math.round((scoreSum / (total * 4)) * 100) : 0;

  return {
    ...auditSummary,
    planned,
    ported,
    needs_audit: needsAudit,
    needs_plan: needsPlan,
    needs_port: needsPort,
    needs_review: needsReview,
    done,
    stage,
    percent,
  };
}

function summarizeFromAudits(
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

function deriveFlowStage(
  total: number,
  needsAudit: number,
  needsPlan: number,
  needsPort: number,
  needsReview: number,
  done: number,
): FlowProgressStage {
  if (total === 0) return "empty";
  if (needsAudit > 0) return "audit";
  if (needsPlan > 0) return "plan";
  if (needsPort > 0) return "port";
  if (needsReview > 0) return "review";
  if (done === total) return "done";
  return "review";
}

export function listFlowsWithProgress(db: Database.Database): FlowListEntry[] {
  const flows = db
    .prepare(
      `SELECT bf.id, bf.name, bf.description, bf.source_file, bf.risk_level,
              COUNT(mt.id) AS symbol_count
       FROM business_flow bf
       LEFT JOIN migration_task mt ON mt.flow_id = bf.id
       GROUP BY bf.id
       ORDER BY bf.name`,
    )
    .all() as {
    id: number;
    name: string;
    description: string | null;
    source_file: string | null;
    risk_level: string;
    symbol_count: number;
  }[];

  const tasks = db
    .prepare("SELECT id, flow_id, status FROM migration_task WHERE flow_id IS NOT NULL")
    .all() as FlowTaskRow[];

  const tasksByFlow = new Map<number, FlowTaskRow[]>();
  const allTaskIds: number[] = [];
  for (const task of tasks) {
    allTaskIds.push(task.id);
    const list = tasksByFlow.get(task.flow_id) ?? [];
    list.push(task);
    tasksByFlow.set(task.flow_id, list);
  }

  const auditsByTask = getLatestAuditsForTasks(db, allTaskIds);
  const plansByTask = getLatestAgentRunsForTasks(db, allTaskIds, "plan-rust");
  const portsByTask = getLatestAgentRunsForTasks(db, allTaskIds, "port");

  return flows.map((flow) => {
    const flowTasks = tasksByFlow.get(flow.id) ?? [];
    const taskIds = flowTasks.map((t) => t.id);
    const statusByTask = new Map(flowTasks.map((t) => [t.id, t.status]));

    return {
      id: flow.id,
      name: flow.name,
      description: flow.description,
      source_file: flow.source_file,
      risk_level: flow.risk_level,
      symbol_count: flow.symbol_count,
      progress: summarizeFlowProgress(
        taskIds,
        auditsByTask,
        statusByTask,
        plansByTask,
        portsByTask,
      ),
    };
  });
}
