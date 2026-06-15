import Database from 'better-sqlite3';

try {
  const db = new Database('port_harness.db');
  
  const cols = db.prepare('PRAGMA table_info(migration_task)').all();
  console.log('migration_task columns:', cols.map(c => c.name).join(', '));
  
  const tasks = db.prepare('SELECT id, source_symbol_id, flow_id, status FROM migration_task WHERE source_symbol_id IN (3171,3169,3173,3176,3177)').all();
  console.log('Tasks found for flow 60 symbols:', tasks.length);
  for (const t of tasks) {
    console.log('Task', t.id, 'symbol', t.source_symbol_id, 'flow', t.flow_id, 'status', t.status);
  }
  
  // Check all tasks for flow 60
  const flowTasks = db.prepare('SELECT id, source_symbol_id, flow_id, status FROM migration_task WHERE flow_id = 60').all();
  console.log('Tasks linked to flow 60:', flowTasks.length);
  for (const t of flowTasks.slice(0, 5)) {
    console.log('Task', t.id, 'symbol', t.source_symbol_id, 'flow', t.flow_id, 'status', t.status);
  }
  
  db.close();
} catch (e) {
  console.error('Error:', e.message);
}
