import { Hono } from "hono";
import { basename } from "node:path";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import { scanReferenceFiles } from "../../files/scanner.js";
import { filePathMatches } from "../../files/paths.js";
import * as fileStatsRepo from "../../db/repositories/fileStats.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import { indexFiles, applyFlowMappings } from "../../index/indexer.js";
import { JobQueue } from "../jobQueue.js";

export interface FileListEntry {
  path: string;
  name: string;
  kind: "cpp" | "h";
  size_bytes: number;
  indexed: boolean;
  symbol_count: number;
  discovered: number;
  documented: number;
  blocked: number;
  flow_count: number;
}

function buildFileList(db: Database.Database, config: HarnessConfig, q?: string): FileListEntry[] {
  const scanned = scanReferenceFiles(config.referenceRoot);
  const allStats = fileStatsRepo.getAllFileStats(db);

  let entries: FileListEntry[] = scanned.map((f) => {
    const stats = allStats.find((s) => filePathMatches(s.file, f.path));
    return {
      path: f.path,
      name: f.name,
      kind: f.kind,
      size_bytes: f.size_bytes,
      indexed: (stats?.symbol_count ?? 0) > 0,
      symbol_count: stats?.symbol_count ?? 0,
      discovered: stats?.discovered ?? 0,
      documented: stats?.documented ?? 0,
      blocked: stats?.blocked ?? 0,
      flow_count: stats?.flow_count ?? 0,
    };
  });

  if (q) {
    const lower = q.toLowerCase();
    entries = entries.filter(
      (e) =>
        e.path.toLowerCase().includes(lower) ||
        e.name.toLowerCase().includes(lower),
    );
  }

  return entries;
}

export function createFilesRoutes(
  db: Database.Database,
  config: HarnessConfig,
  queue: JobQueue,
): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    const q = c.req.query("q");
    const kind = c.req.query("kind");
    let entries = buildFileList(db, config, q);

    if (kind === "cpp" || kind === "h") {
      entries = entries.filter((e) => e.kind === kind);
    }

    const indexed = entries.filter((e) => e.indexed).length;
    return c.json({
      files: entries,
      total: entries.length,
      indexed_count: indexed,
    });
  });

  app.post("/index", async (c) => {
    const body = await c.req.json<{ path: string }>();
    if (!body.path) return c.json({ error: "path required" }, 400);
    if (!body.path.endsWith(".cpp")) {
      return c.json({ error: "Only .cpp files can be indexed" }, 400);
    }

    const result = await indexFiles(db, config, [body.path]);
    const mappingsApplied =
      config.flowMappings && Object.keys(config.flowMappings).length > 0
        ? applyFlowMappings(db, config.flowMappings)
        : 0;

    return c.json({
      ok: true,
      path: body.path,
      ...result,
      mappingsApplied,
      file: buildFileList(db, config).find((f) => f.path === body.path),
    });
  });

  app.post("/document", async (c) => {
    const body = await c.req.json<{ path: string; status?: string }>();
    if (!body.path) return c.json({ error: "path required" }, 400);

    const name = basename(body.path);
    const taskIds = fileStatsRepo.getTaskIdsForFile(
      db,
      name,
      body.status ?? "discovered",
    );

    if (!taskIds.length) {
      return c.json({ error: "No tasks to document. Index the file first." }, 400);
    }

    const jobIds = jobsRepo.createBatchedJobs(
      db,
      "extract",
      taskIds,
      config.jobs.maxBatchSize,
    );
    for (const id of jobIds) queue.enqueue(id);

    return c.json({
      ok: true,
      jobIds,
      totalTasks: taskIds.length,
      batches: jobIds.length,
    });
  });

  app.post("/assemble-flows", async (c) => {
    const body = await c.req.json<{ path: string }>();
    if (!body.path) return c.json({ error: "path required" }, 400);

    const name = basename(body.path);
    const jobId = jobsRepo.createJob(db, "assemble-flows", { file: body.path });
    queue.enqueue(jobId);

    return c.json({ ok: true, jobId });
  });

  app.post("/pipeline", async (c) => {
    const body = await c.req.json<{ path: string }>();
    if (!body.path?.endsWith(".cpp")) {
      return c.json({ error: "path must be a .cpp file" }, 400);
    }

    const indexResult = await indexFiles(db, config, [body.path]);
    const mappingsApplied =
      config.flowMappings && Object.keys(config.flowMappings).length > 0
        ? applyFlowMappings(db, config.flowMappings)
        : 0;

    const name = basename(body.path);
    const taskIds = fileStatsRepo.getTaskIdsForFile(db, name, "discovered");

    const extractJobIds = taskIds.length
      ? jobsRepo.createBatchedJobs(db, "extract", taskIds, config.jobs.maxBatchSize)
      : [];

    const assembleJobId = jobsRepo.createJob(db, "assemble-flows", { file: body.path });

    for (const id of extractJobIds) queue.enqueue(id);
    queue.enqueue(assembleJobId);

    return c.json({
      ok: true,
      indexed: true,
      mappingsApplied,
      indexResult,
      extractJobIds,
      assembleJobId,
      totalTasks: taskIds.length,
    });
  });

  return app;
}
