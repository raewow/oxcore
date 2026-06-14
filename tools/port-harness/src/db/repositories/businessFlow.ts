import type Database from "better-sqlite3";
import type { RiskLevel } from "../../models/index.js";

export interface InsertFlow {
  name: string;
  description: string;
  notes?: string;
  entry_symbol_ids: number[];
  expected_behaviour: string;
  risk_level: RiskLevel;
  source_file?: string;
}

export function upsertFlow(db: Database.Database, flow: InsertFlow): number {
  const existing = db
    .prepare("SELECT id FROM business_flow WHERE name = ?")
    .get(flow.name) as { id: number } | undefined;

  const entryIds = JSON.stringify(flow.entry_symbol_ids);

  if (existing) {
    db.prepare(
      `UPDATE business_flow SET description = ?, notes = ?, entry_symbol_ids = ?,
       expected_behaviour = ?, risk_level = ?, source_file = ? WHERE id = ?`,
    ).run(
      flow.description,
      flow.notes ?? null,
      entryIds,
      flow.expected_behaviour,
      flow.risk_level,
      flow.source_file ?? null,
      existing.id,
    );
    return existing.id;
  }

  const result = db
    .prepare(
      `INSERT INTO business_flow (name, description, notes, entry_symbol_ids, expected_behaviour, risk_level, source_file)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
    )
    .run(
      flow.name,
      flow.description,
      flow.notes ?? null,
      entryIds,
      flow.expected_behaviour,
      flow.risk_level,
      flow.source_file ?? null,
    );
  return Number(result.lastInsertRowid);
}

export function getFlowById(db: Database.Database, id: number) {
  return db.prepare("SELECT * FROM business_flow WHERE id = ?").get(id);
}

export function getFlowByName(db: Database.Database, name: string) {
  return db.prepare("SELECT * FROM business_flow WHERE name = ?").get(name);
}

export function listFlows(db: Database.Database) {
  return db.prepare("SELECT * FROM business_flow ORDER BY name").all();
}

export function listFlowsWithStats(db: Database.Database) {
  return db
    .prepare(
      `SELECT bf.*, COUNT(mt.id) AS symbol_count
       FROM business_flow bf
       LEFT JOIN migration_task mt ON mt.flow_id = bf.id
       GROUP BY bf.id
       ORDER BY bf.name`,
    )
    .all();
}

export function insertBranch(
  db: Database.Database,
  flowId: number,
  condition: string,
  behaviour: string,
  file: string,
  startLine: number,
  endLine: number,
): void {
  db.prepare(
    `INSERT INTO logic_branch (flow_id, condition, behaviour, file, start_line, end_line)
     VALUES (?, ?, ?, ?, ?, ?)`,
  ).run(flowId, condition, behaviour, file, startLine, endLine);
}

export function insertMutation(
  db: Database.Database,
  flowId: number,
  variable: string,
  description: string,
  file: string,
  startLine: number,
  endLine: number,
): void {
  db.prepare(
    `INSERT INTO state_mutation (flow_id, variable_or_field, mutation_description, file, start_line, end_line)
     VALUES (?, ?, ?, ?, ?, ?)`,
  ).run(flowId, variable, description, file, startLine, endLine);
}

export function getBranchesForFlow(db: Database.Database, flowId: number) {
  return db.prepare("SELECT * FROM logic_branch WHERE flow_id = ?").all(flowId);
}

export function getMutationsForFlow(db: Database.Database, flowId: number) {
  return db.prepare("SELECT * FROM state_mutation WHERE flow_id = ?").all(flowId);
}

export function deleteFlowData(db: Database.Database, flowId: number): void {
  db.prepare("DELETE FROM logic_branch WHERE flow_id = ?").run(flowId);
  db.prepare("DELETE FROM state_mutation WHERE flow_id = ?").run(flowId);
}
