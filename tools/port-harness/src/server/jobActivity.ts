import type Database from "better-sqlite3";

const MAX_LOG_LINES = 200;

export interface JobActivity {
  log(message: string): void;
  setCurrent(item: string | null): void;
}

export function createJobActivity(db: Database.Database, jobId: number): JobActivity {
  return {
    log(message: string) {
      appendJobLog(db, jobId, message);
    },
    setCurrent(item: string | null) {
      db.prepare("UPDATE pipeline_job SET current_item = ? WHERE id = ?").run(item, jobId);
    },
  };
}

export function appendJobLog(db: Database.Database, jobId: number, message: string): void {
  const row = db.prepare("SELECT log FROM pipeline_job WHERE id = ?").get(jobId) as
    | { log: string | null }
    | undefined;
  const ts = new Date().toISOString().slice(11, 19);
  const lines = (row?.log ?? "").split("\n").filter(Boolean);
  lines.push(`[${ts}] ${message}`);
  const trimmed = lines.slice(-MAX_LOG_LINES).join("\n");
  db.prepare("UPDATE pipeline_job SET log = ? WHERE id = ?").run(trimmed, jobId);
  console.log(`[job ${jobId}] ${message}`);
}
