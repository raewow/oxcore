import type Database from "better-sqlite3";
import { basename, resolve } from "node:path";
import type { HarnessConfig } from "../config.js";
import { normalizeRelPath, resolveSourceFilePath } from "../files/paths.js";
import {
  extractMethodsFromCpp,
  extractCallsFromRange,
  extractIncludes,
} from "./parser.js";
import { chunkLargeMethod, shouldChunk } from "./chunker.js";
import * as codeSymbolRepo from "../db/repositories/codeSymbol.js";
import * as migrationTaskRepo from "../db/repositories/migrationTask.js";
import * as flowRepo from "../db/repositories/businessFlow.js";

export interface IndexResult {
  symbolsIndexed: number;
  tasksCreated: number;
  callsExtracted: number;
  chunksCreated: number;
}

function inferClassName(fileName: string): string {
  const base = fileName.replace(/\.(cpp|h)$/, "");
  return base.charAt(0).toUpperCase() + base.slice(1);
}

async function indexCppFile(
  db: Database.Database,
  config: HarnessConfig,
  file: string,
): Promise<{ symbolsIndexed: number; tasksCreated: number; callsExtracted: number; chunksCreated: number }> {
  let symbolsIndexed = 0;
  let tasksCreated = 0;
  let callsExtracted = 0;
  let chunksCreated = 0;

  const fullPath = resolve(config.referenceRoot, file);
  const fileName = basename(fullPath);
  const relPath = normalizeRelPath(file);
  const className = inferClassName(fileName);

  codeSymbolRepo.deleteSymbolsByFile(db, relPath);

  const methods = extractMethodsFromCpp(fullPath, className, config.index.excludePatterns);

  for (const method of methods) {
    const symbolId = codeSymbolRepo.upsertSymbol(db, {
      file: relPath,
      name: method.name,
      kind: "method",
      start_line: method.startLine,
      end_line: method.endLine,
    });
    symbolsIndexed++;

    migrationTaskRepo.upsertTask(db, symbolId, { status: "discovered" });
    tasksCreated++;

    const calls = extractCallsFromRange(fullPath, method.startLine, method.endLine);
    for (const call of calls) {
      codeSymbolRepo.insertCall(db, symbolId, call.calleeName, call.line);
      callsExtracted++;
    }

    const symbol = {
      ...method,
      file: relPath,
      startLine: method.startLine,
      endLine: method.endLine,
    };
    if (shouldChunk(symbol, config.index.maxChunkLines)) {
      const chunks = chunkLargeMethod(symbol, config.index.maxChunkLines);
      for (const chunk of chunks) {
        const chunkId = codeSymbolRepo.upsertSymbol(db, {
          file: chunk.file,
          name: chunk.name,
          kind: "chunk",
          start_line: chunk.startLine,
          end_line: chunk.endLine,
          parent_symbol_id: symbolId,
        });
        chunksCreated++;
        migrationTaskRepo.upsertTask(db, chunkId, { status: "discovered" });
        tasksCreated++;
      }
    }
  }

  return { symbolsIndexed, tasksCreated, callsExtracted, chunksCreated };
}

export async function indexFiles(
  db: Database.Database,
  config: HarnessConfig,
  files: string[],
): Promise<IndexResult> {
  let symbolsIndexed = 0;
  let tasksCreated = 0;
  let callsExtracted = 0;
  let chunksCreated = 0;

  for (const file of files) {
    const fullPath = resolve(config.referenceRoot, file);
    const fileName = basename(fullPath);

    if (fileName.endsWith(".cpp")) {
      const result = await indexCppFile(db, config, file);
      symbolsIndexed += result.symbolsIndexed;
      tasksCreated += result.tasksCreated;
      callsExtracted += result.callsExtracted;
      chunksCreated += result.chunksCreated;
    } else if (fileName.endsWith(".h")) {
      const includes = extractIncludes(fullPath);
      void includes;
    }
  }

  return { symbolsIndexed, tasksCreated, callsExtracted, chunksCreated };
}

export function applyFlowMappings(
  db: Database.Database,
  mappings: Record<string, { flow: string; rustTarget: string }>,
): number {
  let applied = 0;
  const symbols = codeSymbolRepo.listAllSymbols(db);
  for (const sym of symbols) {
    const mapping = mappings[sym.name];
    if (!mapping) continue;

    const flow = flowRepo.getFlowByName(db, mapping.flow) as { id: number } | undefined;
    migrationTaskRepo.upsertTask(db, sym.id, {
      target_rust_file: mapping.rustTarget,
      rust_symbol_name: sym.name.split("::").pop(),
      flow_id: flow?.id ?? null,
    });
    applied++;
  }
  return applied;
}
