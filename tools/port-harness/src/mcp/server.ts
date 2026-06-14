#!/usr/bin/env node
/**
 * Port-harness MCP server (read + status writes).
 *
 * Exposes the symbol index, call graph, behaviour claims, flow assembly, and feature
 * tracking in `port_harness.db` to any MCP client (e.g. Claude Code). Most tools are
 * read-only; writes are limited to task status and flow persistence helpers.
 *
 * Run:  npm run mcp      (tsx src/mcp/server.ts)
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import type Database from "better-sqlite3";
import { getDb } from "../db/client.js";
import { TaskStatus } from "../models/index.js";
import * as symbolRepo from "../db/repositories/codeSymbol.js";
import * as claimRepo from "../db/repositories/behaviourClaim.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as featureRepo from "../db/repositories/features.js";
import { getFeatureCoverage } from "../db/repositories/coverage.js";
import { flowDraftSchema, flowSaveSchema, flowUpdateSchema } from "./flowSchemas.js";
import { createFlow, getFlowDetailsForMcp, listFlowsForMcp, saveFlows, updateFlow } from "./flowTools.js";

const db = getDb();

function json(value: unknown) {
  return { content: [{ type: "text" as const, text: JSON.stringify(value, null, 2) }] };
}

function err(message: string) {
  return { isError: true, content: [{ type: "text" as const, text: message }] };
}

function resolveFeature(db: Database.Database, ref: string) {
  const byName = featureRepo.getFeatureByName(db, ref);
  if (byName) return byName;
  if (/^\d+$/.test(ref)) return featureRepo.getFeature(db, Number(ref));
  // case-insensitive fallback
  return featureRepo
    .listFeatures(db)
    .find((f) => f.name.toLowerCase() === ref.toLowerCase());
}

/** Find indexed symbols by (partial, case-insensitive) name. */
function searchSymbols(db: Database.Database, query: string, limit: number) {
  return db
    .prepare(
      `SELECT cs.id, cs.name, cs.file, cs.kind, cs.start_line, cs.end_line, cs.summary,
              mt.status AS task_status,
              (SELECT COUNT(*) FROM behaviour_claim bc WHERE bc.symbol_id = cs.id) AS claim_count
       FROM code_symbol cs
       LEFT JOIN migration_task mt ON mt.source_symbol_id = cs.id
       WHERE cs.name LIKE ? AND cs.kind != 'chunk'
       ORDER BY (cs.name = ?) DESC, length(cs.name) ASC
       LIMIT ?`,
    )
    .all(`%${query}%`, query, limit) as Record<string, unknown>[];
}

const server = new McpServer({ name: "port-harness", version: "0.1.0" });

server.registerTool(
  "find_symbol",
  {
    title: "Find symbol",
    description:
      "Search the indexed C++ symbol tree by name (partial match). Returns file, line range, summary, current porting status, and claim count for each match.",
    inputSchema: { query: z.string().describe("Symbol name or fragment, e.g. 'Spell::cast' or 'FillTargetMap'"), limit: z.number().optional() },
  },
  async ({ query, limit }) => json(searchSymbols(db, query, limit ?? 20)),
);

server.registerTool(
  "symbol_callees",
  {
    title: "Symbol callees",
    description:
      "List everything a symbol calls (outgoing call-graph edges). Each callee is marked indexed:true if it resolves to a known symbol, so you can see which details are already mapped vs external/unindexed.",
    inputSchema: { name: z.string().describe("Qualified symbol name, e.g. 'Spell::cast'") },
  },
  async ({ name }) => {
    const sym = symbolRepo
      .listAllSymbols(db)
      .find((s) => s.name === name) ??
      searchSymbols(db, name, 1)[0];
    if (!sym) return err(`No symbol matching "${name}"`);
    const calls = symbolRepo.getCallsForSymbol(db, sym.id as number);
    const indexedShort = new Set(
      symbolRepo.listAllSymbols(db).map((s) => s.name.split("::").pop()!),
    );
    const callees = calls.map((c) => ({
      callee: c.callee_name,
      line: c.line,
      indexed: indexedShort.has(c.callee_name.split("::").pop()!),
    }));
    return json({ symbol: (sym as { name: string }).name, callee_count: callees.length, callees });
  },
);

server.registerTool(
  "symbol_callers",
  {
    title: "Symbol callers",
    description: "List every indexed symbol that calls the given symbol (incoming call-graph edges). Use to gauge blast radius before porting.",
    inputSchema: { name: z.string().describe("Symbol name (qualified or short)") },
  },
  async ({ name }) => {
    const callers = symbolRepo.getCallersForCallee(db, name);
    const out = callers
      .map((c) => {
        const sym = symbolRepo.getSymbolById(db, c.caller_id);
        return sym ? { caller: sym.name, file: sym.file, line: c.line } : null;
      })
      .filter(Boolean);
    return json({ callee: name, caller_count: out.length, callers: out });
  },
);

server.registerTool(
  "behaviour_claims",
  {
    title: "Behaviour claims",
    description:
      "The extracted behaviour spec for a symbol: inputs, outputs, branches, side effects, assumptions, dangers — plus external dependencies. This is the 'logic to preserve' when porting.",
    inputSchema: { name: z.string().describe("Qualified symbol name, e.g. 'Spell::cast'") },
  },
  async ({ name }) => {
    const sym =
      symbolRepo.listAllSymbols(db).find((s) => s.name === name) ?? searchSymbols(db, name, 1)[0];
    if (!sym) return err(`No symbol matching "${name}"`);
    const id = sym.id as number;
    const claims = claimRepo.getClaimsForSymbol(db, id);
    const byCategory: Record<string, string[]> = {};
    for (const c of claims) (byCategory[c.category] ??= []).push(c.claim_text);
    const deps = db
      .prepare("SELECT type, description FROM dependency WHERE symbol_id = ?")
      .all(id) as { type: string; description: string }[];
    return json({
      symbol: (sym as { name: string }).name,
      file: (sym as { file: string }).file,
      lines: `${(sym as { start_line: number }).start_line}-${(sym as { end_line: number }).end_line}`,
      claim_count: claims.length,
      claims: byCategory,
      dependencies: deps,
    });
  },
);

server.registerTool(
  "feature_coverage",
  {
    title: "Feature coverage",
    description:
      "Is every detail of a feature mapped? Walks the call graph from the feature's symbols and reports mapped %, indexed symbols that should be pulled into the feature, and the top unindexed callees (the triage worklist of possible gaps).",
    inputSchema: { feature: z.string().describe("Feature name or id, e.g. 'spells'") },
  },
  async ({ feature }) => {
    const f = resolveFeature(db, feature);
    if (!f) return err(`No feature "${feature}"`);
    return json(getFeatureCoverage(db, f.id, f.name));
  },
);

server.registerTool(
  "feature_status",
  {
    title: "Feature status",
    description:
      "Porting progress for a feature: task-status breakdown and the symbols that still block 'done' (anything not yet verified), so you know what confidence you actually have.",
    inputSchema: { feature: z.string().describe("Feature name or id") },
  },
  async ({ feature }) => {
    const f = resolveFeature(db, feature);
    if (!f) return err(`No feature "${feature}"`);
    const tasks = featureRepo.getFeatureTasks(db, f.id);
    const stats = featureRepo.getFeatureTaskStats(db, f.id);
    const incomplete = tasks
      .filter((t) => !["verified", "reviewed", "done"].includes(t.status))
      .map((t) => ({ symbol: t.symbol_name, file: t.symbol_file, status: t.status }));
    return json({
      feature: f.name,
      total_tasks: stats.total,
      by_status: stats.by_status,
      blocking_count: incomplete.length,
      blocking: incomplete.slice(0, 100),
    });
  },
);

server.registerTool(
  "list_flows",
  {
    title: "List flows",
    description:
      "List all business flows with progress, stage, and symbol counts. Use this after documentation to choose what to plan and port next.",
    inputSchema: {},
  },
  async () => json(listFlowsForMcp(db)),
);

server.registerTool(
  "save_flows",
  {
    title: "Save flows",
    description:
      "Upsert one or more business flows, replace their branches/mutations, and relink entry tasks. Use this to persist assembled/generated flows into the harness database.",
    inputSchema: flowSaveSchema,
  },
  async ({ flows }) => json(saveFlows(db, flows)),
);

server.registerTool(
  "create_flow",
  {
    title: "Create flow",
    description:
      "Create a single business flow from assembled output. Use when Claude has produced a new flow that does not yet exist in the harness.",
    inputSchema: { flow: flowDraftSchema },
  },
  async ({ flow }) => {
    const result = createFlow(db, flow);
    if (!result.ok) return err(result.error ?? "Failed to create flow");
    return json(result);
  },
);

server.registerTool(
  "update_flow",
  {
    title: "Update flow",
    description:
      "Update an existing flow by id or name. Use this for iterative assembly when the flow already exists and only some fields need to change.",
    inputSchema: flowUpdateSchema,
  },
  async ({ flow, ...patch }) => {
    const result = updateFlow(db, flow, patch);
    if (!result.ok) return err(result.error ?? "Failed to update flow");
    return json(result);
  },
);

server.registerTool(
  "flow_details",
  {
    title: "Flow details",
    description:
      "Get the full flow record plus branches, mutations, linked tasks, audit state, and plan/port drafts. This is the bridge from documentation to planning and porting.",
    inputSchema: { flow: z.string().describe("Flow name or id") },
  },
  async ({ flow }) => {
    const details = getFlowDetailsForMcp(db, null, flow);
    if (!details) return err(`No flow "${flow}"`);
    return json(details);
  },
);

server.registerTool(
  "next_tasks",
  {
    title: "Next tasks",
    description:
      "The next symbols to work on for a feature, ordered by how early they are in the porting ladder (discovered first). Optionally filter to symbols at a specific status.",
    inputSchema: {
      feature: z.string().describe("Feature name or id"),
      status: TaskStatus.optional().describe("Only return tasks at this status"),
      limit: z.number().optional(),
    },
  },
  async ({ feature, status, limit }) => {
    const f = resolveFeature(db, feature);
    if (!f) return err(`No feature "${feature}"`);
    const order = [
      "discovered",
      "documented",
      "fixture_defined",
      "rust_planned",
      "rust_ported",
      "rust_compiled",
      "verified",
      "reviewed",
      "done",
      "blocked",
    ];
    let tasks = featureRepo.getFeatureTasks(db, f.id);
    if (status) tasks = tasks.filter((t) => t.status === status);
    tasks.sort(
      (a, b) =>
        order.indexOf(a.status) - order.indexOf(b.status) ||
        a.symbol_file.localeCompare(b.symbol_file),
    );
    return json(
      tasks.slice(0, limit ?? 15).map((t) => ({
        task_id: t.id,
        symbol: t.symbol_name,
        file: t.symbol_file,
        lines: `${t.start_line}-${t.end_line}`,
        status: t.status,
        flow_name: t.flow_name,
        risk_level: t.risk_level,
        target_rust_file: t.target_rust_file,
        rust_symbol_name: t.rust_symbol_name,
        claim_count: t.claim_count,
      })),
    );
  },
);

server.registerTool(
  "set_task_status",
  {
    title: "Set task status",
    description:
      "Advance a symbol along the porting ladder (discovered → documented → fixture_defined → rust_planned → rust_ported → rust_compiled → verified → reviewed → done; or 'blocked'). This is the tracking write — call it as you finish each stage.",
    inputSchema: {
      symbol: z.string().describe("Qualified symbol name, e.g. 'Spell::cast'"),
      status: TaskStatus.describe("New status"),
      notes: z.string().optional().describe("Optional note stored on the task"),
    },
  },
  async ({ symbol, status, notes }) => {
    const sym = symbolRepo.listAllSymbols(db).find((s) => s.name === symbol);
    if (!sym) return err(`No exact symbol "${symbol}". Use find_symbol to get the precise name.`);
    const taskId = taskRepo.upsertTask(db, sym.id, notes !== undefined ? { status, notes } : { status });
    return json({ ok: true, symbol: sym.name, task_id: taskId, status });
  },
);

const transport = new StdioServerTransport();
await server.connect(transport);
console.error("[port-harness] MCP server ready on stdio");
