import { Hono } from "hono";
import { basename } from "node:path";
import { readFileSync, existsSync } from "node:fs";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import { scanReferenceFiles } from "../../files/scanner.js";
import { filePathMatches, resolveSourceFilePath } from "../../files/paths.js";
import * as fileStatsRepo from "../../db/repositories/fileStats.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import * as codeSymbolRepo from "../../db/repositories/codeSymbol.js";
import * as featureRepo from "../../db/repositories/features.js";
import type { JobQueues } from "../jobQueue.js";

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

function normalizeFile(file: string): string {
  return file.replace(/\\/g, "/");
}

function fileMatchesExactOrBase(stored: string, requested: string): boolean {
  return filePathMatches(stored, requested);
}

function getFileDependencies(db: Database.Database, path: string) {
  const outbound = new Map<string, { file: string; count: number; examples: string[] }>();
  const inbound = new Map<string, { file: string; count: number; examples: string[] }>();
  const symbols = codeSymbolRepo.listSymbolsByFile(db, path);
  const symbolIds = new Set(symbols.map((s) => s.id));

  const calls = db
    .prepare(
      `SELECT caller.id AS caller_id, caller.name AS caller_name, caller.file AS caller_file,
              sc.callee_name, sc.callee_file, sc.line,
              callee.file AS resolved_callee_file
       FROM symbol_call sc
       JOIN code_symbol caller ON caller.id = sc.caller_id
       LEFT JOIN code_symbol callee ON callee.name = sc.callee_name`,
    )
    .all() as {
    caller_id: number;
    caller_name: string;
    caller_file: string;
    callee_name: string;
    callee_file: string | null;
    line: number;
    resolved_callee_file: string | null;
  }[];

  const add = (
    map: Map<string, { file: string; count: number; examples: string[] }>,
    file: string,
    example: string,
  ) => {
    const normalized = normalizeFile(file);
    if (!normalized || fileMatchesExactOrBase(normalized, path)) return;
    const row = map.get(normalized) ?? { file: normalized, count: 0, examples: [] };
    row.count++;
    if (row.examples.length < 3) row.examples.push(example);
    map.set(normalized, row);
  };

  for (const call of calls) {
    const calleeFile = call.callee_file ?? call.resolved_callee_file;
    if (symbolIds.has(call.caller_id) && calleeFile) {
      add(outbound, calleeFile, `${call.caller_name} -> ${call.callee_name}`);
    }
    if (calleeFile && fileMatchesExactOrBase(calleeFile, path) && !symbolIds.has(call.caller_id)) {
      add(inbound, call.caller_file, `${call.caller_name} -> ${call.callee_name}`);
    }
  }

  const deps = db
    .prepare(
      `SELECT d.file, d.description, cs.file AS symbol_file
       FROM dependency d
       JOIN code_symbol cs ON cs.id = d.symbol_id
       WHERE d.file IS NOT NULL`,
    )
    .all() as { file: string; description: string; symbol_file: string }[];

  for (const dep of deps) {
    if (fileMatchesExactOrBase(dep.symbol_file, path)) {
      add(outbound, dep.file, dep.description);
    }
    if (fileMatchesExactOrBase(dep.file, path) && !fileMatchesExactOrBase(dep.symbol_file, path)) {
      add(inbound, dep.symbol_file, dep.description);
    }
  }

  return {
    file: path,
    outbound: [...outbound.values()].sort((a, b) => b.count - a.count),
    inbound: [...inbound.values()].sort((a, b) => b.count - a.count),
  };
}

export function createFilesRoutes(
  db: Database.Database,
  config: HarnessConfig,
  queues: JobQueues,
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

  app.get("/dependencies", (c) => {
    const path = c.req.query("path");
    if (!path) return c.json({ error: "path required" }, 400);
    return c.json(getFileDependencies(db, path));
  });

  app.get("/detail", (c) => {
    const path = c.req.query("path");
    if (!path) return c.json({ error: "path required" }, 400);

    const entry = buildFileList(db, config).find((f) => filePathMatches(f.path, path));
    if (!entry) return c.json({ error: "File not found" }, 404);

    const symbols = codeSymbolRepo.listSymbolsByFile(db, entry.path);
    const { tasks } = taskRepo.listTasksWithDetails(db, {
      file: basename(entry.path),
      limit: 10000,
    });
    const flowsById = new Map<number, { id: number; name: string; risk_level: string | null }>();
    for (const task of tasks) {
      if (task.flow_id && task.flow_name) {
        flowsById.set(task.flow_id, {
          id: task.flow_id,
          name: task.flow_name,
          risk_level: task.risk_level,
        });
      }
    }

    const jobs = jobsRepo
      .enrichJobs(
        db,
        (jobsRepo.listJobs(db, 50) as never[]).filter((job: {
          target_ids: string;
          stage: string;
        }) => {
          try {
            const parsed = JSON.parse(job.target_ids);
            if (Array.isArray(parsed)) {
              const ids = new Set(tasks.map((t) => t.id));
              return parsed.some((id) => ids.has(id));
            }
            if (parsed && typeof parsed === "object") {
              const file = "file" in parsed ? parsed.file : "path" in parsed ? parsed.path : null;
              return typeof file === "string" && filePathMatches(file, entry.path);
            }
          } catch {
            return false;
          }
          return false;
        }),
        { targetLimit: 5 },
      )
      .slice(0, 10);

    const assignments = featureRepo
      .listFeatures(db)
      .filter((feature) =>
        featureRepo
          .listAssignments(db, feature.id)
          .some(
            (a) =>
              (a.target_type === "file" && filePathMatches(a.target_id, entry.path)) ||
              (a.target_type === "flow" && flowsById.has(Number(a.target_id))) ||
              (a.target_type === "task" && tasks.some((t) => t.id === Number(a.target_id))),
          ),
      );

    let source_preview: { start_line: number; text: string } | null = null;
    try {
      const fullPath = resolveSourceFilePath(config.referenceRoot, entry.path);
      if (existsSync(fullPath)) {
        source_preview = {
          start_line: 1,
          text: readFileSync(fullPath, "utf-8").split(/\r?\n/).slice(0, 220).join("\n"),
        };
      }
    } catch {
      source_preview = null;
    }

    return c.json({
      file: entry,
      symbols,
      tasks,
      flows: [...flowsById.values()],
      jobs,
      features: assignments,
      dependencies: getFileDependencies(db, entry.path),
      source_preview,
    });
  });

  app.post("/index", async (c) => {
    const body = await c.req.json<{ path: string }>();
    if (!body.path) return c.json({ error: "path required" }, 400);
    if (!body.path.endsWith(".cpp")) {
      return c.json({ error: "Only .cpp files can be indexed" }, 400);
    }

    const jobId = jobsRepo.createJob(db, "index", { path: body.path });
    queues.enqueue(jobId);

    return c.json({ ok: true, jobId, path: body.path });
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
    for (const id of jobIds) queues.enqueue(id);

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

    const jobId = jobsRepo.createJob(db, "assemble-flows", { file: body.path });
    queues.enqueue(jobId);

    return c.json({ ok: true, jobId });
  });

  app.post("/pipeline", async (c) => {
    const body = await c.req.json<{ path: string }>();
    if (!body.path?.endsWith(".cpp")) {
      return c.json({ error: "path must be a .cpp file" }, 400);
    }

    const jobId = jobsRepo.createJob(db, "file-pipeline", { path: body.path });
    queues.enqueue(jobId);

    return c.json({ ok: true, jobId, path: body.path });
  });

  return app;
}
