-- MySQL dump
--
-- Table structure for table `chat_outbox`
--
-- GM messages requested from the web admin panel. The web server inserts a row here
-- (choosing an online character of the acting GM account as the in-game sender); the
-- world server polls pending rows and delivers the message through ChatSystem.
--

DROP TABLE IF EXISTS `chat_outbox`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
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
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;
