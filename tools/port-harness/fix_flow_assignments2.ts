import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const feature = db.prepare("SELECT * FROM feature_group WHERE name = 'inventory'").get() as any;
const featureId = feature.id;

// Remove unrelated flow assignments (target_id is TEXT)
const unrelatedFlows = ['8', '9', '12', '39', '44', '47', '50', '60', '61', '62', '63', '64'];
const del = db.prepare(`
  DELETE FROM feature_assignment
  WHERE feature_id = ? AND target_type = 'flow' AND target_id IN (${unrelatedFlows.map(() => '?').join(',')})
`);
const result = del.run(featureId, ...unrelatedFlows);
console.log(`Deleted ${result.changes} unrelated flow assignments`);

// Check remaining
const remaining = db.prepare("SELECT * FROM feature_assignment WHERE feature_id = ? AND target_type = 'flow'").all(featureId) as any[];
console.log('Remaining flows:', remaining.length);
for (const f of remaining) {
  const flow = db.prepare("SELECT name FROM business_flow WHERE id = ?").get(f.target_id) as any;
  console.log(`  ${f.target_id}: ${flow?.name}`);
}
