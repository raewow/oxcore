import Database from "better-sqlite3";

const db = new Database("port_harness.db");

const running = db
  .prepare("SELECT id FROM pipeline_job WHERE status = 'running' ORDER BY id")
  .all();

// Keep only the newest running job (likely the real worker); fail the rest.
const keepId = running.length ? running[running.length - 1].id : null;

for (const row of running) {
  if (row.id === keepId) {
    console.log(`Keeping job ${row.id} as active`);
    continue;
  }
  db.prepare(
    `UPDATE pipeline_job
     SET status = 'failed', error = 'Stale — no active worker (use Continue to resume)',
         finished_at = datetime('now'), current_item = NULL
     WHERE id = ?`,
  ).run(row.id);
  console.log(`Failed stale job ${row.id}`);
}

console.log(
  JSON.stringify(
    db
      .prepare(
        "SELECT id, status, progress, total, current_item FROM pipeline_job WHERE id >= 24 ORDER BY id",
      )
      .all(),
    null,
    2,
  ),
);
