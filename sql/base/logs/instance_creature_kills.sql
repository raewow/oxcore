-- MySQL dump
--
-- Table structure for table `instance_creature_kills`
--

DROP TABLE IF EXISTS `instance_creature_kills`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `instance_creature_kills` (
  `mapId` int unsigned NOT NULL COMMENT 'MapId to where creature exist',
  `creatureEntry` int unsigned NOT NULL COMMENT 'entry of the creature who performed the kill',
  `spellEntry` int NOT NULL COMMENT 'entry of spell which did the kill. 0 for melee or unknown',
  `count` int unsigned NOT NULL COMMENT 'number of kills',
  PRIMARY KEY (`mapId`,`creatureEntry`,`spellEntry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='creatures killing players statistics';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;