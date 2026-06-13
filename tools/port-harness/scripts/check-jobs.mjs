import Database from "better-sqlite3";

const db = new Database("port_harness.db");
const rows = db
  .prepare(
    "SELECT id, status, progress, total, current_item, error FROM pipeline_job WHERE id >= 24 OR status = 'running' ORDER BY id",
  )
  .all();
console.log(JSON.stringify(rows, null, 2));
