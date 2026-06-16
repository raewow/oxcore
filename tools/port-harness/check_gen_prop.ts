import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const remaining = db.prepare("SELECT mt.id, cs.name, cs.file, mt.status FROM migration_task mt JOIN code_symbol cs ON mt.source_symbol_id = cs.id WHERE cs.name = 'Item::GenerateItemRandomPropertyId'").all() as any[];
console.log('Item::GenerateItemRandomPropertyId tasks:');
for (const t of remaining) {
  console.log(`  ${t.id}: ${t.name} (${t.file}) status=${t.status}`);
}
