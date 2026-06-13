import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import * as flowRepo from "../../db/repositories/businessFlow.js";
import * as flowProgressRepo from "../../db/repositories/flowProgress.js";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import {
  getLatestAuditsForTasks,
  summarizeFlowAudits,
  recommendNextAction,
} from "../../db/repositories/flowAudits.js";
import {
  getLatestAgentRunsForTasks,
  parsePlanRun,
  parsePortRun,
} from "../../db/repositories/flowArtifacts.js";
import type { JobQueues } from "../jobQueue.js";

function getFlowJobsForTasks(
  db: Database.Database,
  taskIds: number[],
  stages: string[],
  limit = 20,
) {
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

const FLOW_PIPELINE_STAGES = ["audit-rust", "plan-rust", "port"];

export function createFlowsRoutes(
  db: Database.Database,
  config: HarnessConfig,
  queues: JobQueues,
): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const flows = flowProgressRepo.listFlowsWithProgress(db);
    return c.json(flows);
  });

  app.get("/:id", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const flow = flowRepo.getFlowById(db, id);
    if (!flow) return c.json({ error: "Not found" }, 404);

    const branches = flowRepo.getBranchesForFlow(db, id);
    const mutations = flowRepo.getMutationsForFlow(db, id);
    const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 1000 });
    const flowTasks = tasks.filter((t) => t.flow_id === id);
    const taskIds = flowTasks.map((t) => t.id);
    const auditsByTask = getLatestAuditsForTasks(db, taskIds);
    const plansByTask = getLatestAgentRunsForTasks(db, taskIds, "plan-rust");
    const portsByTask = getLatestAgentRunsForTasks(db, taskIds, "port");
    const statusByTask = new Map(flowTasks.map((t) => [t.id, t.status]));

    const tasksWithAudits = flowTasks.map((t) => ({
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
    }));

    return c.json({
      flow,
      branches,
      mutations,
      tasks: tasksWithAudits,
      audit_summary: summarizeFlowAudits(taskIds, auditsByTask, statusByTask),
      pipeline_jobs: getFlowJobsForTasks(db, taskIds, FLOW_PIPELINE_STAGES),
    });
  });

  app.post("/:id/port", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const flow = flowRepo.getFlowById(db, id);
    if (!flow) return c.json({ error: "Not found" }, 404);

    const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 10000 });
    const flowTasks = tasks.filter((t) => t.flow_id === id);
    const audits = getLatestAuditsForTasks(
      db,
      flowTasks.map((t) => t.id),
    );

    const taskIds = flowTasks
      .filter((t) => recommendNextAction(audits.get(t.id), t.status) === "port")
      .map((t) => t.id);

    if (!taskIds.length) {
      return c.json({ error: "No symbols ready to port (plan first)" }, 400);
    }

    const jobIds = jobsRepo.createBatchedJobs(db, "port", taskIds, config.jobs.maxBatchSize);
    for (const jobId of jobIds) queues.enqueue(jobId);

    return c.json({ ok: true, jobIds, totalTasks: taskIds.length, batches: jobIds.length });
  });

  app.post("/:id/plan", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const flow = flowRepo.getFlowById(db, id);
    if (!flow) return c.json({ error: "Not found" }, 404);

    const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 10000 });
    const flowTasks = tasks.filter((t) => t.flow_id === id);
    const audits = getLatestAuditsForTasks(
      db,
      flowTasks.map((t) => t.id),
    );

    const taskIds = flowTasks
      .filter((t) => {
        const action = recommendNextAction(audits.get(t.id), t.status);
        return action === "plan";
      })
      .map((t) => t.id);

    if (!taskIds.length) {
      return c.json({ error: "No symbols need planning (audit first or already planned)" }, 400);
    }

    const jobIds = jobsRepo.createBatchedJobs(db, "plan-rust", taskIds, config.jobs.maxBatchSize);
    for (const jobId of jobIds) queues.enqueue(jobId);

    return c.json({ ok: true, jobIds, totalTasks: taskIds.length, batches: jobIds.length });
  });

  app.post("/:id/audit", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const flow = flowRepo.getFlowById(db, id);
    if (!flow) return c.json({ error: "Not found" }, 404);

    const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 10000 });
    const taskIds = tasks.filter((t) => t.flow_id === id).map((t) => t.id);

    if (!taskIds.length) {
      return c.json({ error: "No tasks linked to this flow" }, 400);
    }

    const jobIds = jobsRepo.createBatchedJobs(
      db,
      "audit-rust",
      taskIds,
      config.jobs.maxBatchSize,
    );
    for (const jobId of jobIds) queues.enqueue(jobId);

    return c.json({
      ok: true,
      jobIds,
      totalTasks: taskIds.length,
      batches: jobIds.length,
    });
  });

  app.post("/:id/done", async (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const flow = flowRepo.getFlowById(db, id);
    if (!flow) return c.json({ error: "Not found" }, 404);

    const body = (await c.req.json<{ taskIds?: number[] }>().catch(() => ({}))) as {
      taskIds?: number[];
    };

    const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 10000 });
    const flowTasks = tasks.filter((t) => t.flow_id === id);

    let taskIds = body.taskIds?.filter((taskId) => flowTasks.some((t) => t.id === taskId));
    if (!taskIds?.length) {
      taskIds = flowTasks
        .filter((t) => t.status !== "done" && t.status !== "reviewed")
        .map((t) => t.id);
    }

    if (!taskIds.length) {
      return c.json({ error: "No symbols to mark done" }, 400);
    }

    taskRepo.bulkUpdateTasks(db, taskIds, { status: "done" });

    return c.json({ ok: true, updated: taskIds.length });
  });

  return app;
}
