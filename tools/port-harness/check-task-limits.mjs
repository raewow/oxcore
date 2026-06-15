import Database from 'better-sqlite3';

try {
  const db = new Database('port_harness.db');
  
  const total = db.prepare('SELECT COUNT(*) as c FROM migration_task').get().c;
  console.log('Total tasks:', total);
  
  const tasks = db.prepare(`SELECT mt.id, mt.flow_id, cs.file, cs.start_line, cs.name 
    FROM migration_task mt 
    JOIN code_symbol cs ON cs.id = mt.source_symbol_id 
    ORDER BY cs.file, cs.start_line 
    LIMIT 1000`).all();
  
  console.log('Tasks in first 1000:', tasks.length);
  
  const flow60Tasks = tasks.filter(t => t.flow_id === 60);
  console.log('Flow 60 tasks in first 1000:', flow60Tasks.length);
  
  const allFlow60Tasks = db.prepare('SELECT COUNT(*) as c FROM migration_task WHERE flow_id = 60').get().c;
  console.log('All flow 60 tasks:', allFlow60Tasks);
  
  // Check if flow 60 tasks are beyond the first 1000
  const flow60TaskPositions = db.prepare(`SELECT mt.id, mt.flow_id, cs.file, cs.start_line, cs.name 
    FROM migration_task mt 
    JOIN code_symbol cs ON cs.id = mt.source_symbol_id 
    WHERE mt.flow_id = 60 
    ORDER BY cs.file, cs.start_line`).all();
  
  console.log('Flow 60 tasks positions:');
  for (const t of flow60TaskPositions) {
    console.log('  ', t.id, t.file, t.start_line, t.name);
  }
  
  db.close();
} catch (e) {
  console.error('Error:', e.message);
}
