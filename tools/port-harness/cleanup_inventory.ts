import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const feature = db.prepare("SELECT * FROM feature_group WHERE name = 'inventory'").get() as any;
const featureId = feature.id;

// Inventory-related files (keep these)
const inventoryFiles = [
  'src/game/Handlers/ItemHandler.cpp',
  'src/game/Objects/Bag.cpp',
  'src/game/Objects/Item.cpp',
  'src/game/Objects/ItemDefines.h',
  'src/game/Objects/ItemPrototype.h',
];

// Specific inventory-related tasks from Player.cpp that we want to keep
const keepTaskIds = new Set([
  '2233', '2232', '2222', '2225', '2218', '2215', '2205', '2201', '2207', '2206',
  '2354', '2383', '2379', '2178', '2180', '2191', '2275', '2224', '2223', '1937',
  '2573', '2257', '2239', '2154',
  // Item/Bag tasks we already updated
  '1615', '1616', '1621', '1622', '1625', '1627', '1628', '1643', '1650',
  '1276', '1282', '1284', '1286', '1291', '1290', '1289', '1294',
  // Creature vendor tasks
  '1443', '1444', '1445', '1446',
]);

// Step 1: Delete file assignments for non-inventory files
const fileDelete = db.prepare(`
  DELETE FROM feature_assignment
  WHERE feature_id = ? AND target_type = 'file'
  AND target_id NOT IN (${inventoryFiles.map(() => '?').join(',')})
`);
const fileDeleted = fileDelete.run(featureId, ...inventoryFiles);
console.log(`Deleted ${fileDeleted.changes} non-inventory file assignments`);

// Step 2: Find all task assignments for this feature
const taskAssignments = db.prepare(`
  SELECT target_id FROM feature_assignment
  WHERE feature_id = ? AND target_type = 'task'
`).all(featureId) as any[];

// Step 3: Find which tasks belong to which files
const taskIds = taskAssignments.map(t => t.target_id);
console.log(`Found ${taskIds.length} task assignments`);

// Step 4: Find tasks that are in inventory files or in our keep list
const batchSize = 100;
let tasksToDelete = 0;
let tasksToKeep = 0;

for (let i = 0; i < taskIds.length; i += batchSize) {
  const batch = taskIds.slice(i, i + batchSize);
  const placeholders = batch.map(() => '?').join(',');

  const tasks = db.prepare(`
    SELECT mt.id, cs.file
    FROM migration_task mt
    JOIN code_symbol cs ON mt.source_symbol_id = cs.id
    WHERE mt.id IN (${placeholders})
  `).all(...batch) as any[];

  for (const task of tasks) {
    const taskId = String(task.id);
    const file = task.file;
    const isInventoryFile = inventoryFiles.some(f => file.includes(f));
    const isKeepList = keepTaskIds.has(taskId);

    if (isInventoryFile || isKeepList) {
      tasksToKeep++;
    } else {
      tasksToDelete++;
    }
  }
}

console.log(`Tasks to keep: ${tasksToKeep}`);
console.log(`Tasks to delete: ${tasksToDelete}`);

// Step 5: Actually delete the non-inventory tasks
for (let i = 0; i < taskIds.length; i += batchSize) {
  const batch = taskIds.slice(i, i + batchSize);
  const placeholders = batch.map(() => '?').join(',');

  const tasks = db.prepare(`
    SELECT mt.id, cs.file
    FROM migration_task mt
    JOIN code_symbol cs ON mt.source_symbol_id = cs.id
    WHERE mt.id IN (${placeholders})
  `).all(...batch) as any[];

  const deleteIds = tasks
    .filter(t => {
      const taskId = String(t.id);
      const file = t.file;
      const isInventoryFile = inventoryFiles.some(f => file.includes(f));
      const isKeepList = keepTaskIds.has(taskId);
      return !isInventoryFile && !isKeepList;
    })
    .map(t => String(t.id));

  if (deleteIds.length > 0) {
    const delPlaceholders = deleteIds.map(() => '?').join(',');
    const del = db.prepare(`
      DELETE FROM feature_assignment
      WHERE feature_id = ? AND target_type = 'task' AND target_id IN (${delPlaceholders})
    `);
    const result = del.run(featureId, ...deleteIds);
    console.log(`Deleted ${result.changes} tasks in batch ${i}`);
  }
}

// Step 6: Verify
const remaining = db.prepare("SELECT COUNT(*) as c FROM feature_assignment WHERE feature_id = ? AND target_type = 'task'").get(featureId) as any;
console.log(`Remaining task assignments: ${remaining.c}`);

const remainingFiles = db.prepare("SELECT COUNT(*) as c FROM feature_assignment WHERE feature_id = ? AND target_type = 'file'").get(featureId) as any;
console.log(`Remaining file assignments: ${remainingFiles.c}`);

// Show remaining files
const files = db.prepare("SELECT target_id FROM feature_assignment WHERE feature_id = ? AND target_type = 'file'").all(featureId) as any[];
console.log('Remaining files:', files.map(f => f.target_id).join(', '));
