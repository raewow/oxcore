-- MySQL dump
--
-- Table structure for table `smartlog_creature`
--

DROP TABLE IF EXISTS `smartlog_creature`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `smartlog_creature` (
  `time` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `type` enum('Death','LongCombat','ScriptInfo','') NOT NULL DEFAULT '',
  `entry` int NOT NULL DEFAULT '0',
  `guid` int NOT NULL DEFAULT '0',
  `specifier` varchar(255) NOT NULL DEFAULT '',
  `combatTime` int NOT NULL DEFAULT '0',
  `content` varchar(255) NOT NULL DEFAULT '',
  KEY `entry` (`entry`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;