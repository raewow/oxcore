import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

// Update remaining duplicate symbols that are still discovered by task ID
const updates = [
  { id: 1617, status: 'rust_ported', notes: 'GUID creation in Rust uses ObjectGuid::new_without_entry(). File: src/game/Objects/Item.cpp' },
  { id: 1618, status: 'rust_ported', notes: 'Same as Bag::RemoveFromWorld - handled via SmsgDestroyItem packets or automatic drop when inventory cache is cleared. File: src/game/Objects/Item.cpp' },
  { id: 1619, status: 'rust_ported', notes: 'Handled by Rust ownership model - objects are dropped when no longer referenced, no explicit world removal needed. File: src/game/Objects/Item.cpp' },
  { id: 1621, status: 'rust_ported', notes: 'Replaced by deferred ops model in InventorySystem::flush_pending_ops(). DB writes are batched and collapsed. File: src/game/Objects/Item.cpp' },
  { id: 1625, status: 'rust_ported', notes: 'Implemented as repository.delete_item() in InventoryRepository which deletes from character_inventory, item_loot, and item_instance in a transaction. File: src/game/Objects/Item.cpp' },
  { id: 1649, status: 'rust_ported', notes: 'Implemented as Item::new() and Item::from_db_row() in crates/world/src/game/items/item.rs. File: src/game/Objects/Item.cpp' },
];

const stmt = db.prepare('UPDATE migration_task SET status = ?, notes = ? WHERE id = ?');
for (const update of updates) {
  const result = stmt.run(update.status, update.notes, update.id);
  console.log(`Updated task ${update.id}: ${result.changes} rows`);
}

// Check remaining discovered tasks
const remaining = db.prepare("SELECT mt.id, cs.name, cs.file FROM migration_task mt JOIN code_symbol cs ON mt.source_symbol_id = cs.id WHERE mt.status = 'discovered' AND cs.file IN ('src/game/Objects/Item.cpp', 'src/game/Objects/Bag.cpp', 'src/game/Objects/Player.cpp')").all() as any[];
console.log(`\nRemaining discovered tasks: ${remaining.length}`);
for (const t of remaining) {
  console.log(`  ${t.id}: ${t.name} (${t.file})`);
}
