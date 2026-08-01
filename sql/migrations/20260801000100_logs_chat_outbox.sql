CREATE TABLE `chat_outbox` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `status` enum('pending','sent','failed') NOT NULL DEFAULT 'pending',
  `sender_account` int unsigned NOT NULL,
  `sender_guid` int unsigned NOT NULL,
  `channel_type` varchar(16) NOT NULL DEFAULT 'Whisper',
  `channel_name` varchar(64) DEFAULT NULL,
  `target_guid` int unsigned DEFAULT NULL,
  `target_name` varchar(32) DEFAULT NULL,
  `message` varchar(512) NOT NULL,
  `processed_at` timestamp NULL DEFAULT NULL,
  `error` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_chat_outbox_status` (`status`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 ROW_FORMAT=DYNAMIC COMMENT='GM chat messages awaiting delivery by the world server';
