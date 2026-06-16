import Database from 'better-sqlite3';
const db = new Database('port_harness.db');
const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").all() as any[];
console.log(tables.map(r => r.name).join(', '));
