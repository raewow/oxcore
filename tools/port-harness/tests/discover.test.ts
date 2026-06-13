import { describe, it, expect } from "vitest";
import Database from "better-sqlite3";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { tokenizeQuery, searchHarness } from "../src/discover/search.js";
import * as codeSymbolRepo from "../src/db/repositories/codeSymbol.js";
import * as taskRepo from "../src/db/repositories/migrationTask.js";
import * as claimRepo from "../src/db/repositories/behaviourClaim.js";

function setupTestDb(): Database.Database {
  const db = new Database(":memory:");
  db.pragma("foreign_keys = ON");
  const schemaDir = join(import.meta.dirname, "..", "schema");
  for (const file of readdirSync(schemaDir).filter((f) => f.endsWith(".sql")).sort()) {
    db.exec(readFileSync(join(schemaDir, file), "utf-8"));
  }
  return db;
}

describe("discover/search", () => {
  it("tokenizeQuery drops stopwords and short tokens", () => {
    expect(tokenizeQuery("There is a bug where quest npc talk")).toEqual(
      expect.arrayContaining(["quest", "npc", "talk"]),
    );
    expect(tokenizeQuery("There is a bug where quest npc talk")).not.toContain("there");
  });

  it("searchHarness finds symbols and claims by keyword", () => {
    const db = setupTestDb();
    const symId = codeSymbolRepo.upsertSymbol(db, {
      file: "src/game/QuestHandler.cpp",
      name: "QuestHandler::HandleQuestgiverStatusQuery",
      kind: "method",
      start_line: 10,
      end_line: 50,
      summary: "Checks quest availability for NPC gossip",
    });
    taskRepo.upsertTask(db, symId, { status: "discovered" });
    claimRepo.insertClaim(db, {
      symbol_id: symId,
      category: "branch",
      claim_text: "Returns unavailable when player cannot take quest",
      file: "src/game/QuestHandler.cpp",
      start_line: 20,
      end_line: 25,
      confidence: "high",
    });

    const hits = searchHarness(db, "quest available npc gossip");
    expect(hits.length).toBeGreaterThan(0);
    expect(hits.some((h) => h.symbol.includes("QuestHandler"))).toBe(true);

    db.close();
  });
});
