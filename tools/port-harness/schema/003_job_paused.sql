CREATE TABLE pipeline_job_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  stage TEXT NOT NULL,
  target_ids TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'paused', 'done', 'failed', 'cancelled')),
  progress INTEGER NOT NULL DEFAULT 0,
  total INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  current_item TEXT,
  log TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT
);

INSERT INTO pipeline_job_new (
  id, stage, target_ids, status, progress, total, error, current_item, log, created_at, finished_at
)
SELECT
  id, stage, target_ids, status, progress, total, error, current_item, COALESCE(log, ''), created_at, finished_at
FROM pipeline_job;

DROP TABLE pipeline_job;
ALTER TABLE pipeline_job_new RENAME TO pipeline_job;
CREATE INDEX IF NOT EXISTS idx_pipeline_job_status ON pipeline_job(status);
