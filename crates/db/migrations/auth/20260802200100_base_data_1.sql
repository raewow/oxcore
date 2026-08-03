-- PostgreSQL migration: auth / base_data_1
-- Checked-in base data.

SET LOCAL standard_conforming_strings = on;

-- Client builds allowed to authenticate. The legacy realm accept loop
-- (crates/auth/src/server.rs) refuses to start at all if this table is
-- empty, so at least one row is required for the auth server to come up.
INSERT INTO "auth"."allowed_clients" ("major_version", "minor_version", "bugfix_version", "hotfix_version", "build", "os", "platform", "integrity_hash") VALUES
(2, 5, 3, 'a', 42597, 'Win', 'x86', ''),
(2, 5, 3, 'a', 42597, 'OSX', 'x86', '');

-- The default realm entry. World::start() only UPDATEs this row by id (it never
-- INSERTs), so a row matching config.toml's realm_id (default 1) must already
-- exist or the realm silently never goes online ("No realm found with id=1").
-- Table is empty at this point so the identity column naturally assigns id=1.
INSERT INTO "auth"."realmlist" ("name") VALUES
('oxcore');
