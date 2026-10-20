-- PostgreSQL migration: world / varchar_template_names
-- Created: 20261020084858
--
-- These were fixed-width CHAR columns, which PostgreSQL returns padded with spaces to their full
-- width: creature names reached the client 100 characters long, and script names failed to match.

ALTER TABLE world.creature_template
    ALTER COLUMN name TYPE VARCHAR(100) USING rtrim(name),
    ALTER COLUMN subname TYPE VARCHAR(100) USING rtrim(subname),
    ALTER COLUMN ai_name TYPE VARCHAR(64) USING rtrim(ai_name),
    ALTER COLUMN script_name TYPE VARCHAR(64) USING rtrim(script_name);

ALTER TABLE world.scripted_event_id
    ALTER COLUMN script_name TYPE VARCHAR(64) USING rtrim(script_name);
