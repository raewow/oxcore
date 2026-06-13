import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import * as flowRepo from "../../db/repositories/businessFlow.js";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import type { JobQueue } from "../jobQueue.js";

export function createFlowsRoutes(
  db: Database.Database,
  config: HarnessConfig,
  queue: JobQueue,
): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const flows = flowRepo.listFlowsWithStats(db);
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

    return c.json({ flow, branches, mutations, tasks: flowTasks });
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
    for (const jobId of jobIds) queue.enqueue(jobId);

    return c.json({
      ok: true,
      jobIds,
      totalTasks: taskIds.length,
      batches: jobIds.length,
    });
  });

  return app;
}
