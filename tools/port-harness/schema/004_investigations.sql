CREATE TABLE IF NOT EXISTS investigation (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  query TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'done', 'failed')),
  seed_json TEXT,
  result_json TEXT,
  job_id INTEGER REFERENCES pipeline_job(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_investigation_status ON investigation(status);
CREATE INDEX IF NOT EXISTS idx_investigation_created ON investigation(created_at);
