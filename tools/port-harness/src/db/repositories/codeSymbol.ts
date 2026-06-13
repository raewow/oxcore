import type Database from "better-sqlite3";
import type { CodeSymbol, SymbolKind } from "../../models/index.js";

export interface InsertSymbol {
  file: string;
  name: string;
  kind: SymbolKind;
  start_line: number;
  end_line: number;
  parent_symbol_id?: number | null;
  summary?: string | null;
}

export function upsertSymbol(db: Database.Database, sym: InsertSymbol): number {
  const existing = db
    .prepare("SELECT id FROM code_symbol WHERE file = ? AND name = ? AND start_line = ?")
    .get(sym.file, sym.name, sym.start_line) as { id: number } | undefined;

  if (existing) {
    db.prepare(
      `UPDATE code_symbol SET kind = ?, end_line = ?, parent_symbol_id = ?, summary = ?
       WHERE id = ?`,
    ).run(sym.kind, sym.end_line, sym.parent_symbol_id ?? null, sym.summary ?? null, existing.id);
    return existing.id;
  }

  const result = db
    .prepare(
      `INSERT INTO code_symbol (file, name, kind, start_line, end_line, parent_symbol_id, summary)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
    )
    .run(
      sym.file,
      sym.name,
      sym.kind,
      sym.start_line,
      sym.end_line,
      sym.parent_symbol_id ?? null,
      sym.summary ?? null,
    );
  return Number(result.lastInsertRowid);
}

export function getSymbolById(db: Database.Database, id: number): CodeSymbol | undefined {
  return db.prepare("SELECT * FROM code_symbol WHERE id = ?").get(id) as CodeSymbol | undefined;
}

export function getSymbolByName(
  db: Database.Database,
  file: string,
  name: string,
): CodeSymbol | undefined {
  return db
    .prepare("SELECT * FROM code_symbol WHERE file = ? AND name = ? ORDER BY start_line LIMIT 1")
    .get(file, name) as CodeSymbol | undefined;
}

export function listSymbolsByFile(db: Database.Database, file: string): CodeSymbol[] {
  const normalized = file.replace(/\\/g, "/");
  const base = normalized.includes("/") ? normalized.split("/").pop()! : normalized;
  return db
    .prepare(
      `SELECT * FROM code_symbol
       WHERE file = ? OR file = ? OR file LIKE ?
       ORDER BY start_line`,
    )
    .all(normalized, base, `%/${base}`) as CodeSymbol[];
}

export function listAllSymbols(db: Database.Database): CodeSymbol[] {
  return db.prepare("SELECT * FROM code_symbol ORDER BY file, start_line").all() as CodeSymbol[];
}

export function insertCall(
  db: Database.Database,
  callerId: number,
  calleeName: string,
  line: number,
  calleeFile?: string,
): void {
  db.prepare(
    `INSERT INTO symbol_call (caller_id, callee_name, callee_file, line) VALUES (?, ?, ?, ?)`,
  ).run(callerId, calleeName, calleeFile ?? null, line);
}

export function getCallsForSymbol(
  db: Database.Database,
  symbolId: number,
): { callee_name: string; line: number }[] {
  return db
    .prepare("SELECT callee_name, line FROM symbol_call WHERE caller_id = ?")
    .all(symbolId) as { callee_name: string; line: number }[];
}

export interface CallGraphNeighbor {
  symbol_id: number;
  symbol: string;
  file: string;
  relation: "caller" | "callee";
  related_symbol: string;
  line?: number;
}

export function getCallersForCallee(
  db: Database.Database,
  calleeName: string,
): { caller_id: number; line: number }[] {
  return db
    .prepare(
      `SELECT sc.caller_id, sc.line
       FROM symbol_call sc
       WHERE sc.callee_name = ? OR sc.callee_name LIKE ?`,
    )
    .all(calleeName, `%${calleeName.split("::").pop()}`) as { caller_id: number; line: number }[];
}

export function getCallGraphNeighbors(
  db: Database.Database,
  symbolIds: number[],
  limit = 20,
): CallGraphNeighbor[] {
  const seen = new Set<number>();
  const neighbors: CallGraphNeighbor[] = [];

  for (const symbolId of symbolIds) {
    if (seen.has(symbolId)) continue;
    seen.add(symbolId);

    const symbol = getSymbolById(db, symbolId);
    if (!symbol) continue;

    for (const call of getCallsForSymbol(db, symbolId)) {
      const callee = db
        .prepare("SELECT id, name, file FROM code_symbol WHERE name = ? LIMIT 1")
        .get(call.callee_name) as { id: number; name: string; file: string } | undefined;
      if (callee && !seen.has(callee.id)) {
        neighbors.push({
          symbol_id: callee.id,
          symbol: callee.name,
          file: callee.file,
          relation: "callee",
          related_symbol: symbol.name,
          line: call.line,
        });
      }
    }

    for (const { caller_id, line } of getCallersForCallee(db, symbol.name)) {
      if (seen.has(caller_id)) continue;
      const caller = getSymbolById(db, caller_id);
      if (caller) {
        neighbors.push({
          symbol_id: caller.id,
          symbol: caller.name,
          file: caller.file,
          relation: "caller",
          related_symbol: symbol.name,
          line,
        });
      }
    }

    if (neighbors.length >= limit) break;
  }

  return neighbors.slice(0, limit);
}

export function deleteSymbolsByFile(db: Database.Database, file: string): void {
  const normalized = file.replace(/\\/g, "/");
  const base = normalized.includes("/") ? normalized.split("/").pop()! : normalized;
  db.prepare(
    `DELETE FROM code_symbol WHERE file = ? OR file = ? OR file LIKE ?`,
  ).run(normalized, base, `%/${base}`);
}
