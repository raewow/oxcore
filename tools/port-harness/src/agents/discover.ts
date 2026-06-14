import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import { readPrompt, fillPrompt } from "../config.js";
import type { AgentProvider } from "./provider.js";
import { hashPrompt } from "./provider.js";
import { DiscoverOutputSchema, type DiscoverOutput } from "../models/index.js";
import {
  searchHarness,
  searchReferenceFiles,
  getMatchedFileStats,
  type DiscoverSeedHit,
} from "../discover/search.js";
import { getCallGraphNeighbors } from "../db/repositories/codeSymbol.js";
import * as investigationRepo from "../db/repositories/investigation.js";
import * as codeSymbolRepo from "../db/repositories/codeSymbol.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import { filePathMatches } from "../files/paths.js";
import type { JobActivity } from "../server/jobActivity.js";

function enrichCandidates(
  db: Database.Database,
  output: DiscoverOutput,
): DiscoverOutput {
  const candidates = output.candidates.map((c) => {
    const fileBase = c.file.split("/").pop() ?? c.file;
    let symbol = codeSymbolRepo.getSymbolByName(db, c.file, c.symbol);
    if (!symbol) {
      const matches = codeSymbolRepo.listSymbolsByFile(db, fileBase);
      symbol = matches.find((s) => s.name === c.symbol);
    }
    if (!symbol) {
      return { ...c, in_index: false };
    }

    const task = taskRepo.getTaskBySymbolId(db, symbol.id);
    return {
      ...c,
      in_index: true,
      task_id: task?.id ?? c.task_id,
      task_status: task?.status ?? c.task_status,
    };
  });

  return { ...output, candidates };
}

function boostByFeatureFiles<T extends { file?: string; path?: string }>(
  items: T[],
  featureFiles: string[],
): T[] {
  if (!featureFiles.length) return items;
  const inFeature = items.filter((item) => {
    const file = (item as { file?: string }).file ?? (item as { path?: string }).path ?? "";
    return featureFiles.some((f) => filePathMatches(file, f));
  });
  const rest = items.filter((item) => {
    const file = (item as { file?: string }).file ?? (item as { path?: string }).path ?? "";
    return !featureFiles.some((f) => filePathMatches(file, f));
  });
  return [...inFeature, ...rest];
}

export async function runDiscover(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  query: string,
  investigationId: number,
  activity?: JobActivity,
  featureFiles?: string[],
): Promise<{ success: boolean; error?: string; output?: DiscoverOutput }> {
  activity?.setCurrent("discover");
  activity?.log(`Investigating: ${query.slice(0, 80)}${query.length > 80 ? "…" : ""}`);

  const rawSeedHits = searchHarness(db, query);
  const seedHits = boostByFeatureFiles(rawSeedHits, featureFiles ?? []);
  const topSymbolIds = seedHits.slice(0, 15).map((h) => h.symbol_id);
  const callGraphNeighbors = getCallGraphNeighbors(db, topSymbolIds, 20);
  const fileStats = getMatchedFileStats(db, seedHits);
  const rawReferenceFileHits = searchReferenceFiles(config.referenceRoot, query, 80);
  const referenceFileHits = boostByFeatureFiles(rawReferenceFileHits, featureFiles ?? []);

  activity?.log(
    `DB seed: ${seedHits.length} hit(s), ${callGraphNeighbors.length} call-graph neighbor(s), ${referenceFileHits.length} file hit(s)`,
  );

  const seedPayload = {
    hits: seedHits,
    callGraphNeighbors,
    fileStats,
    referenceFileHits,
  };

  db.prepare("UPDATE investigation SET seed_json = ? WHERE id = ?").run(
    JSON.stringify(seedPayload),
    investigationId,
  );

  const featureContext =
    featureFiles?.length
      ? `\n## Feature Focus\nThis investigation is scoped to the following feature files (prioritise these over global hits):\n${featureFiles.join("\n")}\n`
      : "";

  const prompt = fillPrompt(readPrompt("discover_symptom"), {
    query,
    featureContext,
    seedHits: JSON.stringify(seedHits.slice(0, 30), null, 2),
    callGraphNeighbors: JSON.stringify(callGraphNeighbors, null, 2),
    fileStats: JSON.stringify(fileStats, null, 2),
    referenceFileHits: JSON.stringify(referenceFileHits, null, 2),
    referenceRoot: config.referenceRoot,
    rustRoot: config.rustRoot,
  });

  const system =
    "You are a migration investigator helping port a C++ game server to Rust. Find all code that may explain a reported bug or missing behavior. Search the codebase thoroughly.";

  activity?.log("Waiting for Cursor agent…");

  try {
    const rawOutput = await provider.completeStructured(
      system,
      prompt,
      DiscoverOutputSchema,
      { stage: "discover" },
    );

    const output = enrichCandidates(db, rawOutput);

    jobsRepo.insertAgentRun(
      db,
      "discover",
      provider.name,
      config.provider.model,
      hashPrompt(system, prompt),
      JSON.stringify(output),
    );

    investigationRepo.finishInvestigation(db, investigationId, "done", JSON.stringify(output));
    activity?.log(`Found ${output.candidates.length} candidate(s), ${output.unindexed_files.length} unindexed file(s)`);

    return { success: true, output };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    investigationRepo.finishInvestigation(db, investigationId, "failed");
    return { success: false, error: message };
  }
}

export function buildSeedForQuery(
  db: Database.Database,
  query: string,
  referenceRoot?: string,
  featureFiles?: string[],
): {
  hits: DiscoverSeedHit[];
  callGraphNeighbors: ReturnType<typeof getCallGraphNeighbors>;
  fileStats: ReturnType<typeof getMatchedFileStats>;
  referenceFileHits: ReturnType<typeof searchReferenceFiles>;
} {
  const rawHits = searchHarness(db, query);
  const hits = boostByFeatureFiles(rawHits, featureFiles ?? []);
  const callGraphNeighbors = getCallGraphNeighbors(
    db,
    hits.slice(0, 15).map((h) => h.symbol_id),
    20,
  );
  const fileStats = getMatchedFileStats(db, hits);
  const rawRefHits = referenceRoot ? searchReferenceFiles(referenceRoot, query) : [];
  const referenceFileHits = boostByFeatureFiles(rawRefHits, featureFiles ?? []);
  return { hits, callGraphNeighbors, fileStats, referenceFileHits };
}
