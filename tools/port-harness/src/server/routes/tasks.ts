import { Hono } from "hono";
import type Database from "better-sqlite3";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import * as claimRepo from "../../db/repositories/behaviourClaim.js";
import type { TaskStatus } from "../../models/index.js";

export function createTasksRoutes(db: Database.Database): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const file = c.req.query("file");
    const status = c.req.query("status") as TaskStatus | undefined;
    const flow = c.req.query("flow");
    const q = c.req.query("q");
    const missingDocs = c.req.query("missingDocs") === "true";
    const blocked = c.req.query("blocked") === "true";
    const limit = parseInt(c.req.query("limit") ?? "100", 10);
    const offset = parseInt(c.req.query("offset") ?? "0", 10);

    const result = taskRepo.listTasksWithDetails(db, {
      file,
      status,
      flow,
      q,
      missingDocs,
      blocked,
      limit,
      offset,
    });

    return c.json(result);
  });

  app.get("/:id", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const task = taskRepo.getTaskById(db, id);
    if (!task) return c.json({ error: "Not found" }, 404);

    const claims = claimRepo.getClaimsForSymbol(db, task.source_symbol_id);
    const deps = claimRepo.getDependenciesForSymbol(db, task.source_symbol_id);

    return c.json({ task, claims, dependencies: deps });
  });

  app.patch("/bulk", async (c) => {
    const body = await c.req.json<{
      ids: number[];
      status?: TaskStatus;
      notes?: string;
      target_rust_file?: string;
      flow_id?: number;
    }>();

    if (!body.ids?.length) {
      return c.json({ error: "ids required" }, 400);
    }

    taskRepo.bulkUpdateTasks(db, body.ids, {
      status: body.status,
      notes: body.notes,
      target_rust_file: body.target_rust_file,
      flow_id: body.flow_id,
    });

    return c.json({ updated: body.ids.length });
  });

  app.patch("/:id", async (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const body = await c.req.json<{ status?: TaskStatus; notes?: string }>();

    if (body.status) {
      taskRepo.updateTaskStatus(db, id, body.status);
    }
    if (body.notes !== undefined) {
      taskRepo.bulkUpdateTasks(db, [id], { notes: body.notes });
    }

    return c.json({ ok: true });
  });

  return app;
}
