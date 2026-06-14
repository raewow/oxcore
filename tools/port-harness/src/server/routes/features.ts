import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import * as featureRepo from "../../db/repositories/features.js";
import * as jobsRepo from "../../db/repositories/jobs.js";
import * as taskRepo from "../../db/repositories/migrationTask.js";
import * as fileStatsRepo from "../../db/repositories/fileStats.js";
import * as investigationRepo from "../../db/repositories/investigation.js";
import { scanReferenceFiles } from "../../files/scanner.js";
import { filePathMatches } from "../../files/paths.js";
import type { JobQueues } from "../jobQueue.js";

function dependencyRowsForFiles(db: Database.Database, files: string[]) {
  if (!files.length) return { nodes: [], edges: [] };
  const fileSet = new Set(files);
  const calls = db
    .prepare(
      `SELECT caller.file AS from_file, COALESCE(sc.callee_file, callee.file) AS to_file,
              COUNT(*) AS count
       FROM symbol_call sc
       JOIN code_symbol caller ON caller.id = sc.caller_id
       LEFT JOIN code_symbol callee ON callee.name = sc.callee_name
       GROUP BY caller.file, COALESCE(sc.callee_file, callee.file)`,
    )
    .all() as { from_file: string; to_file: string | null; count: number }[];

  const edges = calls
    .filter((row) => {
      if (!row.to_file || filePathMatches(row.from_file, row.to_file)) return false;
      return (
        [...fileSet].some((file) => filePathMatches(row.from_file, file)) ||
        [...fileSet].some((file) => filePathMatches(row.to_file!, file))
      );
    })
    .map((row) => ({ from: row.from_file, to: row.to_file!, count: row.count }));

  const nodes = [...new Set([...files, ...edges.flatMap((e) => [e.from, e.to])])].map((file) => ({
    file,
    in_feature: files.some((f) => filePathMatches(file, f)),
  }));

  return { nodes, edges };
}

function featureFileStatus(
  db: Database.Database,
  config: HarnessConfig,
  files: string[],
) {
  const stats = fileStatsRepo.getAllFileStats(db);
  const scanned = scanReferenceFiles(config.referenceRoot);

  return files
    .map((file) => {
      const scannedFile = scanned.find((f) => filePathMatches(f.path, file));
      const row = stats.find((s) => filePathMatches(s.file, file));
      const path = scannedFile?.path ?? file;
      const name = scannedFile?.name ?? path.split("/").pop() ?? path;
      return {
        path,
        name,
        kind: scannedFile?.kind ?? (path.endsWith(".h") ? "h" : "cpp"),
        size_bytes: scannedFile?.size_bytes ?? 0,
        indexed: (row?.symbol_count ?? 0) > 0,
        symbol_count: row?.symbol_count ?? 0,
        discovered: row?.discovered ?? 0,
        documented: row?.documented ?? 0,
        blocked: row?.blocked ?? 0,
        flow_count: row?.flow_count ?? 0,
      };
    })
    .sort((a, b) => a.path.localeCompare(b.path));
}

export function createFeaturesRoutes(db: Database.Database, config: HarnessConfig, queues: JobQueues): Hono {
  const app = new Hono();

  app.get("/", (c) => {
    return c.json(featureRepo.listFeatures(db));
  });

  app.post("/", async (c) => {
    const body = await c.req.json<{ name?: string; description?: string }>();
    if (!body.name?.trim()) return c.json({ error: "name required" }, 400);
    const id = featureRepo.createFeature(db, body.name.trim(), body.description ?? null);
    featureRepo.refreshSuggestions(db, id);
    return c.json({ id });
  });

  app.get("/:id", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const feature = featureRepo.getFeature(db, id);
    if (!feature) return c.json({ error: "Not found" }, 404);

    const assignments = featureRepo.listAssignments(db, id);
    const tasks = featureRepo.getFeatureTasks(db, id);
    const taskIds = new Set(tasks.map((t) => t.id));
    const files = [...new Set(tasks.map((t) => t.symbol_file))];
    const assignedFiles = assignments
      .filter((a) => a.target_type === "file")
      .map((a) => a.target_id);
    for (const file of assignedFiles) {
      if (!files.some((f) => filePathMatches(f, file))) files.push(file);
    }
    const flows = [
      ...new Map(
        tasks
          .filter((t) => t.flow_id && t.flow_name)
          .map((t) => [
            t.flow_id!,
            { id: t.flow_id!, name: t.flow_name!, risk_level: t.risk_level },
          ]),
      ).values(),
    ];
    const jobs = jobsRepo
      .enrichJobs(
        db,
        (jobsRepo.listJobs(db, 50) as never[]).filter((job: { target_ids: string }) => {
          try {
            const parsed = JSON.parse(job.target_ids);
            if (!Array.isArray(parsed)) return false;
            return parsed.some((taskId) => taskIds.has(taskId));
          } catch {
            return false;
          }
        }),
        { targetLimit: 5 },
      )
      .slice(0, 10);

    return c.json({
      feature,
      assignments,
      suggestions: featureRepo.listSuggestions(db, id),
      stats: featureRepo.getFeatureTaskStats(db, id),
      files,
      file_status: featureFileStatus(db, config, files),
      flows,
      tasks,
      jobs,
      dependency_graph: dependencyRowsForFiles(db, files),
    });
  });

  app.patch("/:id", async (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);
    const body = await c.req.json<{ name?: string; description?: string | null }>();
    featureRepo.updateFeature(db, id, body);
    return c.json({ ok: true });
  });

  app.post("/:id/assignments", async (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);
    const body = await c.req.json<{ target_type?: featureRepo.FeatureTargetType; target_id?: string }>();
    if (!body.target_type || !body.target_id) {
      return c.json({ error: "target_type and target_id required" }, 400);
    }
    featureRepo.assignFeature(db, id, body.target_type, body.target_id);
    return c.json({ ok: true });
  });

  app.delete("/:id/assignments", async (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const body = await c.req.json<{ target_type?: featureRepo.FeatureTargetType; target_id?: string }>();
    if (!body.target_type || !body.target_id) {
      return c.json({ error: "target_type and target_id required" }, 400);
    }
    featureRepo.unassignFeature(db, id, body.target_type, body.target_id);
    return c.json({ ok: true });
  });

  app.post("/:id/suggestions/refresh", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);
    return c.json({ inserted: featureRepo.refreshSuggestions(db, id) });
  });

  app.post("/:id/suggestions/accept-all", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);

    let accepted = 0;
    for (const suggestion of featureRepo.listSuggestions(db, id)) {
      if (suggestion.status !== "pending") continue;
      if (featureRepo.acceptSuggestion(db, suggestion.id)) accepted++;
    }

    return c.json({ ok: true, accepted });
  });

  app.post("/suggestions/:suggestionId/accept", (c) => {
    const suggestionId = parseInt(c.req.param("suggestionId"), 10);
    if (!featureRepo.acceptSuggestion(db, suggestionId)) return c.json({ error: "Not found" }, 404);
    return c.json({ ok: true });
  });

  app.post("/suggestions/:suggestionId/reject", (c) => {
    const suggestionId = parseInt(c.req.param("suggestionId"), 10);
    if (!featureRepo.rejectSuggestion(db, suggestionId)) return c.json({ error: "Not found" }, 404);
    return c.json({ ok: true });
  });

  app.get("/:id/investigations", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);
    const investigations = investigationRepo.listInvestigationsByFeature(db, id);
    return c.json(
      investigations.map((inv) => {
        const result = investigationRepo.parseInvestigationResult(inv);
        return {
          id: inv.id,
          query: inv.query,
          status: inv.status,
          candidate_count: result?.candidates.length ?? 0,
          created_at: inv.created_at,
          finished_at: inv.finished_at,
          candidates: result?.candidates ?? [],
          hypothesis: result?.hypothesis ?? null,
          suggested_next_steps: result?.suggested_next_steps ?? [],
        };
      }),
    );
  });

  app.post("/:id/index-all", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);

    const assignments = featureRepo.listAssignments(db, id);
    const tasks = featureRepo.getFeatureTasks(db, id);
    const allFiles = [
      ...tasks.map((t) => t.symbol_file),
      ...assignments.filter((a) => a.target_type === "file").map((a) => a.target_id),
    ];
    const fileStatus = featureFileStatus(db, config, [...new Set(allFiles)]);
    const unindexed = fileStatus.filter((f) => !f.indexed && f.kind === "cpp").map((f) => f.path);

    if (!unindexed.length) {
      return c.json({ ok: true, jobIds: [], fileCount: 0, message: "All files already indexed" });
    }

    const jobId = jobsRepo.createJob(db, "index", { paths: unindexed }, unindexed.length);
    queues.enqueue(jobId);

    return c.json({ ok: true, jobIds: [jobId], fileCount: unindexed.length });
  });

  app.post("/:id/document-all", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);

    const assignments = featureRepo.listAssignments(db, id);
    const tasks = featureRepo.getFeatureTasks(db, id);
    const allFiles = [
      ...tasks.map((t) => t.symbol_file),
      ...assignments.filter((a) => a.target_type === "file").map((a) => a.target_id),
    ];
    const fileStatus = featureFileStatus(db, config, [...new Set(allFiles)]);
    const indexed = fileStatus.filter((f) => f.indexed);

    const allTaskIds: number[] = [];
    for (const file of indexed) {
      const ids = fileStatsRepo.getTaskIdsForFile(db, file.path, "discovered");
      allTaskIds.push(...ids);
    }

    if (!allTaskIds.length) {
      return c.json({ ok: true, jobIds: [], totalTasks: 0, message: "No discovered tasks to document" });
    }

    const jobIds = jobsRepo.createBatchedJobs(db, "extract", allTaskIds, config.jobs.maxBatchSize);
    for (const jobId of jobIds) queues.enqueue(jobId);

    return c.json({ ok: true, jobIds, totalTasks: allTaskIds.length, batches: jobIds.length });
  });

  app.post("/:id/assemble-flows-all", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    if (!featureRepo.getFeature(db, id)) return c.json({ error: "Not found" }, 404);

    const assignments = featureRepo.listAssignments(db, id);
    const tasks = featureRepo.getFeatureTasks(db, id);
    const allFiles = [
      ...tasks.map((t) => t.symbol_file),
      ...assignments.filter((a) => a.target_type === "file").map((a) => a.target_id),
    ];
    const fileStatus = featureFileStatus(db, config, [...new Set(allFiles)]);
    const cppFiles = fileStatus.filter((f) => f.indexed && f.kind === "cpp");

    if (!cppFiles.length) {
      return c.json({ ok: true, jobIds: [], fileCount: 0, message: "No indexed cpp files" });
    }

    const jobIds = cppFiles.map((f) => {
      const jobId = jobsRepo.createJob(db, "assemble-flows", { file: f.path });
      queues.enqueue(jobId);
      return jobId;
    });

    return c.json({ ok: true, jobIds, fileCount: cppFiles.length });
  });

  return app;
}
