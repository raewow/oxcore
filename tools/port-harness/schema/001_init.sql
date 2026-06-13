CREATE TABLE IF NOT EXISTS code_symbol (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('function', 'class', 'struct', 'enum', 'method', 'macro', 'chunk')),
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  parent_symbol_id INTEGER REFERENCES code_symbol(id),
  summary TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(file, name, start_line)
);

CREATE INDEX IF NOT EXISTS idx_code_symbol_file ON code_symbol(file);
CREATE INDEX IF NOT EXISTS idx_code_symbol_name ON code_symbol(name);
CREATE INDEX IF NOT EXISTS idx_code_symbol_parent ON code_symbol(parent_symbol_id);

CREATE TABLE IF NOT EXISTS symbol_call (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  caller_id INTEGER NOT NULL REFERENCES code_symbol(id) ON DELETE CASCADE,
  callee_name TEXT NOT NULL,
  callee_file TEXT,
  line INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_symbol_call_caller ON symbol_call(caller_id);

CREATE TABLE IF NOT EXISTS behaviour_claim (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol_id INTEGER NOT NULL REFERENCES code_symbol(id) ON DELETE CASCADE,
  category TEXT NOT NULL CHECK (category IN ('input', 'output', 'branch', 'side_effect', 'assumption', 'danger', 'unknown')),
  claim_text TEXT NOT NULL,
  file TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  confidence TEXT NOT NULL DEFAULT 'high' CHECK (confidence IN ('high', 'medium', 'low')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_behaviour_claim_symbol ON behaviour_claim(symbol_id);

CREATE TABLE IF NOT EXISTS business_flow (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  entry_symbol_ids TEXT NOT NULL DEFAULT '[]',
  expected_behaviour TEXT,
  risk_level TEXT NOT NULL DEFAULT 'medium' CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
  source_file TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS logic_branch (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  flow_id INTEGER NOT NULL REFERENCES business_flow(id) ON DELETE CASCADE,
  condition TEXT NOT NULL,
  behaviour TEXT NOT NULL,
  file TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS state_mutation (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  flow_id INTEGER NOT NULL REFERENCES business_flow(id) ON DELETE CASCADE,
  variable_or_field TEXT NOT NULL,
  mutation_description TEXT NOT NULL,
  file TEXT NOT NULL,
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dependency (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol_id INTEGER NOT NULL REFERENCES code_symbol(id) ON DELETE CASCADE,
  type TEXT NOT NULL CHECK (type IN ('file', 'db', 'network', 'global', 'config', 'memory')),
  description TEXT NOT NULL,
  file TEXT,
  start_line INTEGER
);

CREATE TABLE IF NOT EXISTS migration_task (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_symbol_id INTEGER NOT NULL UNIQUE REFERENCES code_symbol(id) ON DELETE CASCADE,
  flow_id INTEGER REFERENCES business_flow(id),
  target_rust_file TEXT,
  status TEXT NOT NULL DEFAULT 'discovered' CHECK (status IN (
    'discovered', 'documented', 'fixture_defined', 'rust_planned',
    'rust_ported', 'rust_compiled', 'verified', 'reviewed', 'done', 'blocked'
  )),
  notes TEXT,
  rust_symbol_name TEXT,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_migration_task_status ON migration_task(status);
CREATE INDEX IF NOT EXISTS idx_migration_task_flow ON migration_task(flow_id);

CREATE TABLE IF NOT EXISTS test_fixture (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  input_json TEXT NOT NULL,
  expected_json TEXT NOT NULL,
  covers_flow_ids TEXT NOT NULL DEFAULT '[]',
  symbol_id INTEGER REFERENCES code_symbol(id),
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'ready', 'passing', 'failing')),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_run (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  stage TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  prompt_hash TEXT NOT NULL,
  output_json TEXT NOT NULL,
  symbol_id INTEGER REFERENCES code_symbol(id),
  task_id INTEGER REFERENCES migration_task(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS pipeline_job (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  stage TEXT NOT NULL,
  target_ids TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'done', 'failed', 'cancelled')),
  progress INTEGER NOT NULL DEFAULT 0,
  total INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_pipeline_job_status ON pipeline_job(status);
