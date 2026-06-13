import { basename } from "node:path";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import { scanReferenceCppInDir } from "../files/scanner.js";
import * as fileStatsRepo from "../db/repositories/fileStats.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import { indexFiles, applyFlowMappings } from "../index/indexer.js";
import type { JobActivity } from "./jobActivity.js";
import { JobPausedError } from "./jobErrors.js";

export async function runBackgroundJob(
  db: Database.Database,
  config: HarnessConfig,
  jobId: number,
  activity: JobActivity,
  shouldPause: () => boolean,
  enqueueAgent: (jobId: number) => void,
): Promise<void> {
  const job = jobsRepo.getJobById(db, jobId) as {
    id: number;
    stage: string;
    target_ids: string;
    total: number;
    progress: number;
  };

  if (!job) return;

  jobsRepo.beginJob(db, jobId);
  activity.log(`Job #${jobId} started (${job.stage})`);

  try {
    if (shouldPause()) {
      jobsRepo.pauseJob(db, jobId);
      activity.setCurrent(null);
      activity.log("Paused before start");
      throw new JobPausedError(jobId);
    }

    switch (job.stage) {
      case "index": {
        const payload = JSON.parse(job.target_ids) as { path?: string; paths?: string[] };
        const paths = payload.paths ?? (payload.path ? [payload.path] : []);
        if (!paths.length) {
          jobsRepo.finishJob(db, jobId, "failed", "No paths to index");
          return;
        }

        activity.setCurrent("index");
        let totalSymbols = 0;
        for (let i = 0; i < paths.length; i++) {
          if (shouldPause()) {
            jobsRepo.pauseJob(db, jobId);
            activity.setCurrent(null);
            activity.log(`Paused after ${i}/${paths.length} file(s)`);
            throw new JobPausedError(jobId);
          }

          const path = paths[i]!;
          activity.log(`[${i + 1}/${paths.length}] Indexing ${path}`);
          const result = await indexFiles(db, config, [path]);
          totalSymbols += result.symbolsIndexed;
          jobsRepo.updateJobProgress(db, jobId, i + 1);
        }

        const mappingsApplied =
          config.flowMappings && Object.keys(config.flowMappings).length > 0
            ? applyFlowMappings(db, config.flowMappings)
            : 0;
        if (mappingsApplied > 0) {
          activity.log(`Applied ${mappingsApplied} flow mapping(s)`);
        }

        activity.setCurrent(null);
        activity.log(`Indexed ${paths.length} file(s), ${totalSymbols} symbol(s)`);
        jobsRepo.finishJob(db, jobId, "done");
        return;
      }

      case "index-dir": {
        const payload = JSON.parse(job.target_ids) as { dir: string };
        activity.setCurrent("index-dir");
        activity.log(`Indexing directory ${payload.dir}`);

        const files = scanReferenceCppInDir(config.referenceRoot, payload.dir);
        if (!files.length) {
          jobsRepo.finishJob(db, jobId, "failed", `No .cpp files found under ${payload.dir}`);
          return;
        }

        let totalSymbols = 0;
        for (let i = 0; i < files.length; i++) {
          if (shouldPause()) {
            jobsRepo.pauseJob(db, jobId);
            activity.setCurrent(null);
            activity.log(`Paused after ${i}/${files.length} file(s)`);
            throw new JobPausedError(jobId);
          }

          const file = files[i]!;
          activity.log(`[${i + 1}/${files.length}] ${file.path}`);
          const result = await indexFiles(db, config, [file.path]);
          totalSymbols += result.symbolsIndexed;
          jobsRepo.updateJobProgress(db, jobId, i + 1);
        }

        activity.setCurrent(null);
        activity.log(`Indexed ${files.length} file(s), ${totalSymbols} symbol(s)`);
        jobsRepo.finishJob(db, jobId, "done");
        return;
      }

      case "file-pipeline": {
        const payload = JSON.parse(job.target_ids) as { path: string };
        activity.setCurrent("index");
        activity.log(`Pipeline: indexing ${payload.path}`);

        const indexResult = await indexFiles(db, config, [payload.path]);
        const mappingsApplied =
          config.flowMappings && Object.keys(config.flowMappings).length > 0
            ? applyFlowMappings(db, config.flowMappings)
            : 0;
        if (mappingsApplied > 0) {
          activity.log(`Applied ${mappingsApplied} flow mapping(s)`);
        }
        activity.log(
          `Indexed ${indexResult.symbolsIndexed} symbol(s); queuing extract + assemble`,
        );
        jobsRepo.updateJobProgress(db, jobId, 1);

        const name = basename(payload.path);
        const taskIds = fileStatsRepo.getTaskIdsForFile(db, name, "discovered");
        const extractJobIds = taskIds.length
          ? jobsRepo.createBatchedJobs(db, "extract", taskIds, config.jobs.maxBatchSize)
          : [];
        const assembleJobId = jobsRepo.createJob(db, "assemble-flows", { file: payload.path });

        for (const id of extractJobIds) enqueueAgent(id);
        enqueueAgent(assembleJobId);

        activity.setCurrent(null);
        activity.log(
          `Queued ${extractJobIds.length} extract job(s) and assemble-flows #${assembleJobId}`,
        );
        jobsRepo.finishJob(db, jobId, "done");
        return;
      }

      default:
        jobsRepo.finishJob(db, jobId, "failed", `Unknown background stage: ${job.stage}`);
    }
  } catch (err) {
    if (err instanceof JobPausedError) throw err;
    const message = err instanceof Error ? err.message : String(err);
    activity.setCurrent(null);
    activity.log(`Job failed: ${message}`);
    jobsRepo.finishJob(db, jobId, "failed", message);
  }
}
