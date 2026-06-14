import { describe, it, expect } from "vitest";
import Database from "better-sqlite3";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import * as codeSymbolRepo from "../src/db/repositories/codeSymbol.js";
import * as taskRepo from "../src/db/repositories/migrationTask.js";
import * as flowRepo from "../src/db/repositories/businessFlow.js";
import { createFlow, resolveFlow, updateFlow } from "../src/mcp/flowTools.js";

function setupTestDb(): Database.Database {
  const db = new Database(":memory:");
  db.pragma("foreign_keys = ON");
  const schemaDir = join(import.meta.dirname, "..", "schema");
  for (const file of readdirSync(schemaDir).filter((f) => f.endsWith(".sql")).sort()) {
    db.exec(readFileSync(join(schemaDir, file), "utf-8"));
  }
  return db;
}

describe("flowTools", () => {
  it("creates a flow and links its entry tasks", () => {
    const db = setupTestDb();

    const symId = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/flow.cpp",
      name: "Flow::entry",
      kind: "method",
      start_line: 10,
      end_line: 20,
    });

    taskRepo.upsertTask(db, symId, { status: "documented" });

    const result = createFlow(db, {
      name: "flow_test",
      description: "test flow",
      entry_symbols: ["Flow::entry"],
      expected_behaviour: "does the thing",
      risk_level: "medium",
      branches: [
        {
          condition: "if ready",
          behaviour: "proceeds",
          file: "src/game/flow.cpp",
          start_line: 12,
          end_line: 16,
        },
      ],
      mutations: [
        {
          variable_or_field: "state",
          mutation_description: "advances state",
          file: "src/game/flow.cpp",
          start_line: 16,
          end_line: 18,
        },
      ],
    });

    expect(result.ok).toBe(true);
    expect(result.entry_symbol_ids).toEqual([symId]);

    const flow = resolveFlow(db, "flow_test");
    expect(flow?.name).toBe("flow_test");
    expect(JSON.parse(flow?.entry_symbol_ids ?? "[]")).toEqual([symId]);

    const task = taskRepo.getTaskBySymbolId(db, symId);
    expect(task?.flow_id).toBe(flow?.id);

    expect(flowRepo.getBranchesForFlow(db, flow!.id)).toHaveLength(1);
    expect(flowRepo.getMutationsForFlow(db, flow!.id)).toHaveLength(1);

    db.close();
  });

  it("updates a flow by id without clobbering unrelated data", () => {
    const db = setupTestDb();

    const symA = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/flow.cpp",
      name: "Flow::entryA",
      kind: "method",
      start_line: 1,
      end_line: 5,
    });
    const symB = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/flow.cpp",
      name: "Flow::entryB",
      kind: "method",
      start_line: 6,
      end_line: 10,
    });

    taskRepo.upsertTask(db, symA, { status: "documented" });
    taskRepo.upsertTask(db, symB, { status: "discovered" });

    const created = createFlow(db, {
      name: "flow_alpha",
      description: "alpha",
      entry_symbols: ["Flow::entryA"],
      expected_behaviour: "alpha behaviour",
      risk_level: "low",
      branches: [
        {
          condition: "initial",
          behaviour: "starts",
          file: "src/game/flow.cpp",
          start_line: 2,
          end_line: 4,
        },
      ],
    });

    expect(created.ok).toBe(true);
    const before = resolveFlow(db, "flow_alpha");
    expect(before).toBeDefined();

    const updated = updateFlow(db, "flow_alpha", {
      name: "flow_beta",
      description: "beta",
      entry_symbols: ["Flow::entryB"],
      expected_behaviour: "beta behaviour",
      risk_level: "high",
      replace_data: false,
    });

    expect(updated.ok).toBe(true);
    const after = resolveFlow(db, "flow_beta");
    expect(after?.id).toBe(before?.id);
    expect(resolveFlow(db, "flow_alpha")).toBeUndefined();
    expect(taskRepo.getTaskBySymbolId(db, symA)?.flow_id).toBeNull();
    expect(taskRepo.getTaskBySymbolId(db, symB)?.flow_id).toBe(after?.id);
    expect(flowRepo.getBranchesForFlow(db, after!.id)).toHaveLength(1);

    db.close();
  });
});
