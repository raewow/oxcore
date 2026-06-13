import { Hono } from "hono";
import type Database from "better-sqlite3";
import type { HarnessConfig } from "../../config.js";
import * as codeSymbolRepo from "../../db/repositories/codeSymbol.js";
import { resolveSourceFilePath } from "../../files/paths.js";
import { getSourceSnippet } from "../../index/parser.js";

export function createSymbolsRoutes(db: Database.Database, config: HarnessConfig): Hono {
  const app = new Hono();

  app.get("/:id/source", (c) => {
    const id = parseInt(c.req.param("id"), 10);
    const symbol = codeSymbolRepo.getSymbolById(db, id);
    if (!symbol) return c.json({ error: "Not found" }, 404);

    try {
      const fullPath = resolveSourceFilePath(config.referenceRoot, symbol.file, {
        symbolName: symbol.name,
      });
      const snippet = getSourceSnippet(fullPath, symbol.start_line, symbol.end_line);

      return c.json({
        symbol,
        snippet,
        start_line: symbol.start_line,
        end_line: symbol.end_line,
        resolved_path: fullPath,
      });
    } catch (err) {
      return c.json(
        { error: err instanceof Error ? err.message : String(err) },
        404,
      );
    }
  });

  return app;
}
