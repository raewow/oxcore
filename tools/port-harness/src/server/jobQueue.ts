import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import { getProviderFromConfig } from "../agents/provider.js";
import { runPipelineJob } from "../agents/pipeline.js";
import { createJobActivity } from "./jobActivity.js";
import { JobPausedError } from "./jobErrors.js";

export class JobQueue {
  private queue: number[] = [];
  private activeJobIds = new Set<number>();
  private pauseRequested = new Set<number>();

  constructor(
    private db: Database.Database,
    private config: HarnessConfig,
  ) {}

  getActiveJobIds(): ReadonlySet<number> {
    return this.activeJobIds;
  }

  /** @deprecated Use getActiveJobIds */
  getActiveJobId(): number | null {
    const first = this.activeJobIds.values().next();
    return first.done ? null : first.value;
  }

  isProcessing(): boolean {
    return this.activeJobIds.size > 0;
  }

  isPauseRequested(jobId: number): boolean {
    return this.pauseRequested.has(jobId);
  }

  private get concurrency(): number {
    return Math.max(1, this.config.jobs.concurrency);
  }

  start(): void {
    jobsRepo.recoverStaleRunningJobs(this.db);
    const queued = jobsRepo.getQueuedJobs(this.db) as { id: number }[];
    for (const job of queued) {
      this.enqueue(job.id);
    }
  }

  enqueue(jobId: number): void {
    if (!this.queue.includes(jobId)) {
      this.queue.push(jobId);
    }
    this.pump();
  }

  requestPause(jobId: number): { ok: boolean; immediate?: boolean; pausing?: boolean; error?: string } {
    const job = jobsRepo.getJobById(this.db, jobId) as { status: string } | undefined;
    if (!job) return { ok: false, error: "Not found" };

    if (job.status === "queued") {
      if (!jobsRepo.pauseJob(this.db, jobId)) {
        return { ok: false, error: "Could not pause job" };
      }
      this.queue = this.queue.filter((id) => id !== jobId);
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
      const provider = await getProviderFromConfig({
        ...this.config.provider,
        rustRoot: this.config.rustRoot,
        onActivity: (msg) => activity.log(msg),
      });
      await runPipelineJob(
        this.db,
        this.config,
        provider,
        jobId,
        activity,
        () => this.pauseRequested.has(jobId),
      );
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
