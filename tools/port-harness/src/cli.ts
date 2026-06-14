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
import { scanReferenceCppInDir, scanReferenceFiles } from "./files/scanner.js";
import { exportMarkdownReport, printStatus } from "./export/markdown.js";
import { createApp } from "./server/app.js";
import { JobQueues } from "./server/jobQueue.js";
import { migrateStoredFilePaths } from "./files/migratePaths.js";
import * as featureRepo from "./db/repositories/features.js";
import * as jobsRepo from "./db/repositories/jobs.js";
import * as investigationRepo from "./db/repositories/investigation.js";
import * as taskRepo from "./db/repositories/migrationTask.js";
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

function parseFeatureRef(db: ReturnType<typeof getDb>, ref: string): featureRepo.FeatureGroup {
  const id = Number(ref);
  const feature = Number.isInteger(id) && id > 0
    ? featureRepo.getFeature(db, id)
    : featureRepo.getFeatureByName(db, ref);
  if (!feature) {
    throw new Error(`Feature not found: ${ref}`);
  }
  return feature;
}

function createOrGetFeature(
  db: ReturnType<typeof getDb>,
  name: string,
  description?: string | null,
): featureRepo.FeatureGroup {
  const existing = featureRepo.getFeatureByName(db, name);
  if (existing) {
    if (description !== undefined) {
      featureRepo.updateFeature(db, existing.id, { description });
      return featureRepo.getFeature(db, existing.id)!;
    }
    return existing;
  }
  const id = featureRepo.createFeature(db, name, description ?? null);
  return featureRepo.getFeature(db, id)!;
}

function queueBatchedJobs(
  db: ReturnType<typeof getDb>,
  queues: JobQueues,
  stage: string,
  taskIds: number[],
  maxBatchSize: number,
): number[] {
  const jobIds = jobsRepo.createBatchedJobs(db, stage, taskIds, maxBatchSize);
  for (const jobId of jobIds) queues.enqueue(jobId);
  return jobIds;
}

function collectFeatureQueueTaskIds(
  db: ReturnType<typeof getDb>,
  featureId: number,
  status?: string,
): number[] {
  const tasks = featureRepo.getFeatureTasks(db, featureId);
  return tasks
    .filter((task) => !status || task.status === status)
    .map((task) => task.id);
}

function collectFeatureFilesFromOptions(
  config: HarnessConfig,
  opts: { file?: string[]; dir?: string[]; match?: string[]; includeHeaders?: boolean },
): string[] {
  const files = new Set(opts.file ?? []);
  for (const dir of opts.dir ?? []) {
    for (const file of scanReferenceFiles(config.referenceRoot)) {
      if (!opts.includeHeaders && file.kind !== "cpp") continue;
      if (file.path === dir || file.path.startsWith(`${dir.replace(/\\/g, "/").replace(/\/$/, "")}/`)) {
        files.add(file.path);
      }
    }
  }

  const patterns = (opts.match ?? []).map((p) => p.toLowerCase());
  if (patterns.length) {
    for (const file of scanReferenceFiles(config.referenceRoot)) {
      if (!opts.includeHeaders && file.kind !== "cpp") continue;
      const path = file.path.toLowerCase();
      if (patterns.some((pattern) => path.includes(pattern))) {
        files.add(file.path);
      }
    }
  }

  return [...files].sort();
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
    const seed = buildSeedForQuery(db, query, config.referenceRoot);
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

    const queues = new JobQueues(db, config);
    queues.start();
    queues.enqueue(jobId);
    console.log(`Discovery queued: investigation #${investigationId}, job #${jobId}`);
    console.log(`DB seed hits: ${seed.hits.length}`);
  });

const feature = program
  .command("feature")
  .description("Create feature groups, run discovery, and queue migration jobs");

feature
  .command("list")
  .description("List feature groups")
  .action(async () => {
    const config = await loadConfig();
    const db = getDb(config.database);
    console.log(JSON.stringify(featureRepo.listFeatures(db), null, 2));
  });

feature
  .command("create")
  .description("Create or update a feature group")
  .argument("<name>", "Feature name, e.g. spells")
  .option("--description <text>", "Feature description")
  .option("--file <path...>", "Assign one or more source files")
  .option("--dir <path...>", "Assign all reference files under one or more directories")
  .option("--match <text...>", "Assign files whose path contains one or more substrings")
  .option("--include-headers", "Include .h files when using --dir or --match")
  .option("--refresh-suggestions", "Refresh keyword-based suggestions")
  .action(async (
    name: string,
    opts: {
      description?: string;
      file?: string[];
      dir?: string[];
      match?: string[];
      includeHeaders?: boolean;
      refreshSuggestions?: boolean;
    },
  ) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const feature = createOrGetFeature(db, name, opts.description);
    const files = collectFeatureFilesFromOptions(config, opts);

    for (const file of files) {
      featureRepo.assignFeature(db, feature.id, "file", file);
    }

    const inserted = opts.refreshSuggestions
      ? featureRepo.refreshSuggestions(db, feature.id)
      : 0;

    console.log(JSON.stringify({
      feature,
      assignedFiles: files,
      suggestionsInserted: inserted,
    }, null, 2));
  });

feature
  .command("show")
  .description("Show a feature with assignments, suggestions, tasks, and stats")
  .argument("<feature>", "Feature id or name")
  .action(async (featureRef: string) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const feature = parseFeatureRef(db, featureRef);
    console.log(JSON.stringify({
      feature,
      assignments: featureRepo.listAssignments(db, feature.id),
      suggestions: featureRepo.listSuggestions(db, feature.id),
      stats: featureRepo.getFeatureTaskStats(db, feature.id),
      tasks: featureRepo.getFeatureTasks(db, feature.id),
    }, null, 2));
  });

feature
  .command("assign")
  .description("Assign files, flows, or tasks to a feature")
  .argument("<feature>", "Feature id or name")
  .option("--file <path...>", "Assign source file(s)")
  .option("--dir <path...>", "Assign all reference files under one or more directories")
  .option("--match <text...>", "Assign files whose path contains one or more substrings")
  .option("--include-headers", "Include .h files when using --dir or --match")
  .option("--flow <id...>", "Assign flow id(s)")
  .option("--task <id...>", "Assign task id(s)")
  .action(async (
    featureRef: string,
    opts: {
      file?: string[];
      dir?: string[];
      match?: string[];
      includeHeaders?: boolean;
      flow?: string[];
      task?: string[];
    },
  ) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const feature = parseFeatureRef(db, featureRef);
    const files = collectFeatureFilesFromOptions(config, opts);

    for (const file of files) featureRepo.assignFeature(db, feature.id, "file", file);
    for (const flow of opts.flow ?? []) featureRepo.assignFeature(db, feature.id, "flow", flow);
    for (const task of opts.task ?? []) featureRepo.assignFeature(db, feature.id, "task", task);

    console.log(JSON.stringify({
      ok: true,
      featureId: feature.id,
      assignments: featureRepo.listAssignments(db, feature.id),
    }, null, 2));
  });

feature
  .command("suggest")
  .description("Refresh and optionally accept keyword suggestions for a feature")
  .argument("<feature>", "Feature id or name")
  .option("--accept-all", "Accept all pending suggestions")
  .action(async (featureRef: string, opts: { acceptAll?: boolean }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const feature = parseFeatureRef(db, featureRef);
    const inserted = featureRepo.refreshSuggestions(db, feature.id);
    let accepted = 0;

    if (opts.acceptAll) {
      for (const suggestion of featureRepo.listSuggestions(db, feature.id).filter((s) => s.status === "pending")) {
        if (featureRepo.acceptSuggestion(db, suggestion.id)) accepted++;
      }
    }

    console.log(JSON.stringify({
      featureId: feature.id,
      inserted,
      accepted,
      suggestions: featureRepo.listSuggestions(db, feature.id),
    }, null, 2));
  });

feature
  .command("discover")
  .description("Run discovery for a feature and optionally queue the resulting jobs")
  .argument("<feature>", "Feature id or name")
  .requiredOption("--query <text>", "Discovery query")
  .option("--create", "Create the feature when it does not exist")
  .option("--description <text>", "Description used with --create")
  .option("--sync", "Run discovery inline and print candidates")
  .option("--assign", "Assign discovered files/tasks to the feature")
  .option("--queue", "Queue index/extract/verify jobs from discovery output")
  .action(async (
    featureRef: string,
    opts: {
      query: string;
      create?: boolean;
      description?: string;
      sync?: boolean;
      assign?: boolean;
      queue?: boolean;
    },
  ) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const feature = opts.create
      ? createOrGetFeature(db, featureRef, opts.description)
      : parseFeatureRef(db, featureRef);
    const query = opts.query.trim();
    const seed = buildSeedForQuery(db, query, config.referenceRoot);
    const investigationId = investigationRepo.createInvestigation(db, query, JSON.stringify(seed));

    if (!opts.sync) {
      const jobId = jobsRepo.createJob(db, "discover", { query, investigationId });
      investigationRepo.setInvestigationJobId(db, investigationId, jobId);
      const queues = new JobQueues(db, config);
      queues.start();
      queues.enqueue(jobId);
      console.log(JSON.stringify({
        ok: true,
        featureId: feature.id,
        investigationId,
        jobId,
        seedHitCount: seed.hits.length,
      }, null, 2));
      return;
    }

    const provider = await getProviderFromConfig(providerOpts(config));
    const result = await runDiscover(db, config, provider, query, investigationId);
    if (!result.success || !result.output) {
      console.log(JSON.stringify({
        ok: false,
        featureId: feature.id,
        investigationId,
        error: result.error,
      }, null, 2));
      process.exit(1);
    }

    const output = result.output;
    const assignedFiles = new Set<string>();
    const assignedTasks = new Set<number>();
    const extractTaskIds = new Set<number>();
    const verifyTaskIds = new Set<number>();
    const indexPaths = output.unindexed_files
      .map((file) => file.path)
      .filter((path) => path.endsWith(".cpp"));

    for (const candidate of output.candidates) {
      if (candidate.file) assignedFiles.add(candidate.file);
      if (!candidate.task_id) continue;

      assignedTasks.add(candidate.task_id);
      if (candidate.suggested_action === "verify") verifyTaskIds.add(candidate.task_id);
      if (candidate.suggested_action === "extract" || candidate.task_status === "discovered") {
        extractTaskIds.add(candidate.task_id);
      }
    }

    if (opts.assign) {
      for (const file of assignedFiles) featureRepo.assignFeature(db, feature.id, "file", file);
      for (const taskId of assignedTasks) featureRepo.assignFeature(db, feature.id, "task", String(taskId));
      featureRepo.refreshSuggestions(db, feature.id);
    }

    const queued: Record<string, number[] | number | undefined> = {};
    if (opts.queue) {
      const queues = new JobQueues(db, config);
      queues.start();

      if (indexPaths.length) {
        const jobId = jobsRepo.createJob(db, "index", { paths: indexPaths }, indexPaths.length);
        queues.enqueue(jobId);
        queued.index = jobId;
      }

      const extractIds = taskRepo.filterTaskIdsForExtract(db, [...extractTaskIds]);
      if (extractIds.length) {
        queued.extract = queueBatchedJobs(db, queues, "extract", extractIds, config.jobs.maxBatchSize);
      }

      if (verifyTaskIds.size) {
        queued.verify = queueBatchedJobs(db, queues, "verify", [...verifyTaskIds], config.jobs.maxBatchSize);
      }
    }

    console.log(JSON.stringify({
      ok: true,
      featureId: feature.id,
      investigationId,
      candidates: output.candidates.length,
      unindexedFiles: output.unindexed_files.length,
      assigned: opts.assign ? { files: assignedFiles.size, tasks: assignedTasks.size } : undefined,
      queued,
      result: output,
    }, null, 2));
  });

feature
  .command("queue")
  .description("Queue jobs for all tasks/files assigned to a feature")
  .argument("<feature>", "Feature id or name")
  .requiredOption("--stage <stage>", "pipeline, index, extract, assemble-flows, plan-rust, port, verify, or audit-rust")
  .option("--status <status>", "Only queue tasks with this status")
  .action(async (featureRef: string, opts: { stage: string; status?: string }) => {
    const config = await loadConfig();
    const db = getDb(config.database);
    const feature = parseFeatureRef(db, featureRef);
    const queues = new JobQueues(db, config);
    queues.start();

    if (opts.stage === "index") {
      const files = featureRepo
        .listAssignments(db, feature.id)
        .filter((a) => a.target_type === "file" && a.target_id.endsWith(".cpp"))
        .map((a) => a.target_id);
      if (!files.length) throw new Error("No assigned .cpp files to index");
      const jobId = jobsRepo.createJob(db, "index", { paths: files }, files.length);
      queues.enqueue(jobId);
      console.log(JSON.stringify({ ok: true, featureId: feature.id, jobId, files }, null, 2));
      return;
    }

    if (opts.stage === "pipeline" || opts.stage === "file-pipeline") {
      const files = featureRepo
        .listAssignments(db, feature.id)
        .filter((a) => a.target_type === "file" && a.target_id.endsWith(".cpp"))
        .map((a) => a.target_id);
      if (!files.length) throw new Error("No assigned .cpp files to pipeline");
      const jobIds = files.map((path) => jobsRepo.createJob(db, "file-pipeline", { path }));
      for (const jobId of jobIds) queues.enqueue(jobId);
      console.log(JSON.stringify({ ok: true, featureId: feature.id, jobIds, files }, null, 2));
      return;
    }

    if (opts.stage === "assemble-flows") {
      const files = featureRepo
        .listAssignments(db, feature.id)
        .filter((a) => a.target_type === "file" && a.target_id.endsWith(".cpp"))
        .map((a) => a.target_id);
      const jobIds = files.map((file) => jobsRepo.createJob(db, "assemble-flows", { file }));
      for (const jobId of jobIds) queues.enqueue(jobId);
      console.log(JSON.stringify({ ok: true, featureId: feature.id, jobIds, files }, null, 2));
      return;
    }

    let taskIds = collectFeatureQueueTaskIds(db, feature.id, opts.status);
    if (opts.stage === "extract") taskIds = taskRepo.filterTaskIdsForExtract(db, taskIds);
    if (!taskIds.length) throw new Error(`No feature tasks available for ${opts.stage}`);

    const jobIds = queueBatchedJobs(db, queues, opts.stage, taskIds, config.jobs.maxBatchSize);
    console.log(JSON.stringify({
      ok: true,
      featureId: feature.id,
      stage: opts.stage,
      jobIds,
      totalTasks: taskIds.length,
    }, null, 2));
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
    const queues = new JobQueues(db, config);
    queues.start();
    console.log("Worker started (agent + background queues). Press Ctrl+C to stop.");
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
