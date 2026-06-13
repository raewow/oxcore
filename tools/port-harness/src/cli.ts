#!/usr/bin/env node
import { Command } from "commander";
import { serve } from "@hono/node-server";
import { resolve } from "node:path";
import { loadConfig, getPackageRoot } from "./config.js";
import type { HarnessConfig } from "./config.js";
import { getDb } from "./db/client.js";
import { indexFiles, applyFlowMappings } from "./index/indexer.js";
import { getProviderFromConfig } from "./agents/provider.js";
import {
  runExtract,
  runAssembleFlows,
  runFixtures,
  runPlanRust,
  runPort,
  runVerify,
  runAuditRust,
} from "./agents/pipeline.js";
import { runDiscover, buildSeedForQuery } from "./agents/discover.js";
import { scanReferenceCppInDir } from "./files/scanner.js";
import { exportMarkdownReport, printStatus } from "./export/markdown.js";
import { createApp } from "./server/app.js";
import { JobQueue } from "./server/jobQueue.js";
import { migrateStoredFilePaths } from "./files/migratePaths.js";
import {
  SPELL_FLOW_CATEGORIES_HINT,
  SPELL_FLOW_DEFS,
  SPELL_FLOW_MAP,
  SPELL_PILOT_FILES,
} from "./domains/spells.example.js";

function providerOpts(config: HarnessConfig) {
  return { ...config.provider, rustRoot: config.rustRoot };
}

function maybeApplyFlowMappings(db: ReturnType<typeof getDb>, config: HarnessConfig): number {
  if (!config.flowMappings || Object.keys(config.flowMappings).length === 0) {
    return 0;
  }
  return applyFlowMappings(db, config.flowMappings);
}

const program = new Command();

program
  .name("port-harness")
  .description("C++ to Rust migration agent harness")
  .option("--provider <name>", "LLM provider")
  .option("--model <model>", "LLM model");

program
  .command("index")
  .description("Index C++ files with tree-sitter")
  .argument("[files...]", "Files relative to reference root")
  .option("--dir <path>", "Index all .cpp files under a directory (relative to reference root)")
  .option("--all", "Index all .cpp files in reference tree")
  .option("--apply-mappings", "Apply flowMappings from port-harness.config.ts")
  .action(async (files: string[], opts: { dir?: string; all?: boolean; applyMappings?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);

    let toIndex = files;
    if (opts.all) {
      toIndex = scanReferenceCppInDir(config.referenceRoot, "").map((f) => f.path);
      console.log(`Indexing all ${toIndex.length} .cpp files…`);
    } else if (opts.dir) {
      toIndex = scanReferenceCppInDir(config.referenceRoot, opts.dir).map((f) => f.path);
      console.log(`Indexing ${toIndex.length} .cpp files under ${opts.dir}…`);
    }

    if (!toIndex.length) {
      console.error("No files to index. Provide files, --dir, or --all.");
      process.exit(1);
    }

    let totalSymbols = 0;
    for (const file of toIndex) {
      const result = await indexFiles(db, config, [file]);
      totalSymbols += result.symbolsIndexed;
    }
    const mappingsApplied = opts.applyMappings ? maybeApplyFlowMappings(db, config) : 0;
    console.log("Index complete:", { filesIndexed: toIndex.length, symbolsIndexed: totalSymbols, mappingsApplied });
  });

program
  .command("discover")
  .description("Investigate a bug or missing behavior — find symbols/files to port")
  .requiredOption("--query <text>", "Natural language description of the symptom")
  .option("--sync", "Run inline instead of queuing a job")
  .action(async (opts: { query: string; sync?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const query = opts.query.trim();
    const seed = buildSeedForQuery(db, query);
    const investigationId = (
      await import("./db/repositories/investigation.js")
    ).createInvestigation(db, query, JSON.stringify(seed));

    if (opts.sync) {
      const provider = await getProviderFromConfig(providerOpts(config));
      const result = await runDiscover(db, config, provider, query, investigationId);
      if (result.output) {
        console.log(JSON.stringify(result.output, null, 2));
      } else {
        console.error(result.error);
        process.exit(1);
      }
      return;
    }

    const jobsRepo = await import("./db/repositories/jobs.js");
    const jobId = jobsRepo.createJob(db, "discover", { query, investigationId });
    (
      await import("./db/repositories/investigation.js")
    ).setInvestigationJobId(db, investigationId, jobId);

    const queue = new JobQueue(db, config);
    queue.start();
    console.log(`Discovery queued: investigation #${investigationId}, job #${jobId}`);
    console.log(`DB seed hits: ${seed.hits.length}`);
  });

program
  .command("extract")
  .description("Extract behaviour documentation for a symbol")
  .requiredOption("--symbol <name>", "Symbol name e.g. Unit::Update")
  .option("--file <file>", "File filter")
  .option("--force", "Force re-extract")
  .action(async (opts: { symbol: string; file?: string; force?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runExtract(db, config, provider, opts.symbol, opts.file, opts.force);
    console.log(result);
  });

program
  .command("assemble-flows")
  .description("Assemble business flows from documented symbols")
  .requiredOption("--file <file>", "Source file")
  .action(async (opts: { file: string }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runAssembleFlows(db, config, provider, opts.file);
    console.log(result);
  });

program
  .command("fixtures")
  .description("Generate test fixtures for a flow")
  .requiredOption("--flow <name>", "Flow name")
  .option("--write-rust-tests", "Also emit reference Rust stubs under tools/port-harness/generated/")
  .action(async (opts: { flow: string; writeRustTests?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runFixtures(db, config, provider, opts.flow, opts.writeRustTests);
    console.log(result);
  });

program
  .command("plan-rust")
  .description("Plan Rust mapping for a task")
  .requiredOption("--task <id>", "Task ID", parseInt)
  .action(async (opts: { task: number }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runPlanRust(db, config, provider, opts.task);
    console.log(result);
  });

program
  .command("port")
  .description("Port a documented symbol to Rust")
  .requiredOption("--task <id>", "Task ID", parseInt)
  .option("--write", "Write output to docs/ports")
  .action(async (opts: { task: number; write?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runPort(db, config, provider, opts.task, opts.write);
    console.log(result);
  });

program
  .command("verify")
  .description("Verify Rust port against documented behaviour")
  .requiredOption("--task <id>", "Task ID", parseInt)
  .option("--cargo", "Run cargo test before LLM verification")
  .action(async (opts: { task: number; cargo?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runVerify(db, config, provider, opts.task, opts.cargo);
    console.log(result);
  });

program
  .command("audit-rust")
  .description("Audit existing Rust implementation against documented C++ behaviour")
  .option("--task <id>", "Task ID", parseInt)
  .option("--flow <id>", "Flow ID — audit all linked tasks", parseInt)
  .action(async (opts: { task?: number; flow?: number }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const provider = await getProviderFromConfig(providerOpts(config));

    if (opts.flow) {
      const { listTasksWithDetails } = await import("./db/repositories/migrationTask.js");
      const { tasks } = listTasksWithDetails(db, { limit: 10000 });
      const taskIds = tasks.filter((t) => t.flow_id === opts.flow).map((t) => t.id);
      if (!taskIds.length) {
        console.log({ success: false, error: "No tasks for flow" });
        return;
      }
      let passed = 0;
      for (const taskId of taskIds) {
        const result = await runAuditRust(db, config, provider, taskId);
        console.log({ taskId, ...result });
        if (result.passed) passed++;
      }
      console.log({ done: true, total: taskIds.length, passed });
      return;
    }

    if (!opts.task) {
      console.log({ success: false, error: "Provide --task or --flow" });
      return;
    }

    const result = await runAuditRust(db, config, provider, opts.task);
    console.log(result);
  });

program
  .command("status")
  .description("Print migration status")
  .option("--file <file>", "Filter by file")
  .action(async (opts: { file?: string }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    printStatus(db, opts.file);
  });

program
  .command("export")
  .description("Export migration report")
  .option("--format <format>", "markdown or json", "markdown")
  .option("--out <path>", "Output path", "docs/migration_report.md")
  .action(async (opts: { format: string; out: string }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    if (opts.format === "markdown") {
      exportMarkdownReport(db, resolve(getPackageRoot(), opts.out));
      console.log(`Exported to ${opts.out}`);
    }
  });

program
  .command("serve")
  .description("Start API server")
  .action(async () => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const migrated = migrateStoredFilePaths(db, config);
    if (migrated > 0) {
      console.log(`Migrated ${migrated} symbol file path(s) to full relative paths`);
    }
    const app = createApp(db, config);
    const queue = new JobQueue(db, config);
    queue.start();

    serve({ fetch: app.fetch, hostname: config.web.host, port: config.web.port }, () => {
      console.log(`API server running at http://${config.web.host}:${config.web.port}`);
    });
  });

program
  .command("worker")
  .description("Run job queue worker")
  .action(async () => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const queue = new JobQueue(db, config);
    queue.start();
    console.log("Worker started. Press Ctrl+C to stop.");
  });

const pilot = program
  .command("pilot")
  .description("Run an example domain bootstrap (index + optional flow seeding)");

pilot
  .command("spells")
  .description("Spell system example: index Spell.cpp and seed spell flows")
  .action(async () => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const result = await indexFiles(db, config, SPELL_PILOT_FILES);
    const mappingsApplied = applyFlowMappings(db, SPELL_FLOW_MAP);

    const { upsertFlow } = await import("./db/repositories/businessFlow.js");
    const sourceFile = "src/game/Spells/Spell.cpp";

    for (const f of SPELL_FLOW_DEFS) {
      upsertFlow(db, {
        name: f.name,
        description: f.description,
        entry_symbol_ids: [],
        expected_behaviour: "",
        risk_level: f.risk,
        source_file: sourceFile,
      });
    }

    const { listAllSymbols } = await import("./db/repositories/codeSymbol.js");
    const { getFlowByName, upsertFlow: upsertFlowFn } = await import(
      "./db/repositories/businessFlow.js"
    );

    const flowSymbols: Record<string, number[]> = {};
    for (const sym of listAllSymbols(db)) {
      const mapping = SPELL_FLOW_MAP[sym.name];
      if (mapping) {
        if (!flowSymbols[mapping.flow]) flowSymbols[mapping.flow] = [];
        flowSymbols[mapping.flow]!.push(sym.id);
      }
    }

    for (const [flowName, symbolIds] of Object.entries(flowSymbols)) {
      const existing = getFlowByName(db, flowName) as {
        id: number;
        name: string;
        description: string;
        expected_behaviour: string;
        risk_level: "low" | "medium" | "high" | "critical";
      };
      if (existing) {
        upsertFlowFn(db, {
          name: existing.name,
          description: existing.description,
          entry_symbol_ids: symbolIds,
          expected_behaviour: existing.expected_behaviour,
          risk_level: existing.risk_level,
          source_file: sourceFile,
        });
      }
    }

    exportMarkdownReport(db, resolve(getPackageRoot(), "docs/spell_migration.md"));
    printStatus(db, sourceFile);
    console.log("Spell pilot complete:", { ...result, mappingsApplied, flowCategoriesHint: SPELL_FLOW_CATEGORIES_HINT });
  });

program.parse();
