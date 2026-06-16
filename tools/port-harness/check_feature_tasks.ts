import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const feature = db.prepare("SELECT * FROM feature_group WHERE name = 'inventory'").get() as any;
const featureId = feature.id;

// Check flow assignments
const flows = db.prepare("SELECT * FROM feature_assignment WHERE feature_id = ? AND target_type = 'flow'").all(featureId) as any[];
console.log('Flow assignments:', flows.length);
for (const f of flows) {
  console.log(`  ${f.target_id}`);
}

// Check feature tasks view
const tasks = db.prepare("SELECT * FROM feature_tasks WHERE feature_id = ?").all(featureId) as any[];
console.log('Feature tasks:', tasks.length);

// Check if there are non-assigned tasks in feature_tasks
const assigned = db.prepare("SELECT target_id FROM feature_assignment WHERE feature_id = ? AND target_type = 'task'").all(featureId) as any[];
const assignedIds = new Set(assigned.map(a => a.target_id));
const nonAssigned = tasks.filter(t => !assignedIds.has(String(t.task_id)));
console.log('Non-assigned tasks in feature_tasks:', nonAssigned.length);
for (const t of nonAssigned.slice(0, 10)) {
  console.log(`  ${t.task_id} ${t.symbol_name} (${t.file})`);
}
