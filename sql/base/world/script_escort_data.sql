-- MySQL dump
--
-- Table structure for table `script_escort_data`
-- Table data for table `script_escort_data`
--

DROP TABLE IF EXISTS `script_escort_data`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `script_escort_data` (
  `creature_id` int DEFAULT NULL,
  `quest` int DEFAULT NULL,
  `escort_faction` int DEFAULT NULL,
  UNIQUE KEY `creature_id` (`creature_id`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1 ROW_FORMAT=DYNAMIC;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `script_escort_data`
--

LOCK TABLES `script_escort_data` WRITE;
/*!40000 ALTER TABLE `script_escort_data` DISABLE KEYS */;
INSERT INTO `script_escort_data` (`creature_id`, `quest`, `escort_faction`) VALUES
(9023, 4322, 11);
/*!40000 ALTER TABLE `script_escort_data` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;