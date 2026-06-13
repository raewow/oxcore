import type Database from "better-sqlite3";
import { resolve } from "node:path";
import { writeFileSync, mkdirSync, existsSync } from "node:fs";
import type { HarnessConfig } from "../config.js";
import { readPrompt, fillPrompt, getPackageRoot } from "../config.js";
import { resolveSourceFilePath } from "../files/paths.js";
import type { AgentProvider } from "./provider.js";
import { hashPrompt } from "./provider.js";
import {
  ExtractOutputSchema,
  FlowOutputSchema,
  FixtureOutputSchema,
  RustPlanOutputSchema,
  PortOutputSchema,
  VerifyOutputSchema,
  AuditOutputSchema,
  canTransitionToPort,
} from "../models/index.js";
import * as codeSymbolRepo from "../db/repositories/codeSymbol.js";
import * as claimRepo from "../db/repositories/behaviourClaim.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as flowRepo from "../db/repositories/businessFlow.js";
import * as jobsRepo from "../db/repositories/jobs.js";
import { getSourceSnippet } from "../index/parser.js";
import { validateClaims } from "../verify/citations.js";
import { generateRustTestScaffold } from "../verify/rustTestScaffold.js";
import { runCargoTest } from "../verify/cargoRunner.js";
import { findRustImplementation } from "../verify/rustLookup.js";
import { JobPausedError } from "../server/jobErrors.js";
import type { JobActivity } from "../server/jobActivity.js";
import { cleanPortRustCode } from "../verify/portOutput.js";
import { runDiscover } from "./discover.js";

export async function runExtract(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  symbolName: string,
  fileFilter?: string,
  force = false,
  activity?: JobActivity,
): Promise<{ success: boolean; error?: string; skipped?: boolean }> {
  const symbols = codeSymbolRepo.listAllSymbols(db);
  const symbol = symbols.find(
    (s) => s.name === symbolName && (!fileFilter || s.file.includes(fileFilter)),
  );

  if (!symbol) {
    return { success: false, error: `Symbol not found: ${symbolName}` };
  }

  const task = taskRepo.getTaskBySymbolId(db, symbol.id);
  if (task && task.status !== "discovered" && !force) {
    activity?.log(`Skipped ${symbolName} (already ${task.status})`);
    return { success: true, skipped: true };
  }

  if (!force && claimRepo.countClaimsForSymbol(db, symbol.id) > 0) {
    activity?.log(`Skipped ${symbolName} (already has behaviour claims)`);
    if (task?.status === "discovered") {
      taskRepo.upsertTask(db, symbol.id, { status: "documented" });
    }
    return { success: true, skipped: true };
  }

  activity?.setCurrent(symbolName);
  activity?.log(`Extracting behaviour for ${symbolName}`);
  activity?.log("Waiting for Cursor agent…");

  const fullPath = resolveSourceFilePath(config.referenceRoot, symbol.file, {
    symbolName: symbol.name,
  });
  const snippet = getSourceSnippet(fullPath, symbol.start_line, symbol.end_line);

  const prompt = fillPrompt(readPrompt("extract_behaviour"), {
    symbol: symbol.name,
    file: symbol.file,
    startLine: String(symbol.start_line),
    endLine: String(symbol.end_line),
    sourceSnippet: snippet,
  });

  const system =
    "You are a C++ code analyst documenting behaviour for a Rust port. Every claim must cite source lines.";

  try {
    const output = await provider.completeStructured(
      system,
      prompt,
      ExtractOutputSchema,
    );

    const validation = validateClaims(
      output.claims.map((c) => ({
        symbol_id: symbol.id,
        category: c.category,
        claim_text: c.claim_text,
        file: c.file,
        start_line: c.start_line,
        end_line: c.end_line,
        confidence: c.confidence ?? "high",
      })),
      symbol.file,
      symbol.start_line,
      symbol.end_line,
    );

    if (!validation.valid && !force) {
      // Retry once with validation errors
      const retryPrompt = `${prompt}\n\nPrevious attempt had invalid citations:\n${validation.errors.join("\n")}\nFix all citations.`;
      const retryOutput = await provider.completeStructured(
        system,
        retryPrompt,
        ExtractOutputSchema,
      );
      const retryValidation = validateClaims(
        retryOutput.claims.map((c) => ({
          symbol_id: symbol.id,
          category: c.category,
          claim_text: c.claim_text,
          file: c.file,
          start_line: c.start_line,
          end_line: c.end_line,
          confidence: c.confidence ?? "high",
        })),
        symbol.file,
        symbol.start_line,
        symbol.end_line,
      );

      if (!retryValidation.valid) {
        return { success: false, error: `Citation validation failed: ${retryValidation.errors.join("; ")}` };
      }

      claimRepo.deleteClaimsForSymbol(db, symbol.id);
      for (const claim of retryValidation.claims) {
        claimRepo.insertClaim(db, claim);
      }
      if (retryOutput.dependencies) {
        for (const dep of retryOutput.dependencies) {
          claimRepo.insertDependency(db, symbol.id, dep.type, dep.description, dep.file, dep.start_line);
        }
      }
      jobsRepo.insertAgentRun(
        db,
        "extract",
        provider.name,
        "model",
        hashPrompt(system, retryPrompt),
        JSON.stringify(retryOutput),
        symbol.id,
        task?.id,
      );
    } else {
      claimRepo.deleteClaimsForSymbol(db, symbol.id);
      for (const claim of validation.claims) {
        claimRepo.insertClaim(db, claim);
      }
      if (output.dependencies) {
        for (const dep of output.dependencies) {
          claimRepo.insertDependency(db, symbol.id, dep.type, dep.description, dep.file, dep.start_line);
        }
      }
      jobsRepo.insertAgentRun(
        db,
        "extract",
        provider.name,
        "model",
        hashPrompt(system, prompt),
        JSON.stringify(output),
        symbol.id,
        task?.id,
      );
    }

    codeSymbolRepo.upsertSymbol(db, {
      file: symbol.file,
      name: symbol.name,
      kind: symbol.kind,
      start_line: symbol.start_line,
      end_line: symbol.end_line,
      summary: output.summary,
    });

    taskRepo.upsertTask(db, symbol.id, { status: "documented" });
    exportSymbolDoc(config, symbol.name, symbol.file, snippet, output.summary);

    activity?.log(`Done: ${symbolName} → documented`);
    return { success: true };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    activity?.log(`Failed: ${symbolName} — ${message}`);
    return { success: false, error: message };
  }
}

export async function runAssembleFlows(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  fileFilter: string,
): Promise<{ success: boolean; flowsCreated: number; error?: string }> {
  const symbols = codeSymbolRepo.listSymbolsByFile(db, fileFilter);
  const claimsSummary = symbols
    .map((s) => {
      const claims = claimRepo.getClaimsForSymbol(db, s.id);
      return `### ${s.name}\n${claims.map((c) => `- [${c.category}] ${c.claim_text} (${c.file}:${c.start_line})`).join("\n")}`;
    })
    .join("\n\n");

  const flowCategoriesHint =
    config.flowCategoriesHint?.trim() ||
    "Identify flows from the symbols and claims above — no fixed categories.";

  const prompt = fillPrompt(readPrompt("assemble_flows"), {
    file: fileFilter,
    symbolsSummary: claimsSummary || "No documented symbols yet.",
    flowCategoriesHint,
  });

  try {
    const output = await provider.completeStructured(
      "You assemble C++ functions into business flows for migration tracking.",
      prompt,
      FlowOutputSchema,
    );

    let flowsCreated = 0;
    for (const flow of output.flows) {
      const entryIds: number[] = [];
      for (const symName of flow.entry_symbols) {
        const sym = symbols.find((s) => s.name === symName || s.name.endsWith(`::${symName}`));
        if (sym) entryIds.push(sym.id);
      }

      const flowId = flowRepo.upsertFlow(db, {
        name: flow.name,
        description: flow.description,
        entry_symbol_ids: entryIds,
        expected_behaviour: flow.expected_behaviour,
        risk_level: flow.risk_level,
        source_file: fileFilter,
      });

      flowRepo.deleteFlowData(db, flowId);
      for (const branch of flow.branches) {
        flowRepo.insertBranch(
          db,
          flowId,
          branch.condition,
          branch.behaviour,
          branch.file,
          branch.start_line,
          branch.end_line,
        );
      }
      for (const mut of flow.mutations) {
        flowRepo.insertMutation(
          db,
          flowId,
          mut.variable_or_field,
          mut.mutation_description,
          mut.file,
          mut.start_line,
          mut.end_line,
        );
      }

      for (const symId of entryIds) {
        taskRepo.upsertTask(db, symId, { flow_id: flowId });
      }

      flowsCreated++;
    }

    jobsRepo.insertAgentRun(
      db,
      "assemble-flows",
      provider.name,
      "model",
      hashPrompt("", prompt),
      JSON.stringify(output),
    );

    return { success: true, flowsCreated };
  } catch (err) {
    return { success: false, flowsCreated: 0, error: err instanceof Error ? err.message : String(err) };
  }
}

export async function runFixtures(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  flowName: string,
  writeRustTests = false,
): Promise<{ success: boolean; count: number; error?: string }> {
  const flow = flowRepo.getFlowByName(db, flowName) as {
    id: number;
    name: string;
    description: string;
    expected_behaviour: string;
  } | undefined;

  if (!flow) {
    return { success: false, count: 0, error: `Flow not found: ${flowName}` };
  }

  const branches = flowRepo.getBranchesForFlow(db, flow.id);
  const branchesSummary = branches
    .map((b) => {
      const br = b as { condition: string; behaviour: string; file: string; start_line: number };
      return `- IF ${br.condition} THEN ${br.behaviour} (${br.file}:${br.start_line})`;
    })
    .join("\n");

  const prompt = fillPrompt(readPrompt("generate_fixtures"), {
    flowName: flow.name,
    flowDescription: flow.description ?? "",
    expectedBehaviour: flow.expected_behaviour ?? "",
    branchesSummary: branchesSummary || "No branches documented.",
  });

  try {
    const output = await provider.completeStructured(
      "Generate JSON test fixtures describing inputs/expected outcomes for a C++ port.",
      prompt,
      FixtureOutputSchema,
    );

    const fixturesDir = resolve(getPackageRoot(), "fixtures");
    if (!existsSync(fixturesDir)) mkdirSync(fixturesDir, { recursive: true });

    const fixtureFile = resolve(fixturesDir, `${flow.name.replace(/[^a-z0-9]/gi, "_")}.json`);
    writeFileSync(fixtureFile, JSON.stringify(output.fixtures, null, 2));

    if (writeRustTests) {
      const fixtureSpecs = output.fixtures.map((f) => ({
        name: f.name,
        description: f.description,
        input: f.input,
        expected: f.expected,
        symbolName: flow.name,
      }));
      generateRustTestScaffold(config, fixtureSpecs, flow.name.replace(/[^a-z0-9]/gi, "_"));
    }

    for (const fixture of output.fixtures) {
      jobsRepo.insertFixture(
        db,
        fixture.name,
        JSON.stringify(fixture.input),
        JSON.stringify(fixture.expected),
        [flow.id],
      );
    }

    const entryIds = JSON.parse(
      (flowRepo.getFlowById(db, flow.id) as { entry_symbol_ids: string }).entry_symbol_ids,
    ) as number[];
    for (const symId of entryIds) {
      taskRepo.updateTaskStatus(db, taskRepo.getTaskBySymbolId(db, symId)!.id, "fixture_defined");
    }

    return { success: true, count: output.fixtures.length };
  } catch (err) {
    return { success: false, count: 0, error: err instanceof Error ? err.message : String(err) };
  }
}

export async function runPlanRust(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  taskId: number,
): Promise<{ success: boolean; error?: string }> {
  const task = taskRepo.getTaskById(db, taskId);
  if (!task) return { success: false, error: "Task not found" };

  const symbol = codeSymbolRepo.getSymbolById(db, task.source_symbol_id)!;
  const claims = claimRepo.getClaimsForSymbol(db, symbol.id);
  const fullPath = resolveSourceFilePath(config.referenceRoot, symbol.file, {
    symbolName: symbol.name,
  });
  const snippet = getSourceSnippet(fullPath, symbol.start_line, symbol.end_line);

  const prompt = fillPrompt(readPrompt("plan_rust"), {
    symbol: symbol.name,
    file: symbol.file,
    startLine: String(symbol.start_line),
    endLine: String(symbol.end_line),
    claimsSummary: claims.map((c) => `- [${c.category}] ${c.claim_text}`).join("\n"),
    existingRustPath: task.target_rust_file ?? "unknown",
  });

  try {
    const output = await provider.completeStructured(
      "Plan Rust mapping for C++ port. No code generation.",
      prompt,
      RustPlanOutputSchema,
    );

    taskRepo.upsertTask(db, symbol.id, {
      target_rust_file: output.target_rust_file,
      rust_symbol_name: output.rust_symbol_name,
      notes: output.notes,
      status: "rust_planned",
    });

    exportPlanDoc(symbol.name, output);
    jobsRepo.insertAgentRun(
      db,
      "plan-rust",
      provider.name,
      "model",
      hashPrompt("", prompt),
      JSON.stringify(output),
      symbol.id,
      taskId,
    );

    return { success: true };
  } catch (err) {
    return { success: false, error: err instanceof Error ? err.message : String(err) };
  }
}

export async function runPort(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  taskId: number,
  write = false,
): Promise<{ success: boolean; error?: string }> {
  const task = taskRepo.getTaskById(db, taskId);
  if (!task) return { success: false, error: "Task not found" };

  if (!canTransitionToPort(task.status) && task.status !== "rust_planned") {
    return { success: false, error: `Task status ${task.status} not ready for port` };
  }

  const symbol = codeSymbolRepo.getSymbolById(db, task.source_symbol_id)!;
  const claims = claimRepo.getClaimsForSymbol(db, symbol.id);
  const fullPath = resolveSourceFilePath(config.referenceRoot, symbol.file, {
    symbolName: symbol.name,
  });
  const snippet = getSourceSnippet(fullPath, symbol.start_line, symbol.end_line);

  const prompt = fillPrompt(readPrompt("port_symbol"), {
    symbol: symbol.name,
    file: symbol.file,
    startLine: String(symbol.start_line),
    endLine: String(symbol.end_line),
    sourceSnippet: snippet,
    claimsSummary: claims.map((c) => `- [${c.category}] ${c.claim_text} (${c.file}:${c.start_line})`).join("\n"),
    targetRustFile: task.target_rust_file ?? "",
    rustSymbolName: task.rust_symbol_name ?? symbol.name.split("::").pop() ?? "",
  });

  try {
    const output = await provider.completeStructured(
      "Port C++ to Rust preserving exact behaviour. Write clean idiomatic Rust without C++ trace comments.",
      prompt,
      PortOutputSchema,
    );

    output.rust_code = cleanPortRustCode(output.rust_code);

    if (write) {
      const docsDir = resolve(getPackageRoot(), "docs/ports");
      if (!existsSync(docsDir)) mkdirSync(docsDir, { recursive: true });
      const outFile = resolve(docsDir, `${symbol.name.replace(/::/g, "_")}.rs.txt`);
      writeFileSync(outFile, output.rust_code);
    }

    jobsRepo.insertAgentRun(
      db,
      "port",
      provider.name,
      "model",
      hashPrompt("", prompt),
      JSON.stringify(output),
      symbol.id,
      taskId,
    );

    taskRepo.updateTaskStatus(db, taskId, "rust_ported");
    return { success: true };
  } catch (err) {
    return { success: false, error: err instanceof Error ? err.message : String(err) };
  }
}

export async function runVerify(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  taskId: number,
  runCargo = false,
): Promise<{ success: boolean; passed?: boolean; error?: string }> {
  const task = taskRepo.getTaskById(db, taskId);
  if (!task) return { success: false, error: "Task not found" };

  const symbol = codeSymbolRepo.getSymbolById(db, task.source_symbol_id)!;
  const claims = claimRepo.getClaimsForSymbol(db, symbol.id);
  const fullPath = resolveSourceFilePath(config.referenceRoot, symbol.file, {
    symbolName: symbol.name,
  });
  const snippet = getSourceSnippet(fullPath, symbol.start_line, symbol.end_line);

  let testOutput = "cargo test skipped (use --cargo to run)";
  if (runCargo) {
    const cargoResult = await runCargoTest(config);
    testOutput = cargoResult.stdout + cargoResult.stderr;
    taskRepo.updateTaskStatus(db, taskId, cargoResult.success ? "rust_compiled" : "rust_ported");
  }

  const prompt = fillPrompt(readPrompt("verify_port"), {
    sourceSnippet: snippet,
    claimsSummary: claims.map((c) => `- [${c.category}] ${c.claim_text}`).join("\n"),
    rustCode: "// see target file",
    testOutput,
  });

  try {
    const output = await provider.completeStructured(
      "Verify Rust port against documented C++ behaviour.",
      prompt,
      VerifyOutputSchema,
    );

    if (output.passed) {
      taskRepo.updateTaskStatus(db, taskId, "verified");
    }

    return { success: true, passed: output.passed };
  } catch (err) {
    return { success: false, error: err instanceof Error ? err.message : String(err) };
  }
}

function buildFlowContext(
  db: Database.Database,
  flowId: number | null,
): string {
  if (!flowId) return "No flow assigned to this task.";

  const flow = flowRepo.getFlowById(db, flowId) as {
    name: string;
    description: string | null;
    expected_behaviour: string | null;
  } | null;
  if (!flow) return "Flow not found.";

  const branches = flowRepo.getBranchesForFlow(db, flowId);
  const branchSummary = branches
    .map((b) => {
      const br = b as { condition: string; behaviour: string; file: string; start_line: number };
      return `- IF ${br.condition} THEN ${br.behaviour} (${br.file}:${br.start_line})`;
    })
    .join("\n");

  return [
    `Flow: ${flow.name}`,
    flow.description ? `Description: ${flow.description}` : "",
    flow.expected_behaviour ? `Expected: ${flow.expected_behaviour}` : "",
    branchSummary ? `Branches:\n${branchSummary}` : "",
  ]
    .filter(Boolean)
    .join("\n\n");
}

function exportPlanDoc(
  symbolName: string,
  output: {
    target_rust_file: string;
    rust_symbol_name: string;
    structs?: string[];
    enums?: string[];
    notes?: string;
  },
): void {
  const docsDir = resolve(getPackageRoot(), "docs/plans");
  if (!existsSync(docsDir)) mkdirSync(docsDir, { recursive: true });

  const safeName = symbolName.replace(/::/g, "_");
  const structs = (output.structs ?? []).map((s) => `- ${s}`).join("\n") || "—";
  const enums = (output.enums ?? []).map((e) => `- ${e}`).join("\n") || "—";

  writeFileSync(
    resolve(docsDir, `${safeName}.md`),
    `# Plan: ${symbolName}\n\n` +
      `**Target file:** \`${output.target_rust_file}\`  \n` +
      `**Rust symbol:** \`${output.rust_symbol_name}\`\n\n` +
      `## Structs\n${structs}\n\n` +
      `## Enums\n${enums}\n\n` +
      `## Notes\n${output.notes ?? "—"}\n`,
  );
}

function exportAuditDoc(
  symbolName: string,
  output: {
    implementation_status: string;
    passed: boolean;
    summary: string;
    coverage: { claims_covered: number; claims_total: number };
    issues: { severity: string; message: string }[];
    rust_locations: { file: string; symbol: string }[];
  },
): void {
  const docsDir = resolve(getPackageRoot(), "docs/audits");
  if (!existsSync(docsDir)) mkdirSync(docsDir, { recursive: true });

  const safeName = symbolName.replace(/::/g, "_");
  const issues = output.issues.map((i) => `- [${i.severity}] ${i.message}`).join("\n");
  const locations = output.rust_locations
    .map((l) => `- \`${l.symbol}\` in \`${l.file}\``)
    .join("\n");

  writeFileSync(
    resolve(docsDir, `${safeName}.md`),
    `# Audit: ${symbolName}\n\n` +
      `**Status:** ${output.implementation_status}  \n` +
      `**Passed:** ${output.passed}  \n` +
      `**Coverage:** ${output.coverage.claims_covered}/${output.coverage.claims_total} claims\n\n` +
      `## Summary\n${output.summary}\n\n` +
      `## Rust locations\n${locations || "(none)"}\n\n` +
      `## Issues\n${issues || "(none)"}\n`,
  );
}

export async function runAuditRust(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  taskId: number,
  activity?: JobActivity,
): Promise<{ success: boolean; passed?: boolean; error?: string }> {
  const task = taskRepo.getTaskById(db, taskId);
  if (!task) return { success: false, error: "Task not found" };

  const symbol = codeSymbolRepo.getSymbolById(db, task.source_symbol_id)!;
  const claims = claimRepo.getClaimsForSymbol(db, symbol.id);
  if (!claims.length) {
    return { success: false, error: "No behaviour claims — run extract first" };
  }

  const fullPath = resolveSourceFilePath(config.referenceRoot, symbol.file, {
    symbolName: symbol.name,
  });
  const cppSnippet = getSourceSnippet(fullPath, symbol.start_line, symbol.end_line);
  const rustLookup = findRustImplementation(config.rustRoot, {
    cppSymbolName: symbol.name,
    targetFile: task.target_rust_file,
    rustSymbolName: task.rust_symbol_name,
  });

  activity?.log(`Rust search: ${rustLookup.searchNotes}`);

  const prompt = fillPrompt(readPrompt("audit_implementation"), {
    symbol: symbol.name,
    file: symbol.file,
    startLine: String(symbol.start_line),
    endLine: String(symbol.end_line),
    cppSnippet,
    claimsSummary: claims.map((c) => `- [${c.category}] ${c.claim_text} (${c.file}:${c.start_line})`).join("\n"),
    flowContext: buildFlowContext(db, task.flow_id),
    rustCode: rustLookup.snippets,
    rustSearchNotes: rustLookup.searchNotes,
  });

  try {
    const output = await provider.completeStructured(
      "Audit whether documented C++ behaviour already exists in the Rust codebase.",
      prompt,
      AuditOutputSchema,
    );

    const primary = output.rust_locations[0];
    const issueText = output.issues.map((i) => `- [${i.severity}] ${i.message}`).join("\n");
    const notes = [
      `[audit ${new Date().toISOString().slice(0, 10)}] ${output.implementation_status}`,
      output.summary,
      `Coverage: ${output.coverage.claims_covered}/${output.coverage.claims_total}`,
      issueText,
    ]
      .filter(Boolean)
      .join("\n\n");

    taskRepo.upsertTask(db, symbol.id, {
      notes,
      target_rust_file: primary?.file ?? task.target_rust_file,
      rust_symbol_name: primary?.symbol ?? task.rust_symbol_name,
      status: output.passed ? "reviewed" : task.status,
    });

    exportAuditDoc(symbol.name, output);
    jobsRepo.insertAgentRun(
      db,
      "audit-rust",
      provider.name,
      "model",
      hashPrompt("", prompt),
      JSON.stringify(output),
      symbol.id,
      taskId,
    );

    activity?.log(
      `${symbol.name}: ${output.implementation_status} (${output.coverage.claims_covered}/${output.coverage.claims_total} claims)`,
    );

    return { success: true, passed: output.passed };
  } catch (err) {
    return { success: false, error: err instanceof Error ? err.message : String(err) };
  }
}

function exportSymbolDoc(
  config: HarnessConfig,
  symbolName: string,
  file: string,
  snippet: string,
  summary: string,
): void {
  const docsDir = resolve(getPackageRoot(), "docs");
  if (!existsSync(docsDir)) mkdirSync(docsDir, { recursive: true });

  const safeName = symbolName.replace(/::/g, "_");
  writeFileSync(
    resolve(docsDir, `${safeName}.md`),
    `# ${symbolName}\n\n**File:** ${file}\n\n## Summary\n${summary}\n\n## Source\n\`\`\`cpp\n${snippet}\n\`\`\`\n`,
  );
}

export async function runPipelineJob(
  db: Database.Database,
  config: HarnessConfig,
  provider: AgentProvider,
  jobId: number,
  activity: JobActivity,
  shouldPause: () => boolean = () => false,
): Promise<void> {
  const job = jobsRepo.getJobById(db, jobId) as {
    id: number;
    stage: string;
    target_ids: string;
    total: number;
    progress: number;
  };

  if (!job) return;

  jobsRepo.beginJob(db, jobId);
  activity.log(`Job #${jobId} started (${job.stage})`);

  try {
    if (job.stage === "discover") {
      if (shouldPause()) {
        jobsRepo.pauseJob(db, jobId);
        activity.setCurrent(null);
        activity.log("Paused before discover");
        throw new JobPausedError(jobId);
      }
      activity.setCurrent(job.stage);
      const payload = JSON.parse(job.target_ids) as {
        query: string;
        investigationId: number;
      };
      activity.log(`Discovering: ${payload.query.slice(0, 60)}…`);
      const result = await runDiscover(
        db,
        config,
        provider,
        payload.query,
        payload.investigationId,
        activity,
      );
      if (!result.success) {
        activity.log(`Failed: ${result.error}`);
        jobsRepo.finishJob(db, jobId, "failed", result.error);
        return;
      }
      jobsRepo.updateJobProgress(db, jobId, 1);
      activity.setCurrent(null);
      activity.log("Discovery complete");
      jobsRepo.finishJob(db, jobId, "done");
      return;
    }

    if (job.stage === "assemble-flows") {
      if (shouldPause()) {
        jobsRepo.pauseJob(db, jobId);
        activity.setCurrent(null);
        activity.log("Paused before assemble-flows");
        throw new JobPausedError(jobId);
      }
      activity.setCurrent(job.stage);
      const payload = JSON.parse(job.target_ids) as { file: string };
      activity.log(`Assembling flows for ${payload.file}`);
      const result = await runAssembleFlows(db, config, provider, payload.file);
      if (!result.success) {
        activity.log(`Failed: ${result.error}`);
        jobsRepo.finishJob(db, jobId, "failed", result.error);
        return;
      }
      jobsRepo.updateJobProgress(db, jobId, 1);
      activity.setCurrent(null);
      activity.log("Flows assembled");
      jobsRepo.finishJob(db, jobId, "done");
      return;
    }

    const targetIds = JSON.parse(job.target_ids) as number[];
    const startIndex = job.progress;
    activity.log(`Processing ${targetIds.length} task(s)${startIndex > 0 ? `, resuming at ${startIndex + 1}` : ""}`);

    for (let i = startIndex; i < targetIds.length; i++) {
      if (shouldPause()) {
        jobsRepo.pauseJob(db, jobId);
        activity.setCurrent(null);
        activity.log(`Paused after ${i}/${targetIds.length} symbol(s)`);
        throw new JobPausedError(jobId);
      }

      const taskId = targetIds[i]!;
      const task = taskRepo.getTaskById(db, taskId);
      if (!task) {
        activity.log(`Task ${taskId} not found, skipping`);
        jobsRepo.updateJobProgress(db, jobId, i + 1);
        continue;
      }

      const symbol = codeSymbolRepo.getSymbolById(db, task.source_symbol_id);
      const label = symbol?.name ?? `task ${taskId}`;
      activity.setCurrent(label);
      activity.log(`[${i + 1}/${targetIds.length}] ${label}`);

      switch (job.stage) {
        case "extract": {
          if (!symbol) {
            activity.log(`No symbol for task ${taskId}, skipping`);
            break;
          }
          if (!taskRepo.taskNeedsExtract(db, taskId)) {
            activity.log(`Skipped ${label} (already extracted)`);
            break;
          }
          await runExtract(db, config, provider, symbol.name, undefined, false, activity);
          break;
        }
        case "plan-rust":
          await runPlanRust(db, config, provider, taskId);
          break;
        case "port":
          await runPort(db, config, provider, taskId, true);
          break;
        case "verify":
          await runVerify(db, config, provider, taskId);
          break;
        case "audit-rust":
          await runAuditRust(db, config, provider, taskId, activity);
          break;
      }

      jobsRepo.updateJobProgress(db, jobId, i + 1);
    }

    activity.setCurrent(null);
    activity.log("Job completed");
    jobsRepo.finishJob(db, jobId, "done");
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    activity.setCurrent(null);
    activity.log(`Job failed: ${message}`);
    jobsRepo.finishJob(db, jobId, "failed", message);
  }
}
