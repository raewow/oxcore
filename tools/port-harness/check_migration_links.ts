import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

// Check migration_task schema
const mtCols = db.prepare("PRAGMA table_info(migration_task)").all() as any[];
console.log('migration_task columns:', mtCols.map(c => c.name).join(', '));

// Check how migration_task links to code_symbol
const rows = db.prepare("SELECT * FROM migration_task WHERE source_symbol_id IN (1617, 1618, 1619, 1621, 1625, 1649)").all() as any[];
console.log('\nMigration tasks for duplicate symbols:');
for (const r of rows) {
  const symbol = db.prepare("SELECT * FROM code_symbol WHERE id = ?").get(r.source_symbol_id) as any;
  console.log(`  ${r.id}: source_symbol_id=${r.source_symbol_id} (${symbol?.name} in ${symbol?.file}) status=${r.status}`);
}
