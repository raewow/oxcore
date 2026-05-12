-- MySQL dump
--
-- Table structure for table `instance_wipes`
--

DROP TABLE IF EXISTS `instance_wipes`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `instance_wipes` (
  `mapId` int unsigned NOT NULL COMMENT 'MapId to where creature exist',
  `creatureEntry` int unsigned NOT NULL COMMENT 'creature which the wipe occured against',
  `count` int unsigned NOT NULL COMMENT 'number of wipes',
  PRIMARY KEY (`mapId`,`creatureEntry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='players wiping against creatures statistics';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;