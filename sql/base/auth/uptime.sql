-- MySQL dump
--
-- Table structure for table `uptime`
-- Table data for table `uptime`
--

DROP TABLE IF EXISTS `uptime`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `uptime` (
  `realmid` int unsigned NOT NULL,
  `starttime` bigint unsigned NOT NULL DEFAULT '0',
  `startstring` varchar(64) NOT NULL DEFAULT '',
  `uptime` bigint unsigned NOT NULL DEFAULT '0',
  `onlineplayers` smallint unsigned NOT NULL DEFAULT '0',
  `maxplayers` smallint unsigned NOT NULL DEFAULT '0',
  `revision` varchar(255) NOT NULL DEFAULT 'VMangos',
  PRIMARY KEY (`realmid`,`starttime`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Uptime system';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `uptime`
--

LOCK TABLES `uptime` WRITE;
/*!40000 ALTER TABLE `uptime` DISABLE KEYS */;
INSERT INTO `uptime` (`realmid`, `starttime`, `startstring`, `uptime`, `onlineplayers`, `maxplayers`, `revision`) VALUES
(1, 1765795333, '2025-12-15 10:42:13', 601, 0, 0, '377121d8b3c0e2c8b1ca'),
(1, 1765796351, '2025-12-15 10:59:11', 601, 0, 0, '69c18bce39ae92487a80'),
(1, 1765797671, '2025-12-15 11:21:11', 13801, 0, 1, '69c18bce39ae92487a80'),
(1, 1765811897, '2025-12-15 15:18:17', 0, 0, 0, '81f002a0ab1f8eea3fe2'),
(1, 1765812010, '2025-12-15 15:20:10', 0, 0, 0, '81f002a0ab1f8eea3fe2'),
(1, 1765812097, '2025-12-15 15:21:37', 0, 0, 0, '81f002a0ab1f8eea3fe2'),
(1, 1765812296, '2025-12-15 15:24:56', 0, 0, 0, '81f002a0ab1f8eea3fe2'),
(1, 1765812449, '2025-12-15 15:27:29', 9001, 0, 1, '81f002a0ab1f8eea3fe2'),
(1, 1765961569, '2025-12-17 08:52:49', 80401, 0, 1, '3a4c122e8f67afc5fb95');
/*!40000 ALTER TABLE `uptime` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;