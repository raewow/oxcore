-- MySQL dump
--
-- Table structure for table `instance_buff_removal`
--

DROP TABLE IF EXISTS `instance_buff_removal`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `instance_buff_removal` (
  `map_id` int unsigned NOT NULL COMMENT 'MapId to remove aura from',
  `spell_id` smallint unsigned NOT NULL COMMENT 'aura id to remove on entering MapId',
  `enabled` tinyint(1) NOT NULL COMMENT 'aura removal enabled or not',
  `flags` int NOT NULL COMMENT 'flags, see AuraRemovalMgr.h',
  `comment` varchar(256) NOT NULL COMMENT 'description, what is removed',
  PRIMARY KEY (`map_id`,`spell_id`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Aura removal on map entry';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;