import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import { JobQueue } from "../jobQueue.js";

let queue: JobQueue | null = null;

export function getJobQueue(db: Database.Database, config: HarnessConfig): JobQueue {
  if (!queue) {
    queue = new JobQueue(db, config);
    queue.start();
  }
  return queue;
}

export function createJobsRoutes(
  db: Database.Database,
  config: HarnessConfig,
  jobQueue: JobQueue,
): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const activeJobIds = jobQueue.getActiveJobIds();
    jobsRepo.reconcileStaleRunningJobs(db, activeJobIds);
    const jobs = jobsRepo.enrichJobs(db, jobsRepo.listJobs(db) as never[], {
      targetLimit: 5,
    });
    const activeIds = [...activeJobIds];
    return c.json(
      jobs.map((j) => {
        const isActive = activeJobIds.has(j.id);
        const isStale = j.status === "running" && !isActive;
        return {
          ...j,
          active_job_ids: activeIds,
          active_job_id: activeIds[0] ?? null,
          is_stale: isStale,
          pause_requested: jobQueue.isPauseRequested(j.id),
          display_status:
            isStale
              ? "stale"
              : j.status === "running" && jobQueue.isPauseRequested(j.id)
                ? "pausing"
                : j.status,
        };
      }),
    );
  });

  app.get("/:id", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const activeJobIds = jobQueue.getActiveJobIds();
    jobsRepo.reconcileStaleRunningJobs(db, activeJobIds);
    const job = jobsRepo.getJobById(db, id);
    if (!job) return c.json({ error: "Not found" }, 404);
    const enriched = jobsRepo.enrichJob(db, job as never);
    const activeIds = [...activeJobIds];
    const isActive = activeJobIds.has(enriched.id);
    const isStale = enriched.status === "running" && !isActive;
    const pauseRequested = jobQueue.isPauseRequested(enriched.id);
    return c.json({
      ...enriched,
      active_job_ids: activeIds,
      active_job_id: activeIds[0] ?? null,
      is_stale: isStale,
      pause_requested: pauseRequested,
      display_status: isStale
        ? "stale"
        : enriched.status === "running" && pauseRequested
          ? "pausing"
          : enriched.status,
    });
  });

  app.post("/", async (c) => {
    const body = await c.req.json<{
      stage: string;
      taskIds?: number[];
      filter?: { status?: string; file?: string };
    }>();

    let taskIds = body.taskIds ?? [];

    if (!taskIds.length && body.filter) {
      const { tasks } = taskRepo.listTasksWithDetails(db, {
        status: body.filter.status as never,
        file: body.filter.file,
        limit: config.jobs.maxBatchSize,
      });
      taskIds = tasks.map((t) => t.id);
    }

    if (taskIds.length > config.jobs.maxBatchSize) {
      return c.json(
        { error: `Batch size ${taskIds.length} exceeds max ${config.jobs.maxBatchSize}` },
        400,
      );
    }

    if (!taskIds.length) {
      return c.json({ error: "No tasks to process" }, 400);
    }

    const jobId = jobsRepo.createJob(db, body.stage, taskIds);
    jobQueue.enqueue(jobId);

    return c.json({ jobId, total: taskIds.length });
  });

  app.post("/:id/retry", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const job = jobsRepo.getJobById(db, id);
    if (!job) return c.json({ error: "Not found" }, 404);

    if (!["done", "failed", "cancelled"].includes((job as { status: string }).status)) {
      return c.json({ error: "Only finished jobs can be retried" }, 400);
    }

    if (!jobsRepo.resetJobForRetry(db, id)) {
      return c.json({ error: "Could not retry job" }, 400);
    }

    jobQueue.enqueue(id);
    return c.json({ ok: true, jobId: id });
  });

  app.post("/:id/abandon", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!jobsRepo.getJobById(db, id)) {
      return c.json({ error: "Not found" }, 404);
    }

    if (!jobsRepo.abandonJob(db, id)) {
      return c.json({ error: "Only running jobs can be abandoned" }, 400);
    }

    return c.json({ ok: true });
  });

  app.post("/:id/pause", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!jobsRepo.getJobById(db, id)) {
      return c.json({ error: "Not found" }, 404);
    }

    const result = jobQueue.requestPause(id);
    if (!result.ok) {
      return c.json({ error: result.error ?? "Could not pause job" }, 400);
    }

    return c.json({ ok: true, immediate: result.immediate ?? false, pausing: result.pausing ?? false });
  });

  app.post("/:id/resume", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!jobsRepo.getJobById(db, id)) {
      return c.json({ error: "Not found" }, 404);
    }

    if (!jobQueue.resume(id)) {
      return c.json({ error: "Only paused jobs with remaining work can be resumed" }, 400);
    }

    return c.json({ ok: true, jobId: id });
  });

  app.post("/:id/continue", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const job = jobsRepo.getJobById(db, id);
    if (!job) return c.json({ error: "Not found" }, 404);

    const newJobId = jobsRepo.cloneJobRemaining(db, id);
    if (newJobId === null) {
      return c.json(
        {
          error:
            (job as { stage: string }).stage === "extract"
              ? "Nothing to continue — remaining symbols are already extracted"
              : "Nothing to continue — job finished or not resumable",
        },
        400,
      );
    }

    jobQueue.enqueue(newJobId);
    return c.json({ ok: true, jobId: newJobId });
  });

  app.post("/:id/cancel", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const job = jobsRepo.getJobById(db, id) as { status: string } | undefined;
    if (!job) return c.json({ error: "Not found" }, 404);

    if (job.status === "running") {
      return c.json({ error: "Running jobs cannot be cancelled yet" }, 400);
    }

    if (!jobsRepo.cancelJob(db, id)) {
      return c.json({ error: "Only queued jobs can be cancelled" }, 400);
    }

    return c.json({ ok: true });
  });

  app.delete("/:id", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!jobsRepo.getJobById(db, id)) {
      return c.json({ error: "Not found" }, 404);
    }

    if (!jobsRepo.deleteJob(db, id)) {
      return c.json({ error: "Only finished jobs can be removed" }, 400);
    }

    return c.json({ ok: true });
  });

  return app;
}

export { queue as jobQueueInstance };
