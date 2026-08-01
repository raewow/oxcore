-- MySQL dump
--
-- Table structure for table `chat_log`
--
-- One row per chat message sent on the server (say/yell/whisper/emote, party/raid,
-- guild/officer and custom channels). Mirrors the vmangos `logs_player` chat lineage
-- but keeps the chat attributes in structured columns so the web admin panel can
-- filter by channel, player or account without parsing free-form text.
--

DROP TABLE IF EXISTS `chat_log`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `chat_log` (
  `id` bigint unsigned NOT NULL AUTO_INCREMENT,
  `time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `channel_type` varchar(16) NOT NULL,
  `channel_name` varchar(64) DEFAULT NULL,
  `sender_guid` int unsigned DEFAULT NULL,
  `sender_name` varchar(32) DEFAULT NULL,
  `sender_account` int unsigned DEFAULT NULL,
  `target_guid` int unsigned DEFAULT NULL,
  `target_name` varchar(32) DEFAULT NULL,
  `message` varchar(512) NOT NULL,
  `map` int unsigned DEFAULT NULL,
  `pos_x` float DEFAULT NULL,
  `pos_y` float DEFAULT NULL,
  `pos_z` float DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_chat_log_time` (`time`),
  KEY `idx_chat_log_sender` (`sender_guid`),
  KEY `idx_chat_log_sender_account` (`sender_account`),
  KEY `idx_chat_log_channel` (`channel_type`, `channel_name`),
  KEY `idx_chat_log_target` (`target_guid`),
  KEY `idx_chat_log_sender_time` (`sender_guid`, `time`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 ROW_FORMAT=DYNAMIC COMMENT='chat messages';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;
