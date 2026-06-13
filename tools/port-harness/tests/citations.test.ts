import { describe, it, expect } from "vitest";
import { validateClaims, formatCitation } from "../src/verify/citations.js";
import { matchesExcludePattern, canTransitionToPort } from "../src/models/index.js";
import { chunkLargeMethod, shouldChunk } from "../src/index/chunker.js";

describe("validateClaims", () => {
  it("accepts valid claims with citations", () => {
    const result = validateClaims(
      [
        {
          symbol_id: 1,
          category: "branch",
          claim_text: "Returns false if power insufficient",
          file: "Spell.cpp",
          start_line: 100,
          end_line: 105,
          confidence: "high",
        },
      ],
      "Spell.cpp",
      50,
      200,
    );
    expect(result.valid).toBe(true);
    expect(result.claims).toHaveLength(1);
  });

  it("rejects claims missing file", () => {
    const result = validateClaims(
      [
        {
          symbol_id: 1,
          category: "branch",
          claim_text: "No citation",
          file: "",
          start_line: 100,
          end_line: 105,
          confidence: "high",
        },
      ],
      "Spell.cpp",
      50,
      200,
    );
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it("warns on out-of-range citations but still validates structure", () => {
    const result = validateClaims(
      [
        {
          symbol_id: 1,
          category: "output",
          claim_text: "Returns SPELL_FAILED",
          file: "Spell.cpp",
          start_line: 10,
          end_line: 15,
          confidence: "high",
        },
      ],
      "Spell.cpp",
      50,
      200,
    );
    expect(result.errors.some((e) => e.includes("outside symbol range"))).toBe(true);
  });
});

describe("formatCitation", () => {
  it("formats single line", () => {
    expect(formatCitation("Spell.cpp", 42, 42)).toBe("Spell.cpp:42");
  });

  it("formats range", () => {
    expect(formatCitation("Spell.cpp", 42, 50)).toBe("Spell.cpp:42-50");
  });
});

describe("matchesExcludePattern", () => {
  it("matches wildcard prefix", () => {
    expect(matchesExcludePattern("EffectSchoolDMG", ["Effect*"])).toBe(true);
    expect(matchesExcludePattern("CheckCast", ["Effect*"])).toBe(false);
  });
});

describe("canTransitionToPort", () => {
  it("allows port from documented states", () => {
    expect(canTransitionToPort("documented")).toBe(true);
    expect(canTransitionToPort("rust_planned")).toBe(true);
    expect(canTransitionToPort("discovered")).toBe(false);
  });
});

describe("chunker", () => {
  it("chunks large methods", () => {
    const symbol = {
      file: "Spell.cpp",
      name: "Spell::CheckCast",
      kind: "method" as const,
      startLine: 1,
      endLine: 400,
    };
    expect(shouldChunk(symbol, 150)).toBe(true);
    const chunks = chunkLargeMethod(symbol, 150);
    expect(chunks.length).toBeGreaterThan(1);
    expect(chunks[0]!.name).toContain("chunk_0");
  });

  it("does not chunk small methods", () => {
    const symbol = {
      file: "Spell.cpp",
      name: "Spell::TakePower",
      kind: "method" as const,
      startLine: 1,
      endLine: 50,
    };
    expect(shouldChunk(symbol, 150)).toBe(false);
    expect(chunkLargeMethod(symbol, 150)).toHaveLength(0);
  });
});
