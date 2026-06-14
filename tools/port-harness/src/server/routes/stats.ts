import { Hono } from "hono";
import type Database from "better-sqlite3";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import * as jobsRepo from "../../db/repositories/jobs.js";

export function createStatsRoutes(db: Database.Database): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const statusCounts = taskRepo.getStatusCounts(db);
    const fileProgress = taskRepo.getFileProgress(db);
    const recentRuns = jobsRepo.getRecentAgentRuns(db, 10);
    const recentJobs = jobsRepo.listJobs(db, 5);
    const workingFiles = jobsRepo.listWorkingFiles(db, 8);

    const blocked = statusCounts.find((s) => s.status === "blocked")?.count ?? 0;
    const discovered = statusCounts.find((s) => s.status === "discovered")?.count ?? 0;

    return c.json({
      status_counts: statusCounts,
      file_progress: fileProgress,
      recent_agent_runs: recentRuns,
      recent_jobs: recentJobs,
      working_files: workingFiles,
      warnings: {
        blocked,
        missing_docs: discovered,
      },
    });
  });

  return app;
}
