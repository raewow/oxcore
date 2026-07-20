-- Migration: auth / add_bnet_srp_columns
-- Created: 20260720120000
--
-- Adds Battle.net (SRP6v2) credential columns to the shared account table so a single
-- account works from both the 1.12 vanilla client (existing v/s, SHA-1) and the 1.14.x
-- modern client (these columns, SHA-256). The bnet identity is the account username, and the
-- verifier is computed from HEX(SHA256(UPPER(username))). Salt and verifier are stored as
-- uppercase hex to match the existing v/s convention.

ALTER TABLE `account`
    ADD COLUMN `bnet_srp_version` TINYINT UNSIGNED NOT NULL DEFAULT 2
        COMMENT 'Blizzard SRP version for the bnet verifier (2 = SHA-256)',
    ADD COLUMN `bnet_salt` VARCHAR(64) DEFAULT NULL
        COMMENT 'Bnet SRP6v2 salt, 32 bytes as uppercase hex',
    ADD COLUMN `bnet_verifier` VARCHAR(512) DEFAULT NULL
        COMMENT 'Bnet SRP6v2 verifier, uppercase hex',
    ADD COLUMN `bnet_login_ticket` VARCHAR(64) DEFAULT NULL
        COMMENT 'Active bnet login ticket (OX-<40 hex>), issued by the REST login',
    ADD COLUMN `bnet_login_ticket_expiry` BIGINT NOT NULL DEFAULT 0
        COMMENT 'Unix time the login ticket expires',
    ADD KEY `idx_bnet_login_ticket` (`bnet_login_ticket`);
