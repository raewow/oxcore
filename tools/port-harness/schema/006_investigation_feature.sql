ALTER TABLE investigation ADD COLUMN feature_id INTEGER REFERENCES feature_group(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_investigation_feature ON investigation(feature_id);
