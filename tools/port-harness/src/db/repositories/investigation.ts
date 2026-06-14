import type Database from "better-sqlite3";
import type { DiscoverOutput, InvestigationStatus } from "../../models/index.js";

export interface Investigation {
  id: number;
  query: string;
  status: InvestigationStatus;
  seed_json: string | null;
  result_json: string | null;
  job_id: number | null;
  feature_id: number | null;
  created_at: string;
  finished_at: string | null;
}

export function createInvestigation(
  db: Database.Database,
  query: string,
  seedJson: string,
  jobId?: number,
  featureId?: number,
): number {
  const result = db
    .prepare(
      `INSERT INTO investigation (query, status, seed_json, job_id, feature_id)
       VALUES (?, 'running', ?, ?, ?)`,
    )
    .run(query, seedJson, jobId ?? null, featureId ?? null);
  return Number(result.lastInsertRowid);
}

export function listInvestigationsByFeature(
  db: Database.Database,
  featureId: number,
  limit = 20,
): Investigation[] {
  return db
    .prepare("SELECT * FROM investigation WHERE feature_id = ? ORDER BY created_at DESC LIMIT ?")
    .all(featureId, limit) as Investigation[];
}

export function finishInvestigation(
  db: Database.Database,
  id: number,
  status: InvestigationStatus,
  resultJson?: string,
): void {
  db.prepare(
    `UPDATE investigation
     SET status = ?, result_json = ?, finished_at = datetime('now')
     WHERE id = ?`,
  ).run(status, resultJson ?? null, id);
}

export function setInvestigationJobId(
  db: Database.Database,
  id: number,
  jobId: number,
): void {
  db.prepare("UPDATE investigation SET job_id = ? WHERE id = ?").run(jobId, id);
}

export function getInvestigationById(
  db: Database.Database,
  id: number,
): Investigation | undefined {
  return db.prepare("SELECT * FROM investigation WHERE id = ?").get(id) as
    | Investigation
    | undefined;
}

export function listInvestigations(db: Database.Database, limit = 50): Investigation[] {
  return db
    .prepare("SELECT * FROM investigation ORDER BY created_at DESC LIMIT ?")
    .all(limit) as Investigation[];
}

export function parseInvestigationResult(
  inv: Investigation,
): DiscoverOutput | null {
  if (!inv.result_json) return null;
  try {
    return JSON.parse(inv.result_json) as DiscoverOutput;
  } catch {
    return null;
  }
}

export function parseInvestigationSeed(inv: Investigation): unknown {
  if (!inv.seed_json) return null;
  try {
    return JSON.parse(inv.seed_json);
  } catch {
    return null;
  }
}
