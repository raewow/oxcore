import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import { runPipelineJob } from "../agents/pipeline.js";
import { resolveActiveProvider } from "./activeProvider.js";
import { createJobActivity } from "./jobActivity.js";
import { JobPausedError } from "./jobErrors.js";
import { runBackgroundJob } from "./backgroundJobs.js";
import { queueKindForStage, type JobQueueKind } from "./jobStages.js";
import type { JobActivity } from "./jobActivity.js";

type JobRunner = (
  jobId: number,
  activity: JobActivity,
  shouldPause: () => boolean,
) => Promise<void>;

export class JobQueue {
  private queue: number[] = [];
  private activeJobIds = new Set<number>();
  private pauseRequested = new Set<number>();

  constructor(
    private db: Database.Database,
    private concurrency: number,
    private runJobImpl: JobRunner,
  ) {}

  getActiveJobIds(): ReadonlySet<number> {
    return this.activeJobIds;
  }

  isProcessing(): boolean {
    return this.activeJobIds.size > 0;
  }

  isPauseRequested(jobId: number): boolean {
    return this.pauseRequested.has(jobId);
  }

  enqueue(jobId: number): void {
    if (!this.queue.includes(jobId)) {
      this.queue.push(jobId);
    }
    this.pump();
  }

  removeFromQueue(jobId: number): void {
    this.queue = this.queue.filter((id) => id !== jobId);
  }

  requestPause(jobId: number): { ok: boolean; immediate?: boolean; pausing?: boolean; error?: string } {
    const job = jobsRepo.getJobById(this.db, jobId) as { status: string } | undefined;
    if (!job) return { ok: false, error: "Not found" };

    if (job.status === "queued") {
      if (!jobsRepo.pauseJob(this.db, jobId)) {
        return { ok: false, error: "Could not pause job" };
      }
      this.removeFromQueue(jobId);
      return { ok: true, immediate: true };
    }

    if (job.status === "running" && this.activeJobIds.has(jobId)) {
      this.pauseRequested.add(jobId);
      return { ok: true, pausing: true };
    }

    return { ok: false, error: "Only queued or active jobs can be paused" };
  }

  resume(jobId: number): boolean {
    if (!jobsRepo.resumeJob(this.db, jobId)) return false;
    this.pauseRequested.delete(jobId);
    this.enqueue(jobId);
    return true;
  }

  private pump(): void {
    while (this.activeJobIds.size < this.concurrency && this.queue.length > 0) {
      const jobId = this.queue.shift()!;
      const job = jobsRepo.getJobById(this.db, jobId) as { status: string } | undefined;
      if (!job || job.status === "paused") continue;
      void this.runJob(jobId);
    }
  }

  private async runJob(jobId: number): Promise<void> {
    this.activeJobIds.add(jobId);

    try {
      const activity = createJobActivity(this.db, jobId);
      await this.runJobImpl(jobId, activity, () => this.pauseRequested.has(jobId));
    } catch (err) {
      if (err instanceof JobPausedError) {
        this.pauseRequested.delete(jobId);
      } else {
        jobsRepo.finishJob(
          this.db,
          jobId,
          "failed",
          err instanceof Error ? err.message : String(err),
        );
      }
    } finally {
      this.activeJobIds.delete(jobId);
      this.pump();
    }
  }
}

export class JobQueues {
  readonly agent: JobQueue;
  readonly background: JobQueue;

  constructor(
    private db: Database.Database,
    config: HarnessConfig,
  ) {
    this.agent = new JobQueue(
      db,
      Math.max(1, config.jobs.concurrency),
      async (jobId, activity, shouldPause) => {
        const provider = await resolveActiveProvider(db, config, (msg) => activity.log(msg));
        await runPipelineJob(db, config, provider, jobId, activity, shouldPause);
      },
    );

    this.background = new JobQueue(
      db,
      Math.max(1, config.jobs.backgroundConcurrency),
      (jobId, activity, shouldPause) =>
        runBackgroundJob(db, config, jobId, activity, shouldPause, (id) =>
          this.agent.enqueue(id),
        ),
    );
  }

  start(): void {
    jobsRepo.recoverStaleRunningJobs(this.db);
    const queued = jobsRepo.getQueuedJobs(this.db) as { id: number; stage: string }[];
    for (const job of queued) {
      this.enqueue(job.id);
    }
  }

  enqueue(jobId: number): void {
    const job = jobsRepo.getJobById(this.db, jobId) as { stage: string } | undefined;
    if (!job) return;
    this.queueForKind(queueKindForStage(job.stage)).enqueue(jobId);
  }

  getActiveJobIds(): ReadonlySet<number> {
    return new Set([...this.agent.getActiveJobIds(), ...this.background.getActiveJobIds()]);
  }

  /** @deprecated Use getActiveJobIds */
  getActiveJobId(): number | null {
    const first = this.getActiveJobIds().values().next();
    return first.done ? null : first.value;
  }

  isProcessing(): boolean {
    return this.agent.isProcessing() || this.background.isProcessing();
  }

  isPauseRequested(jobId: number): boolean {
    return this.agent.isPauseRequested(jobId) || this.background.isPauseRequested(jobId);
  }

  requestPause(jobId: number): { ok: boolean; immediate?: boolean; pausing?: boolean; error?: string } {
    return this.queueForJob(jobId).requestPause(jobId);
  }

  resume(jobId: number): boolean {
    return this.queueForJob(jobId).resume(jobId);
  }

  private queueForJob(jobId: number): JobQueue {
    const job = jobsRepo.getJobById(this.db, jobId) as { stage: string } | undefined;
    if (!job) return this.agent;
    return this.queueForKind(queueKindForStage(job.stage));
  }

  private queueForKind(kind: JobQueueKind): JobQueue {
    return kind === "agent" ? this.agent : this.background;
  }
}

/** @deprecated Use JobQueues */
export { JobQueues as JobQueueManager };
