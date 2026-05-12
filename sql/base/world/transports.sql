-- MySQL dump
--
-- Table structure for table `transports`
-- Table data for table `transports`
--

DROP TABLE IF EXISTS `transports`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `transports` (
  `entry` mediumint unsigned NOT NULL DEFAULT '0',
  `build` smallint unsigned NOT NULL DEFAULT '0',
  `name` text,
  `period` mediumint unsigned NOT NULL DEFAULT '0',
  PRIMARY KEY (`entry`,`build`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=FIXED COMMENT='Transports';
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `transports`
--

LOCK TABLES `transports` WRITE;
/*!40000 ALTER TABLE `transports` DISABLE KEYS */;
INSERT INTO `transports` (`entry`, `build`, `name`, `period`) VALUES
(20808, 0, 'Ratchet and Booty Bay', 339575),
(20808, 4695, 'Ratchet and Booty Bay', 350818),
(164871, 0, 'Orgrimmar and Undercity', 360016),
(164871, 4695, 'Orgrimmar and Undercity', 356284),
(175080, 0, 'Grom\'Gol Base Camp and Orgrimmar', 303463),
(176231, 0, 'Menethil Harbor and Theramore Isle', 311153),
(176231, 4695, 'Menethil Harbor and Theramore Isle', 329313),
(176244, 0, 'Teldrassil and Auberdine', 316251),
(176310, 0, 'Menethil Harbor and Auberdine', 283065),
(176310, 4695, 'Menethil Harbor and Auberdine', 295579),
(176495, 0, 'Grom\'Gol Base Camp and Undercity', 333044),
(177233, 0, 'Forgotton Coast and Feathermoon Stronghold', 299437),
(177233, 5464, 'Forgotton Coast and Feathermoon Stronghold', 317040),
(181056, 5464, 'Naxxramas', 1208014);
/*!40000 ALTER TABLE `transports` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;