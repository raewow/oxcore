CREATE TABLE `web_sessions` (
    `token_hash` BINARY(32) NOT NULL COMMENT 'SHA-256 of the opaque browser cookie token',
    `account_id` INT UNSIGNED NOT NULL,
    `created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `last_seen_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `expires_at` TIMESTAMP NOT NULL,
    PRIMARY KEY (`token_hash`),
    KEY `idx_web_sessions_account` (`account_id`),
    KEY `idx_web_sessions_expiry` (`expires_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `web_identity_tokens` (
    `token_hash` BINARY(32) NOT NULL COMMENT 'SHA-256 of a one-time email token',
    `account_id` INT UNSIGNED NOT NULL,
    `purpose` ENUM('email_verification', 'password_reset') NOT NULL,
    `created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `expires_at` TIMESTAMP NOT NULL,
    `consumed_at` TIMESTAMP NULL DEFAULT NULL,
    PRIMARY KEY (`token_hash`),
    KEY `idx_web_identity_tokens_account` (`account_id`, `purpose`),
    KEY `idx_web_identity_tokens_expiry` (`expires_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `web_audit_log` (
    `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    `occurred_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `actor_account_id` INT UNSIGNED DEFAULT NULL,
    `action` VARCHAR(96) NOT NULL,
    `target_type` VARCHAR(64) NOT NULL,
    `target_id` VARCHAR(128) DEFAULT NULL,
    `reason` VARCHAR(1024) DEFAULT NULL,
    `request_id` CHAR(36) DEFAULT NULL,
    `remote_ip` VARCHAR(45) DEFAULT NULL,
    `details` JSON DEFAULT NULL,
    PRIMARY KEY (`id`),
    KEY `idx_web_audit_actor_time` (`actor_account_id`, `occurred_at`),
    KEY `idx_web_audit_target_time` (`target_type`, `target_id`, `occurred_at`),
    KEY `idx_web_audit_action_time` (`action`, `occurred_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
