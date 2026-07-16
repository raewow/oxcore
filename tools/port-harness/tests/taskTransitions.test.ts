import { describe, expect, it } from "vitest";
import { canManuallyTransitionStatus, canMarkTaskDone } from "../src/db/repositories/migrationTask.js";

describe("manual task transitions", () => {
  it("requires evidence writes for verification and review", () => {
    expect(canManuallyTransitionStatus("rust_compiled", "verified")).toBe(false);
    expect(canManuallyTransitionStatus("verified", "reviewed")).toBe(false);
    expect(canManuallyTransitionStatus("reviewed", "done")).toBe(true);
  });

  it("only completes reviewed tasks", () => {
    expect(canMarkTaskDone("verified")).toBe(false);
    expect(canMarkTaskDone("reviewed")).toBe(true);
  });
});
