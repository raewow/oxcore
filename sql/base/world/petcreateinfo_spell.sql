-- MySQL dump
--
-- Table structure for table `petcreateinfo_spell`
-- Table data for table `petcreateinfo_spell`
--

DROP TABLE IF EXISTS `petcreateinfo_spell`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `petcreateinfo_spell` (
  `entry` mediumint unsigned NOT NULL DEFAULT '0',
  `spell1` smallint unsigned NOT NULL DEFAULT '0',
  `spell2` smallint unsigned NOT NULL DEFAULT '0',
  `spell3` smallint unsigned NOT NULL DEFAULT '0',
  `spell4` smallint unsigned NOT NULL DEFAULT '0',
  `patch_min` tinyint unsigned NOT NULL DEFAULT '0' COMMENT 'Minimum content patch to load this entry',
  `patch_max` tinyint unsigned NOT NULL DEFAULT '10' COMMENT 'Maximum content patch to load this entry',
  PRIMARY KEY (`entry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Pet Create Spells';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `petcreateinfo_spell`
--

LOCK TABLES `petcreateinfo_spell` WRITE;
/*!40000 ALTER TABLE `petcreateinfo_spell` DISABLE KEYS */;
INSERT INTO `petcreateinfo_spell` (`entry`, `spell1`, `spell2`, `spell3`, `spell4`, `patch_min`, `patch_max`) VALUES
(416, 3110, 0, 0, 0, 0, 10),
(417, 19505, 0, 0, 0, 0, 10),
(510, 6873, 9672, 0, 0, 0, 10),
(1860, 3716, 0, 0, 0, 0, 10),
(1863, 7814, 0, 0, 0, 0, 10),
(5807, 17254, 0, 0, 0, 0, 10),
(15429, 25163, 0, 0, 0, 7, 10);
/*!40000 ALTER TABLE `petcreateinfo_spell` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;