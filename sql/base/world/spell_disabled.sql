-- MySQL dump
--
-- Table structure for table `spell_disabled`
-- Table data for table `spell_disabled`
--

DROP TABLE IF EXISTS `spell_disabled`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `spell_disabled` (
  `entry` smallint unsigned NOT NULL COMMENT 'Disabled spell',
  PRIMARY KEY (`entry`)
) ENGINE=MyISAM DEFAULT CHARSET=latin1;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `spell_disabled`
--

LOCK TABLES `spell_disabled` WRITE;
/*!40000 ALTER TABLE `spell_disabled` DISABLE KEYS */;
INSERT INTO `spell_disabled` (`entry`) VALUES
(21563);
/*!40000 ALTER TABLE `spell_disabled` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;