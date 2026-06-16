import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const feature = db.prepare("SELECT * FROM feature_group WHERE name = 'inventory'").get() as any;
const featureId = feature.id;

const tasks = db.prepare(`
  SELECT mt.id, cs.name, cs.file, mt.status
  FROM feature_assignment fa
  JOIN migration_task mt ON fa.target_id = mt.id
  JOIN code_symbol cs ON mt.source_symbol_id = cs.id
  WHERE fa.feature_id = ? AND fa.target_type = 'task'
  ORDER BY cs.file, cs.name
`).all(featureId) as any[];

console.log(`Remaining tasks: ${tasks.length}`);

for (const t of tasks) {
  console.log(`${t.id}\t${t.status}\t${t.file}\t${t.name}`);
}
