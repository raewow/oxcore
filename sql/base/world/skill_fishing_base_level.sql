-- MySQL dump
--
-- Table structure for table `skill_fishing_base_level`
-- Table data for table `skill_fishing_base_level`
--

DROP TABLE IF EXISTS `skill_fishing_base_level`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `skill_fishing_base_level` (
  `entry` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'Area identifier',
  `skill` smallint NOT NULL DEFAULT '0' COMMENT 'Base skill level requirement',
  PRIMARY KEY (`entry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=FIXED COMMENT='Fishing system';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `skill_fishing_base_level`
--

LOCK TABLES `skill_fishing_base_level` WRITE;
/*!40000 ALTER TABLE `skill_fishing_base_level` DISABLE KEYS */;
INSERT INTO `skill_fishing_base_level` (`entry`, `skill`) VALUES
(1, -70),
(12, -70),
(14, -70),
(85, -70),
(141, -70),
(215, -70),
(17, -20),
(38, -20),
(40, -20),
(130, -20),
(148, -20),
(718, -20),
(719, -20),
(1519, -20),
(1537, -20),
(1581, -20),
(1637, -20),
(1638, -20),
(1657, -20),
(10, 55),
(11, 55),
(44, 55),
(267, 55),
(331, 55),
(406, 55),
(8, 130),
(15, 130),
(33, 130),
(36, 130),
(45, 130),
(400, 130),
(405, 130),
(796, 130),
(16, 205),
(28, 205),
(47, 205),
(357, 205),
(361, 205),
(440, 205),
(490, 205),
(493, 205),
(1417, 205),
(2100, 205),
(41, 330),
(46, 330),
(139, 330),
(618, 330),
(1377, 330),
(1977, 330),
(2017, 330),
(2057, 330),
(297, 205),
(1112, 330),
(1222, 330),
(1227, 330),
(3140, 330);
/*!40000 ALTER TABLE `skill_fishing_base_level` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;