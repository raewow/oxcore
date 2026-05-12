-- MySQL dump
--
-- Table structure for table `logs_battleground`
--

DROP TABLE IF EXISTS `logs_battleground`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `logs_battleground` (
  `time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `bgid` int DEFAULT NULL,
  `bgtype` int DEFAULT NULL,
  `bgteamcount` int DEFAULT NULL,
  `bgduration` int DEFAULT NULL,
  `playerGuid` int DEFAULT NULL,
  `team` int DEFAULT NULL,
  `deaths` int DEFAULT NULL,
  `honorBonus` int DEFAULT NULL,
  `honorableKills` int DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;