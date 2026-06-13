import { describe, it, expect } from "vitest";
import Database from "better-sqlite3";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import * as codeSymbolRepo from "../src/db/repositories/codeSymbol.js";
import * as taskRepo from "../src/db/repositories/migrationTask.js";
import * as claimRepo from "../src/db/repositories/behaviourClaim.js";
import * as jobsRepo from "../src/db/repositories/jobs.js";

function setupTestDb(): Database.Database {
  const db = new Database(":memory:");
  db.pragma("foreign_keys = ON");
  const schemaDir = join(import.meta.dirname, "..", "schema");
  for (const file of readdirSync(schemaDir).filter((f) => f.endsWith(".sql")).sort()) {
    db.exec(readFileSync(join(schemaDir, file), "utf-8"));
  }
  return db;
}

describe("cloneJobRemaining", () => {
  it("filters already-extracted tasks when continuing an extract job", () => {
    const db = setupTestDb();

    const symDone = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/A.cpp",
      name: "A::done",
      kind: "method",
      start_line: 1,
      end_line: 10,
    });
    const symPending = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/A.cpp",
      name: "A::pending",
      kind: "method",
      start_line: 20,
      end_line: 30,
    });

    const taskDone = taskRepo.upsertTask(db, symDone, { status: "documented" });
    const taskPending = taskRepo.upsertTask(db, symPending, { status: "discovered" });
    claimRepo.insertClaim(db, {
      symbol_id: symDone,
      category: "output",
      claim_text: "Already documented",
      file: "src/game/A.cpp",
      start_line: 1,
      end_line: 2,
      confidence: "high",
    });

    const jobId = jobsRepo.createJob(db, "extract", [taskDone, taskPending]);
    jobsRepo.finishJob(db, jobId, "failed", "interrupted");
    db.prepare("UPDATE pipeline_job SET progress = 1 WHERE id = ?").run(jobId);

    const continuedId = jobsRepo.cloneJobRemaining(db, jobId);
    expect(continuedId).not.toBeNull();

    const continued = jobsRepo.getJobById(db, continuedId!) as {
      target_ids: string;
      total: number;
    };
    const targets = JSON.parse(continued.target_ids) as number[];
    expect(targets).toEqual([taskPending]);
    expect(continued.total).toBe(1);

    db.close();
  });

  it("returns null when all remaining extract tasks are already done", () => {
    const db = setupTestDb();

    const symId = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/B.cpp",
      name: "B::done",
      kind: "method",
      start_line: 1,
      end_line: 10,
    });
    const taskId = taskRepo.upsertTask(db, symId, { status: "documented" });

    const jobId = jobsRepo.createJob(db, "extract", [taskId]);
    jobsRepo.finishJob(db, jobId, "failed");
    db.prepare("UPDATE pipeline_job SET progress = 0 WHERE id = ?").run(jobId);

    expect(jobsRepo.cloneJobRemaining(db, jobId)).toBeNull();

    db.close();
  });
});
