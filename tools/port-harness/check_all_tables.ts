import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").all();
console.log('Tables:', tables.map((r: any) => r.name).join(', '));

for (const t of tables) {
  const schema = db.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name = ?").get(t.name) as any;
  console.log(`\n--- ${t.name} ---`);
  console.log(schema.sql);
}
