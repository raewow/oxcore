import { describe, it, expect } from "vitest";
import { summarizeFlowProgress } from "../src/db/repositories/flowProgress.js";
import type { TaskAuditSummary } from "../src/db/repositories/flowAudits.js";

describe("summarizeFlowProgress", () => {
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
