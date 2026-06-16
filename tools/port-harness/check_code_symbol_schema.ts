import Database from 'better-sqlite3';
const db = new Database('port_harness.db');
const columns = db.prepare("PRAGMA table_info(code_symbol)").all() as any[];
console.log(columns.map(c => c.name).join(', '));

// Check a few entries
const rows = db.prepare("SELECT * FROM code_symbol WHERE file LIKE '%Item.cpp%' LIMIT 5").all() as any[];
console.log('\nSample entries:');
for (const r of rows) {
  console.log(`  ${r.id}: ${r.name} (${r.file})`);
}
