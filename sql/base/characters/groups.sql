-- MySQL dump
--
-- Table structure for table `groups`
--

DROP TABLE IF EXISTS `groups`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `groups` (
  `group_id` int unsigned NOT NULL,
  `leader_guid` int unsigned NOT NULL,
  `main_tank_guid` int unsigned NOT NULL,
  `main_assistant_guid` int unsigned NOT NULL,
  `loot_method` tinyint unsigned NOT NULL,
  `loot_threshold` tinyint unsigned NOT NULL,
  `looter_guid` int unsigned NOT NULL,
  `icon1` int unsigned NOT NULL,
  `icon2` int unsigned NOT NULL,
  `icon3` int unsigned NOT NULL,
  `icon4` int unsigned NOT NULL,
  `icon5` int unsigned NOT NULL,
  `icon6` int unsigned NOT NULL,
  `icon7` int unsigned NOT NULL,
  `icon8` int unsigned NOT NULL,
  `is_raid` tinyint unsigned NOT NULL,
  PRIMARY KEY (`group_id`),
  UNIQUE KEY `key_leaderGuid` (`leader_guid`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=FIXED COMMENT='Groups';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;