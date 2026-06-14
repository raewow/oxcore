import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import * as flowRepo from "../db/repositories/businessFlow.js";
import * as flowProgressRepo from "../db/repositories/flowProgress.js";
import { getLatestAuditsForTasks, recommendNextAction, summarizeFlowAudits } from "../db/repositories/flowAudits.js";
import { getLatestAgentRunsForTasks, parsePlanRun, parsePortRun, planDocRelPath, readPlanDocFile, portDraftRelPath, auditDocRelPath, readAuditDocFile } from "../db/repositories/flowArtifacts.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as claimRepo from "../db/repositories/behaviourClaim.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import { resolveSourceFilePath } from "../files/paths.js";
import { getSourceSnippet } from "../index/parser.js";

type FlowRecord = {
  id: number;
  name: string;
  description: string | null;
  source_file: string | null;
  risk_level: string;
  entry_symbol_ids: string | null;
  expected_behaviour: string | null;
};

export function resolveFlow(db: Database.Database, ref: string): FlowRecord | undefined {
  const byName = flowRepo.getFlowByName(db, ref) as FlowRecord | undefined;
  if (byName) return byName;
  if (/^\d+$/.test(ref)) return flowRepo.getFlowById(db, Number(ref)) as FlowRecord | undefined;
  return (flowRepo.listFlows(db) as FlowRecord[]).find((flow) => flow.name.toLowerCase() === ref.toLowerCase());
}

function getFlowJobsForTasks(db: Database.Database, taskIds: number[], stages: string[], limit = 20) {
  const idSet = new Set(taskIds);
  const stageSet = new Set(stages);
  const jobs = jobsRepo.listJobs(db, 50) as {
    id: number;
    stage: string;
    target_ids: string;
    status: string;
    progress: number;
    total: number;
    error: string | null;
    created_at: string;
  }[];

  return jobs
    .filter((job) => {
      if (!stageSet.has(job.stage)) return false;
      try {
        const ids = JSON.parse(job.target_ids) as number[];
        return Array.isArray(ids) && ids.some((id) => idSet.has(id));
      } catch {
        return false;
      }
    })
    .slice(0, limit)
    .map((job) => jobsRepo.enrichJob(db, job as never, { targetLimit: 3 }));
}

export function listFlowsForMcp(db: Database.Database) {
  return flowProgressRepo.listFlowsWithProgress(db);
}

export function getFlowDetailsForMcp(
  db: Database.Database,
  config: Pick<HarnessConfig, "referenceRoot"> | null | undefined,
  ref: string,
) {
  const flow = resolveFlow(db, ref);
  if (!flow) return null;

  const branches = flowRepo.getBranchesForFlow(db, flow.id);
  const mutations = flowRepo.getMutationsForFlow(db, flow.id);
  const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 1000 });
  const flowTasks = tasks.filter((t) => t.flow_id === flow.id);
  const taskIds = flowTasks.map((t) => t.id);
  const auditsByTask = getLatestAuditsForTasks(db, taskIds);
  const plansByTask = getLatestAgentRunsForTasks(db, taskIds, "plan-rust");
  const portsByTask = getLatestAgentRunsForTasks(db, taskIds, "port");
  const statusByTask = new Map(flowTasks.map((t) => [t.id, t.status]));
  const progress = flowProgressRepo.summarizeFlowProgress(taskIds, auditsByTask, statusByTask, plansByTask, portsByTask);
  const auditSummary = summarizeFlowAudits(taskIds, auditsByTask, statusByTask);

  const tasksWithContext = flowTasks.map((t) => {
    let sourceSnippet: { start_line: number; end_line: number; text: string } | null = null;
    if (config?.referenceRoot) {
      try {
        const sourcePath = resolveSourceFilePath(config.referenceRoot, t.symbol_file, { symbolName: t.symbol_name });
        sourceSnippet = {
          start_line: t.start_line,
          end_line: t.end_line,
          text: getSourceSnippet(sourcePath, t.start_line, t.end_line),
        };
      } catch {
        sourceSnippet = null;
      }
    }

    return {
      ...t,
      audit: auditsByTask.get(t.id) ?? null,
      plan: parsePlanRun(plansByTask.get(t.id), {
        target_rust_file: t.target_rust_file,
        rust_symbol_name: t.rust_symbol_name,
        notes: t.notes,
        status: t.status,
      }),
      port_draft: parsePortRun(portsByTask.get(t.id), t.symbol_name),
      next_action: recommendNextAction(auditsByTask.get(t.id), t.status),
      claims: claimRepo.getClaimsForSymbol(db, t.source_symbol_id),
      dependencies: claimRepo.getDependenciesForSymbol(db, t.source_symbol_id),
      source_snippet: sourceSnippet,
      generated_docs: {
        audit_path: auditDocRelPath(t.symbol_name),
        audit_markdown: readAuditDocFile(t.symbol_name),
        plan_path: planDocRelPath(t.symbol_name),
        plan_markdown: readPlanDocFile(t.symbol_name),
        port_path: portDraftRelPath(t.symbol_name),
        port_code: parsePortRun(portsByTask.get(t.id), t.symbol_name)?.rust_code ?? null,
      },
    };
  });

  return {
    flow,
    branches,
    mutations,
    progress,
    audit_summary: auditSummary,
    tasks: tasksWithContext,
    pipeline_jobs: getFlowJobsForTasks(db, taskIds, ["audit-rust", "plan-rust", "port"]),
  };
}
