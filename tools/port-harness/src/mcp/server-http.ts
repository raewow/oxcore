#!/usr/bin/env node
/**
 * Port-harness MCP server — HTTP/SSE transport.
 *
 * Run standalone:  npm run mcp:http
 * Codex: codex mcp add port-harness --url http://localhost:3456/mcp
 * Legacy SSE clients: { "type": "sse", "url": "http://localhost:3456/sse" }
 */
import { createServer, IncomingMessage, ServerResponse } from "node:http";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { SSEServerTransport } from "@modelcontextprotocol/sdk/server/sse.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { z } from "zod";
import type Database from "better-sqlite3";
import { getDb } from "../db/client.js";
import { TaskStatus } from "../models/index.js";
import * as symbolRepo from "../db/repositories/codeSymbol.js";
import * as claimRepo from "../db/repositories/behaviourClaim.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as featureRepo from "../db/repositories/features.js";
import { getFeatureCoverage } from "../db/repositories/coverage.js";
import { getFlowDetailsForMcp, listFlowsForMcp } from "./flowTools.js";

const PORT = Number(process.env.MCP_PORT ?? 3456);

function log(msg: string) {
  console.error(`[port-harness:http] ${msg}`);
}

log("Initialising database...");
const db = getDb();
log("Database ready.");

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
  return featureRepo.listFeatures(db).find((f) => f.name.toLowerCase() === ref.toLowerCase());
}
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

function buildServer() {
  const server = new McpServer({ name: "port-harness", version: "0.1.0" });

  server.registerTool("find_symbol", {
    title: "Find symbol",
    description: "Search the indexed C++ symbol tree by name (partial match).",
    inputSchema: { query: z.string(), limit: z.number().optional() },
  }, async ({ query, limit }) => {
    log(`find_symbol query="${query}"`);
    return json(searchSymbols(db, query, limit ?? 20));
  });

  server.registerTool("symbol_callees", {
    title: "Symbol callees",
    description: "List everything a symbol calls (outgoing call-graph edges).",
    inputSchema: { name: z.string() },
  }, async ({ name }) => {
    log(`symbol_callees name="${name}"`);
    const sym = symbolRepo.listAllSymbols(db).find((s) => s.name === name) ?? searchSymbols(db, name, 1)[0];
    if (!sym) return err(`No symbol matching "${name}"`);
    const calls = symbolRepo.getCallsForSymbol(db, sym.id as number);
    const indexedShort = new Set(symbolRepo.listAllSymbols(db).map((s) => s.name.split("::").pop()!));
    const callees = calls.map((c) => ({ callee: c.callee_name, line: c.line, indexed: indexedShort.has(c.callee_name.split("::").pop()!) }));
    return json({ symbol: (sym as { name: string }).name, callee_count: callees.length, callees });
  });

  server.registerTool("symbol_callers", {
    title: "Symbol callers",
    description: "List every indexed symbol that calls the given symbol (incoming call-graph edges).",
    inputSchema: { name: z.string() },
  }, async ({ name }) => {
    log(`symbol_callers name="${name}"`);
    const callers = symbolRepo.getCallersForCallee(db, name);
    const out = callers.map((c) => { const sym = symbolRepo.getSymbolById(db, c.caller_id); return sym ? { caller: sym.name, file: sym.file, line: c.line } : null; }).filter(Boolean);
    return json({ callee: name, caller_count: out.length, callers: out });
  });

  server.registerTool("behaviour_claims", {
    title: "Behaviour claims",
    description: "The extracted behaviour spec for a symbol.",
    inputSchema: { name: z.string() },
  }, async ({ name }) => {
    log(`behaviour_claims name="${name}"`);
    const sym = symbolRepo.listAllSymbols(db).find((s) => s.name === name) ?? searchSymbols(db, name, 1)[0];
    if (!sym) return err(`No symbol matching "${name}"`);
    const id = sym.id as number;
    const claims = claimRepo.getClaimsForSymbol(db, id);
    const byCategory: Record<string, string[]> = {};
    for (const c of claims) (byCategory[c.category] ??= []).push(c.claim_text);
    const deps = db.prepare("SELECT type, description FROM dependency WHERE symbol_id = ?").all(id) as { type: string; description: string }[];
    return json({ symbol: (sym as { name: string }).name, file: (sym as { file: string }).file, lines: `${(sym as { start_line: number }).start_line}-${(sym as { end_line: number }).end_line}`, claim_count: claims.length, claims: byCategory, dependencies: deps });
  });

  server.registerTool("feature_coverage", {
    title: "Feature coverage",
    description: "Is every detail of a feature mapped?",
    inputSchema: { feature: z.string() },
  }, async ({ feature }) => {
    log(`feature_coverage feature="${feature}"`);
    const f = resolveFeature(db, feature);
    if (!f) return err(`No feature "${feature}"`);
    return json(getFeatureCoverage(db, f.id, f.name));
  });

  server.registerTool("feature_status", {
    title: "Feature status",
    description: "Porting progress for a feature.",
    inputSchema: { feature: z.string() },
  }, async ({ feature }) => {
    log(`feature_status feature="${feature}"`);
    const f = resolveFeature(db, feature);
    if (!f) return err(`No feature "${feature}"`);
    const tasks = featureRepo.getFeatureTasks(db, f.id);
    const stats = featureRepo.getFeatureTaskStats(db, f.id);
    const incomplete = tasks.filter((t) => !["verified", "reviewed", "done"].includes(t.status)).map((t) => ({ symbol: t.symbol_name, file: t.symbol_file, status: t.status }));
    return json({ feature: f.name, total_tasks: stats.total, by_status: stats.by_status, blocking_count: incomplete.length, blocking: incomplete.slice(0, 100) });
  });

  server.registerTool("list_flows", {
    title: "List flows",
    description: "List all business flows with progress and stage.",
    inputSchema: {},
  }, async () => json(listFlowsForMcp(db)));

  server.registerTool("flow_details", {
    title: "Flow details",
    description: "Get the full flow record plus branches, mutations, linked tasks, and plan/port drafts.",
    inputSchema: { flow: z.string() },
  }, async ({ flow }) => {
    const details = getFlowDetailsForMcp(db, null, flow);
    if (!details) return err(`No flow "${flow}"`);
    return json(details);
  });

  server.registerTool("next_tasks", {
    title: "Next tasks",
    description: "The next symbols to work on for a feature.",
    inputSchema: { feature: z.string(), status: TaskStatus.optional(), limit: z.number().optional() },
  }, async ({ feature, status, limit }) => {
    log(`next_tasks feature="${feature}" status=${status}`);
    const f = resolveFeature(db, feature);
    if (!f) return err(`No feature "${feature}"`);
    const order = ["discovered","documented","fixture_defined","rust_planned","rust_ported","rust_compiled","verified","reviewed","done","blocked"];
    let tasks = featureRepo.getFeatureTasks(db, f.id);
    if (status) tasks = tasks.filter((t) => t.status === status);
    tasks.sort((a, b) => order.indexOf(a.status) - order.indexOf(b.status) || a.symbol_file.localeCompare(b.symbol_file));
    return json(tasks.slice(0, limit ?? 15).map((t) => ({ task_id: t.id, symbol: t.symbol_name, file: t.symbol_file, lines: `${t.start_line}-${t.end_line}`, status: t.status, flow_name: t.flow_name, risk_level: t.risk_level, target_rust_file: t.target_rust_file, rust_symbol_name: t.rust_symbol_name, claim_count: t.claim_count })));
  });

  server.registerTool("set_task_status", {
    title: "Set task status",
    description: "Advance a symbol along the porting ladder.",
    inputSchema: { symbol: z.string(), status: TaskStatus.describe("New status"), notes: z.string().optional() },
  }, async ({ symbol, status, notes }) => {
    log(`set_task_status symbol="${symbol}" status="${status}"`);
    const sym = symbolRepo.listAllSymbols(db).find((s) => s.name === symbol);
    if (!sym) return err(`No exact symbol "${symbol}". Use find_symbol to get the precise name.`);
    const taskId = taskRepo.upsertTask(db, sym.id, notes !== undefined ? { status, notes } : { status });
    return json({ ok: true, symbol: sym.name, task_id: taskId, status });
  });

  return server;
}

// Session map: sessionId -> transport (for routing POST /messages back)
const sessions = new Map<string, SSEServerTransport>();

const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
  log(`${req.method} ${req.url}`);

  if (req.url === "/mcp") {
    if (req.method !== "POST") {
      res.writeHead(405, { "Content-Type": "application/json" });
      res.end(JSON.stringify({
        jsonrpc: "2.0",
        error: { code: -32000, message: "Method not allowed." },
        id: null,
      }));
      return;
    }

    const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
    const server = buildServer();
    await server.connect(transport);
    res.on("close", () => {
      void transport.close();
      void server.close();
    });
    await transport.handleRequest(req, res);
    return;
  }

  if (req.method === "GET" && req.url === "/sse") {
    log("New SSE client connected");
    const transport = new SSEServerTransport("/messages", res);
    sessions.set(transport.sessionId, transport);
    transport.onclose = () => {
      log(`SSE client disconnected (session ${transport.sessionId})`);
      sessions.delete(transport.sessionId);
    };
    const server = buildServer();
    await server.connect(transport);
    log(`SSE transport connected (session ${transport.sessionId})`);
    return;
  }

  if (req.method === "POST" && req.url?.startsWith("/messages")) {
    const sessionId = new URL(req.url, `http://localhost:${PORT}`).searchParams.get("sessionId");
    log(`POST /messages sessionId=${sessionId} known=${sessions.has(sessionId ?? "")}`);
    const transport = sessionId ? sessions.get(sessionId) : undefined;
    if (!transport) {
      log(`ERROR: no session found for sessionId=${sessionId}`);
      res.writeHead(404);
      res.end("Unknown session");
      return;
    }
    await transport.handlePostMessage(req, res);
    return;
  }

  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end(`port-harness MCP\nCodex Streamable HTTP: http://localhost:${PORT}/mcp\nLegacy SSE: http://localhost:${PORT}/sse\n`);
});

httpServer.listen(PORT, () => {
  log(`Listening on http://localhost:${PORT}`);
  log(`Streamable HTTP endpoint: http://localhost:${PORT}/mcp`);
  log(`SSE endpoint: http://localhost:${PORT}/sse`);
});
