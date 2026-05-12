-- MySQL dump
--
-- Table structure for table `creature_groups_entry_limit`
-- Table data for table `creature_groups_entry_limit`
--

DROP TABLE IF EXISTS `creature_groups_entry_limit`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `creature_groups_entry_limit` (
  `leader_guid` int unsigned NOT NULL,
  `creature_id` int unsigned NOT NULL,
  `min_count` int unsigned NOT NULL DEFAULT '0',
  `max_count` int unsigned NOT NULL DEFAULT '1',
  PRIMARY KEY (`leader_guid`,`creature_id`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `creature_groups_entry_limit`
--

LOCK TABLES `creature_groups_entry_limit` WRITE;
/*!40000 ALTER TABLE `creature_groups_entry_limit` DISABLE KEYS */;
INSERT INTO `creature_groups_entry_limit` (`leader_guid`, `creature_id`, `min_count`, `max_count`) VALUES
(84519, 12467, 1, 1),
(84519, 12465, 1, 2),
(84519, 12464, 1, 2),
(84519, 12463, 1, 2),
(84525, 12467, 1, 1),
(84525, 12465, 1, 2),
(84525, 12464, 1, 2),
(84525, 12463, 1, 2);
/*!40000 ALTER TABLE `creature_groups_entry_limit` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;