ALTER TABLE pipeline_job ADD COLUMN current_item TEXT;
ALTER TABLE pipeline_job ADD COLUMN log TEXT NOT NULL DEFAULT '';
