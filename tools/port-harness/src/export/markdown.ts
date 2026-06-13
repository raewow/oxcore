import type Database from "better-sqlite3";
import { writeFileSync, mkdirSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { getPackageRoot } from "../config.js";
import * as taskRepo from "../db/repositories/migrationTask.js";
import * as claimRepo from "../db/repositories/behaviourClaim.js";
import * as codeSymbolRepo from "../db/repositories/codeSymbol.js";
import * as flowRepo from "../db/repositories/businessFlow.js";

export function exportMarkdownReport(db: Database.Database, outPath: string): void {
  const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 10000 });
  const statusCounts = taskRepo.getStatusCounts(db);
  const fileProgress = taskRepo.getFileProgress(db);

  let md = `# Migration Dashboard\n\nGenerated: ${new Date().toISOString()}\n\n`;

  md += `## Status Summary\n\n`;
  md += `| Status | Count |\n|--------|-------|\n`;
  for (const s of statusCounts) {
    md += `| ${s.status} | ${s.count} |\n`;
  }

  md += `\n## File Progress\n\n`;
  md += `| File | Documented | Total |\n|------|------------|-------|\n`;
  for (const f of fileProgress) {
    md += `| ${f.file} | ${f.documented} | ${f.total} |\n`;
  }

  md += `\n## Tasks\n\n`;
  md += `| Symbol | File | Status | Rust Target | Claims |\n`;
  md += `|--------|------|--------|-------------|--------|\n`;
  for (const t of tasks) {
    md += `| ${t.symbol_name} | ${t.symbol_file}:${t.start_line} | ${t.status} | ${t.target_rust_file ?? "-"} | ${t.claim_count} |\n`;
  }

  const dir = resolve(outPath, "..");
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  writeFileSync(outPath, md);
}

export function exportJsonReport(db: Database.Database): Record<string, unknown> {
  const { tasks } = taskRepo.listTasksWithDetails(db, { limit: 10000 });
  const flows = flowRepo.listFlows(db);
  const statusCounts = taskRepo.getStatusCounts(db);

  return {
    generated_at: new Date().toISOString(),
    status_counts: statusCounts,
    tasks,
    flows,
  };
}

export function printStatus(db: Database.Database, fileFilter?: string): void {
  const filter = fileFilter ? { file: fileFilter } : {};
  const { tasks, total } = taskRepo.listTasksWithDetails(db, { ...filter, limit: 10000 });
  const statusCounts = taskRepo.getStatusCounts(db);

  console.log("\n=== Migration Status ===\n");
  console.log("Status counts:");
  for (const s of statusCounts) {
    console.log(`  ${s.status}: ${s.count}`);
  }

  console.log(`\nTasks${fileFilter ? ` (${fileFilter})` : ""}: ${total}\n`);
  console.log(
    "Symbol".padEnd(40) +
      "Status".padEnd(18) +
      "Claims".padEnd(8) +
      "Rust Target",
  );
  console.log("-".repeat(100));

  for (const t of tasks) {
    console.log(
      t.symbol_name.padEnd(40) +
        t.status.padEnd(18) +
        String(t.claim_count).padEnd(8) +
        (t.target_rust_file ?? "-"),
    );
  }
}
