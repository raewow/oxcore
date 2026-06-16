import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const feature = db.prepare("SELECT * FROM feature_group WHERE name = 'inventory'").get() as any;
const featureId = feature.id;

// Check flow assignments
const flows = db.prepare("SELECT * FROM feature_assignment WHERE feature_id = ? AND target_type = 'flow'").all(featureId) as any[];
console.log('Flow assignments:', flows.length);
for (const f of flows) {
  const flow = db.prepare("SELECT * FROM business_flow WHERE id = ?").get(f.target_id) as any;
  console.log(`  ${f.target_id}: ${flow?.name || 'UNKNOWN'}`);
}
