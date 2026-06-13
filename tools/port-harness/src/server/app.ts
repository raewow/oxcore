import { Hono } from "hono";
import { cors } from "hono/cors";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import { createTasksRoutes } from "./routes/tasks.js";
import { createFlowsRoutes } from "./routes/flows.js";
import { createSymbolsRoutes } from "./routes/symbols.js";
import { createJobsRoutes, getJobQueues } from "./routes/jobs.js";
import { createStatsRoutes } from "./routes/stats.js";
import { createFilesRoutes } from "./routes/files.js";
import { createDiscoverRoutes } from "./routes/discover.js";

export function createApp(db: Database.Database, config: HarnessConfig): Hono {
  const app = new Hono();

  app.use("*", cors());

  const queues = getJobQueues(db, config);

  app.route("/api/tasks", createTasksRoutes(db));
  app.route("/api/flows", createFlowsRoutes(db, config, queues));
  app.route("/api/symbols", createSymbolsRoutes(db, config));
  app.route("/api/jobs", createJobsRoutes(db, config, queues));
  app.route("/api/stats", createStatsRoutes(db));
  app.route("/api/files", createFilesRoutes(db, config, queues));
  app.route("/api/discover", createDiscoverRoutes(db, config, queues));

  app.get("/api/health", (c) => c.json({ ok: true }));

  return app;
}
