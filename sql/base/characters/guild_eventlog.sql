-- MySQL dump
--
-- Table structure for table `guild_eventlog`
--

DROP TABLE IF EXISTS `guild_eventlog`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `guild_eventlog` (
  `guild_id` int NOT NULL COMMENT 'Guild Identificator',
  `log_guid` int NOT NULL COMMENT 'Log record identificator - auxiliary column',
  `event_type` tinyint(1) NOT NULL COMMENT 'Event type',
  `player_guid1` int NOT NULL COMMENT 'Player 1',
  `player_guid2` int NOT NULL COMMENT 'Player 2',
  `new_rank` tinyint NOT NULL COMMENT 'New rank(in case promotion/demotion)',
  `timestamp` bigint NOT NULL COMMENT 'Event UNIX time',
  PRIMARY KEY (`guild_id`,`log_guid`),
  KEY `idx_PlayerGuid1` (`player_guid1`),
  KEY `idx_PlayerGuid2` (`player_guid2`),
  KEY `idx_LogGuid` (`log_guid`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 COMMENT='Guild Eventlog';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;