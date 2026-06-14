import { describe, expect, it } from "vitest";
import { z } from "zod";
import { schemaToJsonSchema } from "../src/agents/provider.js";
import { buildCodexExecArgs, normalizeSchemaForCodex, resolveCodexBin } from "../src/agents/providers/codex.js";

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

  it("normalizes object schemas to require every property for Codex structured output", () => {
    const schema = schemaToJsonSchema(
      z.object({
        rust_locations: z.array(
          z.object({
            file: z.string(),
            symbol: z.string(),
            start_line: z.number().nullable(),
          }),
        ),
      }),
    );
    const normalized = normalizeSchemaForCodex(schema);
    const rustLocations = (normalized.properties as Record<string, unknown>).rust_locations as {
      items: { required: string[]; additionalProperties: boolean };
    };

    expect(normalized.required).toEqual(["rust_locations"]);
    expect(rustLocations.items.required).toEqual(["file", "symbol", "start_line"]);
    expect(rustLocations.items.additionalProperties).toBe(false);
  });
});
