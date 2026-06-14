import type Database from "better-sqlite3";
import type { TaskWithDetails } from "../../models/index.js";
import { listTasksWithDetails } from "./migrationTask.js";

export type FeatureTargetType = "file" | "flow" | "task";
export type SuggestionConfidence = "high" | "medium" | "low";

export interface FeatureGroup {
  id: number;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface FeatureAssignment {
  id: number;
  feature_id: number;
  target_type: FeatureTargetType;
  target_id: string;
  created_at: string;
}

export interface FeatureSuggestion {
  id: number;
  feature_id: number;
  target_type: FeatureTargetType;
  target_id: string;
  reason: string;
  confidence: SuggestionConfidence;
  status: "pending" | "accepted" | "rejected";
  created_at: string;
}

export interface FeatureSummary extends FeatureGroup {
  file_count: number;
  flow_count: number;
  task_count: number;
  pending_suggestions: number;
  done: number;
  blocked: number;
  total_tasks: number;
}

function normalizeText(text: string): string {
  return text.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
}

function keywordsForFeature(feature: FeatureGroup): string[] {
  const words = normalizeText(`${feature.name} ${feature.description ?? ""}`)
    .split(/\s+/)
    .filter((w) => w.length >= 4);
  return [...new Set(words)];
}

function includesAny(haystack: string, needles: string[]): string | null {
  const normalized = normalizeText(haystack);
  return needles.find((k) => normalized.includes(k)) ?? null;
}

export function listFeatures(db: Database.Database): FeatureSummary[] {
  const features = db
    .prepare("SELECT * FROM feature_group ORDER BY name")
    .all() as FeatureGroup[];

  return features.map((feature) => {
    const counts = db
      .prepare(
        `SELECT
           SUM(CASE WHEN target_type = 'file' THEN 1 ELSE 0 END) AS file_count,
           SUM(CASE WHEN target_type = 'flow' THEN 1 ELSE 0 END) AS flow_count,
           SUM(CASE WHEN target_type = 'task' THEN 1 ELSE 0 END) AS task_count
         FROM feature_assignment
         WHERE feature_id = ?`,
      )
      .get(feature.id) as {
      file_count: number | null;
      flow_count: number | null;
      task_count: number | null;
    };
    const pending = db
      .prepare(
        "SELECT COUNT(*) AS c FROM feature_suggestion WHERE feature_id = ? AND status = 'pending'",
      )
      .get(feature.id) as { c: number };
    const taskStats = getFeatureTaskStats(db, feature.id);
    return {
      ...feature,
      file_count: counts.file_count ?? 0,
      flow_count: counts.flow_count ?? 0,
      task_count: counts.task_count ?? 0,
      pending_suggestions: pending.c,
      done: taskStats.done,
      blocked: taskStats.blocked,
      total_tasks: taskStats.total,
    };
  });
}

export function getFeature(db: Database.Database, id: number): FeatureGroup | undefined {
  return db.prepare("SELECT * FROM feature_group WHERE id = ?").get(id) as
    | FeatureGroup
    | undefined;
}

export function getFeatureByName(
  db: Database.Database,
  name: string,
): FeatureGroup | undefined {
  return db.prepare("SELECT * FROM feature_group WHERE name = ?").get(name) as
    | FeatureGroup
    | undefined;
}

export function createFeature(
  db: Database.Database,
  name: string,
  description?: string | null,
): number {
  const result = db
    .prepare("INSERT INTO feature_group (name, description) VALUES (?, ?)")
    .run(name, description ?? null);
  return Number(result.lastInsertRowid);
}

export function updateFeature(
  db: Database.Database,
  id: number,
  fields: { name?: string; description?: string | null },
): void {
  const sets = ["updated_at = datetime('now')"];
  const values: unknown[] = [];
  if (fields.name !== undefined) {
    sets.push("name = ?");
    values.push(fields.name);
  }
  if (fields.description !== undefined) {
    sets.push("description = ?");
    values.push(fields.description);
  }
  values.push(id);
  db.prepare(`UPDATE feature_group SET ${sets.join(", ")} WHERE id = ?`).run(...values);
}

export function listAssignments(
  db: Database.Database,
  featureId: number,
): FeatureAssignment[] {
  return db
    .prepare("SELECT * FROM feature_assignment WHERE feature_id = ? ORDER BY target_type, target_id")
    .all(featureId) as FeatureAssignment[];
}

export function assignFeature(
  db: Database.Database,
  featureId: number,
  targetType: FeatureTargetType,
  targetId: string,
): void {
  db.prepare(
    `INSERT OR IGNORE INTO feature_assignment (feature_id, target_type, target_id)
     VALUES (?, ?, ?)`,
  ).run(featureId, targetType, targetId);
}

export function unassignFeature(
  db: Database.Database,
  featureId: number,
  targetType: FeatureTargetType,
  targetId: string,
): void {
  db.prepare(
    "DELETE FROM feature_assignment WHERE feature_id = ? AND target_type = ? AND target_id = ?",
  ).run(featureId, targetType, targetId);
}

export function listSuggestions(
  db: Database.Database,
  featureId: number,
): FeatureSuggestion[] {
  return db
    .prepare(
      `SELECT * FROM feature_suggestion
       WHERE feature_id = ?
       ORDER BY CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, target_type, target_id`,
    )
    .all(featureId) as FeatureSuggestion[];
}

export function acceptSuggestion(db: Database.Database, suggestionId: number): boolean {
  const suggestion = db
    .prepare("SELECT * FROM feature_suggestion WHERE id = ?")
    .get(suggestionId) as FeatureSuggestion | undefined;
  if (!suggestion) return false;
  assignFeature(db, suggestion.feature_id, suggestion.target_type, suggestion.target_id);
  db.prepare("UPDATE feature_suggestion SET status = 'accepted' WHERE id = ?").run(suggestionId);
  return true;
}

export function rejectSuggestion(db: Database.Database, suggestionId: number): boolean {
  const result = db
    .prepare("UPDATE feature_suggestion SET status = 'rejected' WHERE id = ?")
    .run(suggestionId);
  return result.changes > 0;
}

export function refreshSuggestions(db: Database.Database, featureId: number): number {
  const feature = getFeature(db, featureId);
  if (!feature) return 0;
  const keywords = keywordsForFeature(feature);
  if (!keywords.length) return 0;

  let inserted = 0;
  const assignments = new Set(
    listAssignments(db, featureId).map((a) => `${a.target_type}:${a.target_id}`),
  );

  const insert = (
    targetType: FeatureTargetType,
    targetId: string,
    reason: string,
    confidence: SuggestionConfidence,
  ) => {
    if (assignments.has(`${targetType}:${targetId}`)) return;
    const result = db
      .prepare(
        `INSERT OR IGNORE INTO feature_suggestion
           (feature_id, target_type, target_id, reason, confidence)
         VALUES (?, ?, ?, ?, ?)`,
      )
      .run(featureId, targetType, targetId, reason, confidence);
    inserted += result.changes;
  };

  const tasks = listTasksWithDetails(db, { limit: 100000 }).tasks;
  const files = new Set<string>();
  const flows = new Map<number, string>();

  for (const task of tasks) {
    files.add(task.symbol_file);
    if (task.flow_id && task.flow_name) flows.set(task.flow_id, task.flow_name);

    const taskHit = includesAny(
      `${task.symbol_name} ${task.symbol_file} ${task.target_rust_file ?? ""} ${task.flow_name ?? ""}`,
      keywords,
    );
    if (taskHit) {
      insert("task", String(task.id), `Task matches "${taskHit}"`, "medium");
    }
  }

  for (const file of files) {
    const hit = includesAny(file, keywords);
    if (hit) insert("file", file, `File path matches "${hit}"`, "high");
  }

  for (const [flowId, flowName] of flows) {
    const hit = includesAny(flowName, keywords);
    if (hit) insert("flow", String(flowId), `Flow name matches "${hit}"`, "high");
  }

  return inserted;
}

export function getFeatureTasks(
  db: Database.Database,
  featureId: number,
): TaskWithDetails[] {
  const assignments = listAssignments(db, featureId);
  const taskIds = new Set<number>();
  const files = new Set<string>();
  const flowIds = new Set<number>();

  for (const assignment of assignments) {
    if (assignment.target_type === "task") taskIds.add(Number(assignment.target_id));
    if (assignment.target_type === "file") files.add(assignment.target_id);
    if (assignment.target_type === "flow") flowIds.add(Number(assignment.target_id));
  }

  const tasks = listTasksWithDetails(db, { limit: 100000 }).tasks;
  return tasks.filter(
    (task) =>
      taskIds.has(task.id) ||
      files.has(task.symbol_file) ||
      (task.flow_id !== null && flowIds.has(task.flow_id)),
  );
}

export function getFeatureTaskStats(
  db: Database.Database,
  featureId: number,
): { total: number; done: number; blocked: number; by_status: { status: string; count: number }[] } {
  const tasks = getFeatureTasks(db, featureId);
  const counts = new Map<string, number>();
  for (const task of tasks) counts.set(task.status, (counts.get(task.status) ?? 0) + 1);
  return {
    total: tasks.length,
    done: counts.get("done") ?? 0,
    blocked: counts.get("blocked") ?? 0,
    by_status: [...counts.entries()].map(([status, count]) => ({ status, count })),
  };
}
