import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").all();
console.log('Tables:', tables.map((r: any) => r.name).join(', '));

const schema = db.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='feature_assignment'").get() as any;
console.log('---');
console.log(schema.sql);

const feature = db.prepare("SELECT * FROM feature_group WHERE name = 'inventory'").get() as any;
console.log('---');
console.log('Feature:', JSON.stringify(feature, null, 2));

const assignments = db.prepare("SELECT * FROM feature_assignment WHERE feature_id = ?").all(feature.id);
console.log('---');
console.log('Assignments:', JSON.stringify(assignments, null, 2));
