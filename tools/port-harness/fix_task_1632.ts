import Database from 'better-sqlite3';
const db = new Database('port_harness.db');

const result = db.prepare("UPDATE migration_task SET status = 'blocked', notes = 'Requires ItemRandomProperties.dbc and ItemRandomSuffix.dbc data loading. Blocked by DBC data loading feature.' WHERE id = 1632").run();
console.log(`Updated task 1632: ${result.changes} rows`);
