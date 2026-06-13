import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import { buildSeedForQuery, runDiscover } from "../../agents/discover.js";
import { getProviderFromConfig } from "../../agents/provider.js";
import * as investigationRepo from "../../db/repositories/investigation.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import { indexFiles } from "../../index/indexer.js";
import { JobQueue } from "../jobQueue.js";

export function createDiscoverRoutes(
  db: Database.Database,
  config: HarnessConfig,
  queue: JobQueue,
): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const investigations = investigationRepo.listInvestigations(db);
    return c.json(
      investigations.map((inv) => ({
        id: inv.id,
        query: inv.query,
        status: inv.status,
        job_id: inv.job_id,
        created_at: inv.created_at,
        finished_at: inv.finished_at,
        candidate_count: investigationRepo.parseInvestigationResult(inv)?.candidates.length ?? 0,
      })),
    );
  });

  app.get("/:id", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const inv = investigationRepo.getInvestigationById(db, id);
    if (!inv) return c.json({ error: "Not found" }, 404);

    const result = investigationRepo.parseInvestigationResult(inv);
    const seed = investigationRepo.parseInvestigationSeed(inv);

    let job = null;
    if (inv.job_id) {
      job = jobsRepo.enrichJob(db, jobsRepo.getJobById(db, inv.job_id) as never);
    }

    return c.json({ investigation: inv, result, seed, job });
  });

  app.post("/", async (c) => {
    const body = await c.req.json<{ query: string; sync?: boolean }>();
    if (!body.query?.trim()) {
      return c.json({ error: "query required" }, 400);
    }

    const query = body.query.trim();
    const seed = buildSeedForQuery(db, query);
    const investigationId = investigationRepo.createInvestigation(
      db,
      query,
      JSON.stringify(seed),
    );

    if (body.sync) {
      const provider = await getProviderFromConfig({
        ...config.provider,
        rustRoot: config.rustRoot,
      });
      const result = await runDiscover(db, config, provider, query, investigationId);
      const inv = investigationRepo.getInvestigationById(db, investigationId);
      return c.json({
        ok: result.success,
        investigationId,
        result: result.output,
        investigation: inv,
        error: result.error,
      });
    }

    const jobId = jobsRepo.createJob(db, "discover", { query, investigationId });
    investigationRepo.setInvestigationJobId(db, investigationId, jobId);
    queue.enqueue(jobId);

    return c.json({
      ok: true,
      investigationId,
      jobId,
      seedHitCount: seed.hits.length,
    });
  });

  app.post("/:id/actions", async (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const inv = investigationRepo.getInvestigationById(db, id);
    if (!inv) return c.json({ error: "Not found" }, 404);

    const body = await c.req.json<{
      action: "index" | "extract" | "verify";
      paths?: string[];
      taskIds?: number[];
    }>();

    if (!body.action) return c.json({ error: "action required" }, 400);

    if (body.action === "index") {
      const paths = body.paths ?? [];
      if (!paths.length) return c.json({ error: "paths required for index" }, 400);

      const results = [];
      for (const path of paths) {
        if (!path.endsWith(".cpp")) continue;
        const result = await indexFiles(db, config, [path]);
        results.push({ path, ...result });
      }
      return c.json({ ok: true, results });
    }

    const taskIds = body.taskIds ?? [];
    if (!taskIds.length) {
      return c.json({ error: "taskIds required for extract/verify" }, 400);
    }

    const stage = body.action === "extract" ? "extract" : "verify";
    const jobIds = jobsRepo.createBatchedJobs(
      db,
      stage,
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

  app.post("/index-dir", async (c) => {
    const body = await c.req.json<{ dir?: string }>();
    const dir = body.dir ?? "src/game";

    const { scanReferenceCppInDir } = await import("../../files/scanner.js");
    const files = scanReferenceCppInDir(config.referenceRoot, dir);

    if (!files.length) {
      return c.json({ error: `No .cpp files found under ${dir}` }, 400);
    }

    let totalSymbols = 0;
    const indexed: string[] = [];
    for (const file of files) {
      const result = await indexFiles(db, config, [file.path]);
      totalSymbols += result.symbolsIndexed;
      indexed.push(file.path);
    }

    return c.json({
      ok: true,
      dir,
      filesIndexed: indexed.length,
      symbolsIndexed: totalSymbols,
      files: indexed,
    });
  });

  return app;
}
