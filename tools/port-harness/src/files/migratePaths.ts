import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import { resolveFullPathForStored } from "../files/paths.js";
import * as codeSymbolRepo from "../db/repositories/codeSymbol.js";

/** Fix code_symbol.file rows that only store a basename (legacy index). */
export function migrateStoredFilePaths(
  db: Database.Database,
  config: HarnessConfig,
): number {
  let updated = 0;
  const symbols = codeSymbolRepo.listAllSymbols(db);

  for (const sym of symbols) {
    if (sym.file.includes("/")) continue;

    const fullPath = resolveFullPathForStored(config.referenceRoot, sym.file, {
      symbolName: sym.name,
    });

    if (fullPath !== sym.file && fullPath.includes("/")) {
      db.prepare("UPDATE code_symbol SET file = ? WHERE id = ?").run(fullPath, sym.id);
      updated++;
    }
  }

  return updated;
}
