-- MySQL dump
--
-- Table structure for table `logs_player`
--

DROP TABLE IF EXISTS `logs_player`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `logs_player` (
  `id` int unsigned NOT NULL AUTO_INCREMENT,
  `time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `type` enum('Basic','WorldPacket','Chat','BG','Character','Honor','RA','DBError','DBErrorFix','ClientIds','Loot','LevelUp','Performance','MoneyTrade','GM','GMCritical','ChatSpam','Anticheat') NOT NULL,
  `subtype` varchar(20) DEFAULT NULL,
  `account` int unsigned NOT NULL,
  `ip` varchar(16) DEFAULT NULL,
  `guid` int DEFAULT NULL,
  `name` varchar(20) DEFAULT NULL,
  `map` int unsigned DEFAULT NULL,
  `pos_x` float DEFAULT NULL,
  `pos_y` float DEFAULT NULL,
  `pos_z` float DEFAULT NULL,
  `text` varchar(512) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci NOT NULL,
  PRIMARY KEY (`id`),
  KEY `account` (`account`),
  KEY `guid` (`guid`),
  KEY `name` (`name`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='player and account specific log entries';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;