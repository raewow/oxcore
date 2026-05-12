-- MySQL dump
--
-- Table structure for table `gameobject_requirement`
-- Table data for table `gameobject_requirement`
--

DROP TABLE IF EXISTS `gameobject_requirement`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `gameobject_requirement` (
  `guid` int unsigned NOT NULL AUTO_INCREMENT COMMENT 'Global Unique Identifier',
  `reqType` int unsigned NOT NULL DEFAULT '0' COMMENT 'Gameobject Identifier',
  `reqGuid` int unsigned NOT NULL DEFAULT '0' COMMENT 'Gameobject Identifier',
  PRIMARY KEY (`guid`)
) ENGINE=MyISAM AUTO_INCREMENT=397162 DEFAULT CHARSET=utf8mb3 ROW_FORMAT=FIXED COMMENT='Gameobject System';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `gameobject_requirement`
--

LOCK TABLES `gameobject_requirement` WRITE;
/*!40000 ALTER TABLE `gameobject_requirement` DISABLE KEYS */;
INSERT INTO `gameobject_requirement` (`guid`, `reqType`, `reqGuid`) VALUES
(43121, 1, 15536),
(43126, 1, 35801),
(43127, 1, 15365),
(43128, 1, 15364),
(43135, 1, 15363),
(43123, 1, 39924),
(362149, 1, 43094),
(43137, 1, 43094),
(43134, 1, 15646),
(43120, 1, 17904),
(43136, 1, 15329),
(43129, 1, 15330),
(43124, 1, 15331),
(43122, 1, 15306),
(43125, 1, 35864),
(43219, 1, 43178),
(27871, 0, 52161),
(397160, 0, 56942),
(397161, 0, 56942);
/*!40000 ALTER TABLE `gameobject_requirement` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;