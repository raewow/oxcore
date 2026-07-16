import { describe, it, expect } from "vitest";
import Database from "better-sqlite3";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { summarizeFlowProgress } from "../src/db/repositories/flowProgress.js";
import type { TaskAuditSummary } from "../src/db/repositories/flowAudits.js";
import { getLatestAuditsForTasks } from "../src/db/repositories/flowAudits.js";

function setupTestDb(): Database.Database {
  const db = new Database(":memory:");
  db.pragma("foreign_keys = ON");
  const schemaDir = join(import.meta.dirname, "..", "schema");
  for (const file of readdirSync(schemaDir).filter((f) => f.endsWith(".sql")).sort()) {
    db.exec(readFileSync(join(schemaDir, file), "utf-8"));
  }
  return db;
}

describe("summarizeFlowProgress", () => {
  it("keeps audits scoped to their task when runs share a timestamp", () => {
    const db = setupTestDb();
    db.prepare("INSERT INTO code_symbol (file, name, kind, start_line, end_line) VALUES ('A.cpp', 'A::one', 'method', 1, 2)").run();
    db.prepare("INSERT INTO code_symbol (file, name, kind, start_line, end_line) VALUES ('B.cpp', 'B::two', 'method', 1, 2)").run();
    db.prepare("INSERT INTO migration_task (source_symbol_id) VALUES (1)").run();
    db.prepare("INSERT INTO migration_task (source_symbol_id) VALUES (2)").run();
    const output = JSON.stringify({
      implementation_status: "missing",
      passed: false,
      coverage: { claims_covered: 0, claims_total: 0 },
      summary: "missing",
    });
    db.prepare("INSERT INTO agent_run (stage, provider, model, prompt_hash, output_json, task_id) VALUES ('audit-rust', 'test', 'test', 'one', ?, 1)").run(output);
    db.prepare("INSERT INTO agent_run (stage, provider, model, prompt_hash, output_json, task_id) VALUES ('audit-rust', 'test', 'test', 'two', ?, 2)").run(output);

    const audits = getLatestAuditsForTasks(db, [1]);
    expect([...audits.keys()]).toEqual([1]);
    db.close();
  });

  it("derives audit stage when symbols are not audited", () => {
    const progress = summarizeFlowProgress(
      [1, 2],
      new Map(),
      new Map([
        [1, "documented"],
        [2, "documented"],
      ]),
      new Map(),
      new Map(),
    );

    expect(progress.stage).toBe("audit");
    expect(progress.needs_audit).toBe(2);
    expect(progress.percent).toBe(0);
  });

  it("derives plan stage after audit", () => {
    const audit: TaskAuditSummary = {
      task_id: 1,
      audited_at: "2026-01-01",
      implementation_status: "missing",
      passed: false,
      coverage: { claims_covered: 0, claims_total: 5 },
      summary: "missing",
      issues: [],
      rust_locations: [],
    };

    const progress = summarizeFlowProgress(
      [1],
      new Map([[1, audit]]),
      new Map([[1, "documented"]]),
      new Map(),
      new Map(),
    );

    expect(progress.stage).toBe("plan");
    expect(progress.needs_plan).toBe(1);
    expect(progress.audited).toBe(1);
    expect(progress.percent).toBe(25);
  });

  it("marks done when all symbols are reviewed", () => {
    const progress = summarizeFlowProgress(
      [1, 2],
      new Map(),
      new Map([
        [1, "reviewed"],
        [2, "done"],
      ]),
      new Map(),
      new Map(),
    );

    expect(progress.stage).toBe("done");
    expect(progress.done).toBe(2);
    expect(progress.percent).toBe(100);
  });
});
