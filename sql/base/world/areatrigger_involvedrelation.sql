-- MySQL dump
--
-- Table structure for table `areatrigger_involvedrelation`
-- Table data for table `areatrigger_involvedrelation`
--

DROP TABLE IF EXISTS `areatrigger_involvedrelation`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `areatrigger_involvedrelation` (
  `id` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'Identifier',
  `quest` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'Quest Identifier',
  PRIMARY KEY (`id`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=FIXED COMMENT='Trigger System';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `areatrigger_involvedrelation`
--

LOCK TABLES `areatrigger_involvedrelation` WRITE;
/*!40000 ALTER TABLE `areatrigger_involvedrelation` DISABLE KEYS */;
INSERT INTO `areatrigger_involvedrelation` (`id`, `quest`) VALUES
(2946, 6421),
(3366, 6025),
(2327, 4842),
(2486, 4811),
(1205, 2989),
(482, 1699),
(362, 1448),
(231, 984),
(230, 954),
(223, 944),
(216, 870),
(196, 578),
(97, 287),
(98, 201),
(78, 155),
(178, 503),
(87, 76),
(88, 62),
(3986, 8286),
(1387, 3505),
(175, 455),
(246, 1149),
(232, 984),
(235, 984),
(2926, 25),
(522, 1719),
(197, 62),
(342, 76),
(2206, 5156),
(2207, 5156),
(2208, 5156),
(822, 2240),
(173, 437),
(3991, 1658),
(2726, 6185),
(4101, 9263),
(4103, 9264),
(4100, 9265),
(224, 944),
(225, 944),
(4092, 9260),
(4094, 9260),
(4095, 9260),
(4096, 9260),
(4098, 9261),
(4099, 9261),
(4104, 9262),
(4105, 9262),
(3367, 6025),
(4102, 9263);
/*!40000 ALTER TABLE `areatrigger_involvedrelation` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;