import { describe, expect, it } from "vitest";
import { buildCodexExecArgs, resolveCodexBin } from "../src/agents/providers/codex.js";

describe("codex provider", () => {
  it("builds structured JSON exec args with the selected model and sandbox", () => {
    const args = buildCodexExecArgs({
      model: "gpt-5.4-mini",
      sandbox: "workspace-write",
      schemaFile: "schema.json",
    });

    expect(args).toContain("exec");
    expect(args).toContain("--json");
    expect(args).toContain("--output-schema");
    expect(args).toContain("schema.json");
    expect(args).toContain("gpt-5.4-mini");
    expect(args).toContain("workspace-write");
    expect(args.at(-1)).toBe("-");
  });

  it("preserves explicit codex paths", () => {
    expect(resolveCodexBin("C:\\Tools\\codex.exe")).toBe("C:\\Tools\\codex.exe");
  });
});
