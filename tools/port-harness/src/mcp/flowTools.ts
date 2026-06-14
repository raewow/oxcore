import type Database from "better-sqlite3";
import type { HarnessConfig } from "../config.js";
import * as flowRepo from "../db/repositories/businessFlow.js";
import * as flowProgressRepo from "../db/repositories/flowProgress.js";
import { getLatestAuditsForTasks, recommendNextAction, summarizeFlowAudits } from "../db/repositories/flowAudits.js";
import { getLatestAgentRunsForTasks, parsePlanRun, parsePortRun, planDocRelPath, readPlanDocFile, portDraftRelPath, auditDocRelPath, readAuditDocFile } from "../db/repositories/flowArtifacts.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as claimRepo from "../db/repositories/behaviourClaim.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import * as symbolRepo from "../db/repositories/codeSymbol.js";
import { resolveSourceFilePath } from "../files/paths.js";
import { getSourceSnippet } from "../index/parser.js";
import type { RiskLevel } from "../models/index.js";

type FlowRecord = {
  id: number;
  name: string;
  description: string | null;
  source_file: string | null;
  risk_level: string;
  entry_symbol_ids: string | null;
  expected_behaviour: string | null;
};

export type FlowBranchInput = {
  condition: string;
  behaviour: string;
  file: string;
  start_line: number;
  end_line: number;
};

export type FlowMutationInput = {
  variable_or_field: string;
  mutation_description: string;
  file: string;
  start_line: number;
  end_line: number;
};

export type FlowDraftInput = {
  name: string;
  description: string;
  entry_symbols: string[];
  expected_behaviour: string;
  risk_level: RiskLevel;
  source_file?: string;
  replace_data?: boolean;
  branches?: FlowBranchInput[];
  mutations?: FlowMutationInput[];
};

export type FlowUpdateInput = Partial<Omit<FlowDraftInput, "entry_symbols">> & {
  entry_symbols?: string[];
};

export function resolveFlow(db: Database.Database, ref: string): FlowRecord | undefined {
  const byName = flowRepo.getFlowByName(db, ref) as FlowRecord | undefined;
  if (byName) return byName;
  if (/^\d+$/.test(ref)) return flowRepo.getFlowById(db, Number(ref)) as FlowRecord | undefined;
  return (flowRepo.listFlows(db) as FlowRecord[]).find((flow) => flow.name.toLowerCase() === ref.toLowerCase());
}

function getFlowJobsForTasks(db: Database.Database, taskIds: number[], stages: string[], limit = 20) {
  const idSet = new Set(taskIds);
  const stageSet = new Set(stages);
  const jobs = jobsRepo.listJobs(db, 50) as {
    id: number;
    stage: string;
    target_ids: string;
    status: string;
    progress: number;
    total: number;
    error: string | null;
    created_at: string;
  }[];

  return jobs
    .filter((job) => {
      if (!stageSet.has(job.stage)) return false;
      try {
        const ids = JSON.parse(job.target_ids) as number[];
        return Array.isArray(ids) && ids.some((id) => idSet.has(id));
      } catch {
        return false;
      }
    })
    .slice(0, limit)
    .map((job) => jobsRepo.enrichJob(db, job as never, { targetLimit: 3 }));
}

export function listFlowsForMcp(db: Database.Database) {
  return flowProgressRepo.listFlowsWithProgress(db);
}

function parseEntrySymbolIds(entrySymbolIds: string | null): number[] {
  if (!entrySymbolIds) return [];
  try {
    const parsed = JSON.parse(entrySymbolIds) as unknown;
    return Array.isArray(parsed) ? parsed.filter((id): id is number => Number.isInteger(id)) : [];
  } catch {
    return [];
  }
}

function syncFlowTaskLinks(db: Database.Database, flowId: number, entryIds: number[]) {
  const desired = new Set(entryIds);
  const currentTasks = db
    .prepare("SELECT id, source_symbol_id FROM migration_task WHERE flow_id = ?")
    .all(flowId) as { id: number; source_symbol_id: number }[];

  for (const task of currentTasks) {
    if (!desired.has(task.source_symbol_id)) {
      db.prepare("UPDATE migration_task SET flow_id = NULL, updated_at = datetime('now') WHERE id = ?").run(task.id);
    }
  }

  for (const entryId of entryIds) {
    taskRepo.upsertTask(db, entryId, { flow_id: flowId });
  }
}

function replaceFlowDetails(db: Database.Database, flowId: number, branches?: FlowBranchInput[], mutations?: FlowMutationInput[]) {
  if (branches === undefined && mutations === undefined) return;
  flowRepo.deleteFlowData(db, flowId);

  for (const branch of branches ?? []) {
    flowRepo.insertBranch(db, flowId, branch.condition, branch.behaviour, branch.file, branch.start_line, branch.end_line);
  }

  for (const mutation of mutations ?? []) {
    flowRepo.insertMutation(
      db,
      flowId,
      mutation.variable_or_field,
      mutation.mutation_description,
      mutation.file,
      mutation.start_line,
      mutation.end_line,
    );
  }
}

function persistFlowDraft(
  db: Database.Database,
  flow: Omit<FlowDraftInput, "entry_symbols">,
  entrySymbols: string[],
  branches?: FlowBranchInput[],
  mutations?: FlowMutationInput[],
  entryIdsOverride?: number[],
  existingFlowId?: number,
) {
  const scopedSymbols = flow.source_file
    ? symbolRepo.listAllSymbols(db).filter((sym) => sym.file === flow.source_file)
    : symbolRepo.listAllSymbols(db);
  const exactSymbols = new Map(scopedSymbols.map((sym) => [sym.name, sym] as const));
  const shortSymbols = new Map<string, typeof scopedSymbols>();
  for (const sym of scopedSymbols) {
    const shortName = sym.name.split("::").pop() ?? sym.name;
    const bucket = shortSymbols.get(shortName);
    if (bucket) bucket.push(sym);
    else shortSymbols.set(shortName, [sym]);
  }

  const entryIds: number[] = [];
  const unresolved: string[] = [];
  if (entryIdsOverride) {
    entryIds.push(...entryIdsOverride);
  } else {
    for (const entry of entrySymbols) {
      const sym = exactSymbols.get(entry) ?? shortSymbols.get(entry)?.[0];
      if (sym && !entryIds.includes(sym.id)) entryIds.push(sym.id);
      else if (!sym) unresolved.push(entry);
    }
  }

  let flowId: number;
  if (existingFlowId !== undefined) {
    const collision = db
      .prepare("SELECT id FROM business_flow WHERE name = ? AND id != ?")
      .get(flow.name, existingFlowId) as { id: number } | undefined;
    if (collision) {
      return { ok: false, error: `Flow name \"${flow.name}\" already exists` };
    }

    db.prepare(
      `UPDATE business_flow SET name = ?, description = ?, entry_symbol_ids = ?,
       expected_behaviour = ?, risk_level = ?, source_file = ? WHERE id = ?`,
    ).run(
      flow.name,
      flow.description,
      JSON.stringify(entryIds),
      flow.expected_behaviour,
      flow.risk_level,
      flow.source_file ?? null,
      existingFlowId,
    );
    flowId = existingFlowId;
  } else {
    flowId = flowRepo.upsertFlow(db, {
      name: flow.name,
      description: flow.description,
      entry_symbol_ids: entryIds,
      expected_behaviour: flow.expected_behaviour,
      risk_level: flow.risk_level,
      source_file: flow.source_file,
    });
  }

  syncFlowTaskLinks(db, flowId, entryIds);

  if (flow.replace_data !== false) {
    replaceFlowDetails(db, flowId, branches, mutations);
  }

  return { ok: true, name: flow.name, flow_id: flowId, entry_symbol_ids: entryIds, unresolved_symbols: unresolved };
}

export function saveFlows(db: Database.Database, flows: FlowDraftInput[]) {
  return {
    ok: true,
    flows: flows.map((flow) =>
      persistFlowDraft(db, flow, flow.entry_symbols, flow.branches, flow.mutations),
    ),
  };
}

export function createFlow(db: Database.Database, flow: FlowDraftInput) {
  if (resolveFlow(db, flow.name)) {
    return { ok: false, error: `Flow "${flow.name}" already exists` };
  }

  return persistFlowDraft(db, flow, flow.entry_symbols, flow.branches, flow.mutations);
}

export function updateFlow(db: Database.Database, ref: string, patch: FlowUpdateInput) {
  const existing = resolveFlow(db, ref);
  if (!existing) {
    return { ok: false, error: `No flow "${ref}"` };
  }

  const previousEntryIds = parseEntrySymbolIds(existing.entry_symbol_ids);
  const nextEntrySymbols = patch.entry_symbols;
  const scopedSymbols = patch.source_file
    ? symbolRepo.listAllSymbols(db).filter((sym) => sym.file === patch.source_file)
    : symbolRepo.listAllSymbols(db);
  const exactSymbols = new Map(scopedSymbols.map((sym) => [sym.name, sym] as const));
  const shortSymbols = new Map<string, typeof scopedSymbols>();
  for (const sym of scopedSymbols) {
    const shortName = sym.name.split("::").pop() ?? sym.name;
    const bucket = shortSymbols.get(shortName);
    if (bucket) bucket.push(sym);
    else shortSymbols.set(shortName, [sym]);
  }

  const entryIds: number[] = [];
  const unresolved: string[] = [];
  if (nextEntrySymbols) {
    for (const entry of nextEntrySymbols) {
      const sym = exactSymbols.get(entry) ?? shortSymbols.get(entry)?.[0];
      if (sym && !entryIds.includes(sym.id)) entryIds.push(sym.id);
      else if (!sym) unresolved.push(entry);
    }
  } else {
    entryIds.push(...previousEntryIds);
  }

  const merged: Omit<FlowDraftInput, "entry_symbols"> = {
    name: patch.name ?? existing.name,
    description: patch.description ?? existing.description ?? "",
    expected_behaviour: patch.expected_behaviour ?? existing.expected_behaviour ?? "",
    risk_level: patch.risk_level ?? (existing.risk_level as RiskLevel),
    source_file: patch.source_file ?? existing.source_file ?? undefined,
    replace_data: patch.replace_data ?? (patch.branches !== undefined || patch.mutations !== undefined),
  };

  const result = persistFlowDraft(
    db,
    merged,
    nextEntrySymbols ?? [],
    patch.branches,
    patch.mutations,
    entryIds,
    existing.id,
  ) as { ok: boolean; error?: string; name?: string; flow_id?: number; entry_symbol_ids?: number[]; unresolved_symbols?: string[] };
  if (!result.ok) return result;
  return { updated: true, ...result, unresolved_symbols: unresolved.length ? unresolved : result.unresolved_symbols ?? [] };
}

export function getFlowDetailsForMcp(
  db: Database.Database,
  config: Pick<HarnessConfig, "referenceRoot"> | null | undefined,
  ref: string,
) {
  const flow = resolveFlow(db, ref);
  if (!flow) return null;

  const branches = flowRepo.getBranchesForFlow(db, flow.id);
  const mutations = flowRepo.getMutationsForFlow(db, flow.id);
  const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 1000 });
  const flowTasks = tasks.filter((t) => t.flow_id === flow.id);
  const taskIds = flowTasks.map((t) => t.id);
  const auditsByTask = getLatestAuditsForTasks(db, taskIds);
  const plansByTask = getLatestAgentRunsForTasks(db, taskIds, "plan-rust");
  const portsByTask = getLatestAgentRunsForTasks(db, taskIds, "port");
  const statusByTask = new Map(flowTasks.map((t) => [t.id, t.status]));
  const progress = flowProgressRepo.summarizeFlowProgress(taskIds, auditsByTask, statusByTask, plansByTask, portsByTask);
  const auditSummary = summarizeFlowAudits(taskIds, auditsByTask, statusByTask);

  const tasksWithContext = flowTasks.map((t) => {
    let sourceSnippet: { start_line: number; end_line: number; text: string } | null = null;
    if (config?.referenceRoot) {
      try {
        const sourcePath = resolveSourceFilePath(config.referenceRoot, t.symbol_file, { symbolName: t.symbol_name });
        sourceSnippet = {
          start_line: t.start_line,
          end_line: t.end_line,
          text: getSourceSnippet(sourcePath, t.start_line, t.end_line),
        };
      } catch {
        sourceSnippet = null;
      }
    }

    return {
      ...t,
      audit: auditsByTask.get(t.id) ?? null,
      plan: parsePlanRun(plansByTask.get(t.id), {
        target_rust_file: t.target_rust_file,
        rust_symbol_name: t.rust_symbol_name,
        notes: t.notes,
        status: t.status,
      }),
      port_draft: parsePortRun(portsByTask.get(t.id), t.symbol_name),
      next_action: recommendNextAction(auditsByTask.get(t.id), t.status),
      claims: claimRepo.getClaimsForSymbol(db, t.source_symbol_id),
      dependencies: claimRepo.getDependenciesForSymbol(db, t.source_symbol_id),
      source_snippet: sourceSnippet,
      generated_docs: {
        audit_path: auditDocRelPath(t.symbol_name),
        audit_markdown: readAuditDocFile(t.symbol_name),
        plan_path: planDocRelPath(t.symbol_name),
        plan_markdown: readPlanDocFile(t.symbol_name),
        port_path: portDraftRelPath(t.symbol_name),
        port_code: parsePortRun(portsByTask.get(t.id), t.symbol_name)?.rust_code ?? null,
      },
    };
  });

  return {
    flow,
    branches,
    mutations,
    progress,
    audit_summary: auditSummary,
    tasks: tasksWithContext,
    pipeline_jobs: getFlowJobsForTasks(db, taskIds, ["audit-rust", "plan-rust", "port"]),
  };
}
