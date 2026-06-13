import type Database from "better-sqlite3";
import type { JobStatus } from "../../models/index.js";

const STAGE_LABELS: Record<string, string> = {
  extract: "Extract behaviour",
  "assemble-flows": "Assemble flows",
  "plan-rust": "Plan Rust",
  port: "Port",
  verify: "Verify",
  discover: "Discover",
  "audit-rust": "Audit Rust impl",
};

export interface JobTarget {
  taskId: number;
  symbolName: string;
  file: string;
  state: "done" | "running" | "pending";
}

export interface EnrichedJob {
  id: number;
  stage: string;
  target_ids: string;
  status: string;
  progress: number;
  total: number;
  error: string | null;
  current_item: string | null;
  log: string;
  created_at: string;
  finished_at: string | null;
  summary: string;
  stage_label: string;
  targets: JobTarget[];
  last_log_line: string | null;
}

interface RawJob {
  id: number;
  stage: string;
  target_ids: string;
  status: string;
  progress: number;
  total: number;
  error: string | null;
  current_item: string | null;
  log: string | null;
  created_at: string;
  finished_at: string | null;
}

export function getLastLogLine(log: string | null | undefined): string | null {
  if (!log?.trim()) return null;
  const lines = log.split("\n").filter(Boolean);
  return lines.length ? lines[lines.length - 1]! : null;
}

export function enrichJob(
  db: Database.Database,
  job: RawJob,
  options: { targetLimit?: number } = {},
): EnrichedJob {
  const stageLabel = STAGE_LABELS[job.stage] ?? job.stage;
  const log = job.log ?? "";
  let summary = stageLabel;
  let targets: JobTarget[] = [];

  try {
    const parsed = JSON.parse(job.target_ids);

    if (job.stage === "assemble-flows" && parsed && typeof parsed === "object" && "file" in parsed) {
      const file = String((parsed as { file: string }).file);
      summary = `${stageLabel} · ${file}`;
    } else if (
      job.stage === "discover" &&
      parsed &&
      typeof parsed === "object" &&
      "query" in parsed
    ) {
      const q = String((parsed as { query: string }).query);
      summary = `${stageLabel} · ${q.length > 50 ? `${q.slice(0, 50)}…` : q}`;
    } else if (Array.isArray(parsed)) {
      const taskIds = parsed as number[];
      const files = new Set<string>();

      for (let i = 0; i < taskIds.length; i++) {
        const taskId = taskIds[i]!;
        const row = db
          .prepare(
            `SELECT cs.name AS symbol_name, cs.file AS symbol_file
             FROM migration_task mt
             JOIN code_symbol cs ON cs.id = mt.source_symbol_id
             WHERE mt.id = ?`,
          )
          .get(taskId) as { symbol_name: string; symbol_file: string } | undefined;

        const symbolName = row?.symbol_name ?? `task ${taskId}`;
        const file = row?.symbol_file ?? "";
        if (file) files.add(file);

        let state: JobTarget["state"] = "pending";
        if (i < job.progress) state = "done";
        else if (i === job.progress && job.status === "running") state = "running";

        targets.push({ taskId, symbolName, file, state });
      }

      const fileHint =
        files.size === 1
          ? [...files][0]
          : files.size > 1
            ? `${files.size} files`
            : null;
      summary = fileHint
        ? `${stageLabel} · ${taskIds.length} symbol${taskIds.length === 1 ? "" : "s"} · ${fileHint}`
        : `${stageLabel} · ${taskIds.length} symbol${taskIds.length === 1 ? "" : "s"}`;
    }
  } catch {
    summary = stageLabel;
  }

  const limit = options.targetLimit;
  const limitedTargets = limit !== undefined ? targets.slice(0, limit) : targets;

  return {
    ...job,
    log,
    summary,
    stage_label: stageLabel,
    targets: limitedTargets,
    last_log_line: getLastLogLine(log),
  };
}

export function enrichJobs(
  db: Database.Database,
  jobs: RawJob[],
  options: { targetLimit?: number } = {},
): EnrichedJob[] {
  return jobs.map((j) => enrichJob(db, j, options));
}

export function createJob(
  db: Database.Database,
  stage: string,
  targetIds: number[] | Record<string, unknown>,
  total?: number,
): number {
  const payload = Array.isArray(targetIds) ? targetIds : targetIds;
  const count = total ?? (Array.isArray(targetIds) ? targetIds.length : 1);
  const result = db
    .prepare(`INSERT INTO pipeline_job (stage, target_ids, total) VALUES (?, ?, ?)`)
    .run(stage, JSON.stringify(payload), count);
  return Number(result.lastInsertRowid);
}

export function createBatchedJobs(
  db: Database.Database,
  stage: string,
  taskIds: number[],
  maxBatchSize: number,
): number[] {
  const jobIds: number[] = [];
  for (let i = 0; i < taskIds.length; i += maxBatchSize) {
    const batch = taskIds.slice(i, i + maxBatchSize);
    jobIds.push(createJob(db, stage, batch));
  }
  return jobIds;
}

export function getJobById(db: Database.Database, id: number) {
  return db.prepare("SELECT * FROM pipeline_job WHERE id = ?").get(id);
}

export function listJobs(db: Database.Database, limit = 20) {
  return db
    .prepare("SELECT * FROM pipeline_job ORDER BY created_at DESC LIMIT ?")
    .all(limit);
}

export function updateJobProgress(
  db: Database.Database,
  id: number,
  progress: number,
  status?: JobStatus,
): void {
  if (status) {
    db.prepare(
      `UPDATE pipeline_job SET progress = ?, status = ? WHERE id = ?`,
    ).run(progress, status, id);
  } else {
    db.prepare(`UPDATE pipeline_job SET progress = ? WHERE id = ?`).run(progress, id);
  }
}

export function finishJob(
  db: Database.Database,
  id: number,
  status: JobStatus,
  error?: string,
): void {
  db.prepare(
    `UPDATE pipeline_job SET status = ?, error = ?, finished_at = datetime('now') WHERE id = ?`,
  ).run(status, error ?? null, id);
}

export function getQueuedJobs(db: Database.Database) {
  return db
    .prepare("SELECT * FROM pipeline_job WHERE status = 'queued' ORDER BY created_at")
    .all();
}

export function recoverStaleRunningJobs(db: Database.Database): number {
  const result = db
    .prepare(
      `UPDATE pipeline_job
       SET status = 'failed', error = 'Server restarted — use Continue to resume', finished_at = datetime('now')
       WHERE status = 'running'`,
    )
    .run();
  return result.changes;
}

/** Fail orphaned running jobs when another worker takes over. */
export function supersedeOtherRunningJobs(
  db: Database.Database,
  exceptJobId: number,
  reason = "Superseded — another job started",
): number {
  const result = db
    .prepare(
      `UPDATE pipeline_job
       SET status = 'failed', error = ?, finished_at = datetime('now'), current_item = NULL
       WHERE status = 'running' AND id != ?`,
    )
    .run(reason, exceptJobId);
  return result.changes;
}

/** Fail any running job that is not tracked by an in-memory worker. */
export function reconcileStaleRunningJobs(
  db: Database.Database,
  activeJobIds: ReadonlySet<number> | number[] | number | null,
): number {
  const active = new Set<number>(
    activeJobIds === null
      ? []
      : typeof activeJobIds === "number"
        ? [activeJobIds]
        : Array.isArray(activeJobIds)
          ? activeJobIds
          : [...activeJobIds],
  );

  const running = db
    .prepare("SELECT id FROM pipeline_job WHERE status = 'running' ORDER BY id")
    .all() as { id: number }[];

  let failed = 0;
  for (const row of running) {
    if (active.has(row.id)) continue;
    finishJob(db, row.id, "failed", "Stale — no active worker (use Continue to resume)");
    failed++;
  }
  return failed;
}

/** @deprecated Use reconcileStaleRunningJobs */
export function reconcileDuplicateRunningJobs(
  db: Database.Database,
  preferredJobId: number | null,
): number {
  return reconcileStaleRunningJobs(db, preferredJobId);
}

export function beginJob(db: Database.Database, id: number): void {
  const job = getJobById(db, id) as { progress: number } | undefined;
  const progress = job?.progress ?? 0;
  updateJobProgress(db, id, progress, "running");
}

export function pauseJob(db: Database.Database, id: number): boolean {
  const result = db
    .prepare(
      `UPDATE pipeline_job
       SET status = 'paused', current_item = NULL, finished_at = NULL
       WHERE id = ? AND status IN ('queued', 'running')`,
    )
    .run(id);
  return result.changes > 0;
}

export function resumeJob(db: Database.Database, id: number): boolean {
  const result = db
    .prepare(
      `UPDATE pipeline_job
       SET status = 'queued', error = NULL, finished_at = NULL
       WHERE id = ? AND status = 'paused' AND progress < total`,
    )
    .run(id);
  return result.changes > 0;
}

export function abandonJob(db: Database.Database, id: number): boolean {
  const result = db
    .prepare(
      `UPDATE pipeline_job
       SET status = 'failed', error = 'Abandoned — use Continue to resume', finished_at = datetime('now'), current_item = NULL
       WHERE id = ? AND status = 'running'`,
    )
    .run(id);
  return result.changes > 0;
}

export function deleteJob(db: Database.Database, id: number): boolean {
  const result = db
    .prepare(
      `DELETE FROM pipeline_job WHERE id = ? AND status IN ('done', 'failed', 'cancelled', 'paused')`,
    )
    .run(id);
  return result.changes > 0;
}

export function cancelJob(db: Database.Database, id: number): boolean {
  const result = db
    .prepare(
      `UPDATE pipeline_job SET status = 'cancelled', finished_at = datetime('now')
       WHERE id = ? AND status = 'queued'`,
    )
    .run(id);
  return result.changes > 0;
}

export function resetJobForRetry(db: Database.Database, id: number): boolean {
  const result = db
    .prepare(
      `UPDATE pipeline_job
       SET status = 'queued', progress = 0, error = NULL, finished_at = NULL,
           current_item = NULL, log = ''
       WHERE id = ? AND status IN ('done', 'failed', 'cancelled')`,
    )
    .run(id);
  return result.changes > 0;
}

export function cloneJobRemaining(db: Database.Database, id: number): number | null {
  const job = getJobById(db, id) as RawJob | undefined;
  if (!job) return null;
  if (!["failed", "cancelled"].includes(job.status)) return null;
  if (job.progress >= job.total) return null;

  let remaining: number[] | Record<string, unknown>;
  try {
    const parsed = JSON.parse(job.target_ids);
    if (job.stage === "assemble-flows" || job.stage === "discover") {
      return null;
    }
    if (!Array.isArray(parsed)) return null;
    remaining = (parsed as number[]).slice(job.progress);
    if (!remaining.length) return null;
  } catch {
    return null;
  }

  return createJob(db, job.stage, remaining);
}

export function insertAgentRun(
  db: Database.Database,
  stage: string,
  provider: string,
  model: string,
  promptHash: string,
  outputJson: string,
  symbolId?: number,
  taskId?: number,
): number {
  const result = db
    .prepare(
      `INSERT INTO agent_run (stage, provider, model, prompt_hash, output_json, symbol_id, task_id)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
    )
    .run(stage, provider, model, promptHash, outputJson, symbolId ?? null, taskId ?? null);
  return Number(result.lastInsertRowid);
}

export function getRecentAgentRuns(db: Database.Database, limit = 10) {
  return db
    .prepare("SELECT * FROM agent_run ORDER BY created_at DESC LIMIT ?")
    .all(limit);
}

export function insertFixture(
  db: Database.Database,
  name: string,
  inputJson: string,
  expectedJson: string,
  coversFlowIds: number[],
  symbolId?: number,
): number {
  const result = db
    .prepare(
      `INSERT INTO test_fixture (name, input_json, expected_json, covers_flow_ids, symbol_id)
       VALUES (?, ?, ?, ?, ?)`,
    )
    .run(name, inputJson, expectedJson, JSON.stringify(coversFlowIds), symbolId ?? null);
  return Number(result.lastInsertRowid);
}

export function countFixturesForSymbol(db: Database.Database, symbolId: number): number {
  const row = db
    .prepare("SELECT COUNT(*) as c FROM test_fixture WHERE symbol_id = ?")
    .get(symbolId) as { c: number };
  return row.c;
}

export function getFixturesForSymbol(db: Database.Database, symbolId: number) {
  return db.prepare("SELECT * FROM test_fixture WHERE symbol_id = ?").all(symbolId);
}

export function getFixturesForFlow(db: Database.Database, flowId: number) {
  return db
    .prepare("SELECT * FROM test_fixture WHERE covers_flow_ids LIKE ?")
    .all(`%${flowId}%`);
}
