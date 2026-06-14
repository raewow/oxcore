CREATE TABLE IF NOT EXISTS feature_group (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS feature_assignment (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  feature_id INTEGER NOT NULL REFERENCES feature_group(id) ON DELETE CASCADE,
  target_type TEXT NOT NULL CHECK (target_type IN ('file', 'flow', 'task')),
  target_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(feature_id, target_type, target_id)
);

CREATE INDEX IF NOT EXISTS idx_feature_assignment_feature ON feature_assignment(feature_id);
CREATE INDEX IF NOT EXISTS idx_feature_assignment_target ON feature_assignment(target_type, target_id);

CREATE TABLE IF NOT EXISTS feature_suggestion (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  feature_id INTEGER NOT NULL REFERENCES feature_group(id) ON DELETE CASCADE,
  target_type TEXT NOT NULL CHECK (target_type IN ('file', 'flow', 'task')),
  target_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  confidence TEXT NOT NULL DEFAULT 'medium' CHECK (confidence IN ('high', 'medium', 'low')),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected')),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(feature_id, target_type, target_id)
);

CREATE INDEX IF NOT EXISTS idx_feature_suggestion_feature ON feature_suggestion(feature_id);
CREATE INDEX IF NOT EXISTS idx_feature_suggestion_status ON feature_suggestion(status);
