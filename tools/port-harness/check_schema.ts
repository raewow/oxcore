import Database from 'better-sqlite3';
const db = new Database('port_harness.db');
const columns = db.prepare("PRAGMA table_info(migration_task)").all() as any[];
console.log(columns.map(c => c.name).join(', '));
