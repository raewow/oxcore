import type Database from "better-sqlite3";
import type { BehaviourClaim, ClaimCategory } from "../../models/index.js";

export interface InsertClaim {
  symbol_id: number;
  category: ClaimCategory;
  claim_text: string;
  file: string;
  start_line: number;
  end_line: number;
  confidence?: string;
}

export function insertClaim(db: Database.Database, claim: InsertClaim): number {
  const result = db
    .prepare(
      `INSERT INTO behaviour_claim (symbol_id, category, claim_text, file, start_line, end_line, confidence)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
    )
    .run(
      claim.symbol_id,
      claim.category,
      claim.claim_text,
      claim.file,
      claim.start_line,
      claim.end_line,
      claim.confidence ?? "high",
    );
  return Number(result.lastInsertRowid);
}

export function deleteClaimsForSymbol(db: Database.Database, symbolId: number): void {
  db.prepare("DELETE FROM behaviour_claim WHERE symbol_id = ?").run(symbolId);
}

export function getClaimsForSymbol(db: Database.Database, symbolId: number): BehaviourClaim[] {
  return db
    .prepare("SELECT * FROM behaviour_claim WHERE symbol_id = ? ORDER BY start_line")
    .all(symbolId) as BehaviourClaim[];
}

export function countClaimsForSymbol(db: Database.Database, symbolId: number): number {
  const row = db
    .prepare("SELECT COUNT(*) as c FROM behaviour_claim WHERE symbol_id = ?")
    .get(symbolId) as { c: number };
  return row.c;
}

export function insertDependency(
  db: Database.Database,
  symbolId: number,
  type: string,
  description: string,
  file?: string,
  startLine?: number,
): void {
  db.prepare(
    `INSERT INTO dependency (symbol_id, type, description, file, start_line) VALUES (?, ?, ?, ?, ?)`,
  ).run(symbolId, type, description, file ?? null, startLine ?? null);
}

export function getDependenciesForSymbol(
  db: Database.Database,
  symbolId: number,
): { type: string; description: string; file: string | null; start_line: number | null }[] {
  return db
    .prepare("SELECT type, description, file, start_line FROM dependency WHERE symbol_id = ?")
    .all(symbolId) as { type: string; description: string; file: string | null; start_line: number | null }[];
}
