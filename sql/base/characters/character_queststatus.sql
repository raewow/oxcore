-- MySQL dump
--
-- Table structure for table `character_queststatus`
--

DROP TABLE IF EXISTS `character_queststatus`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `character_queststatus` (
  `guid` int unsigned NOT NULL DEFAULT '0' COMMENT 'Global Unique Identifier',
  `quest` int unsigned NOT NULL DEFAULT '0' COMMENT 'Quest Identifier',
  `status` int unsigned NOT NULL DEFAULT '0',
  `rewarded` tinyint unsigned NOT NULL DEFAULT '0',
  `explored` tinyint unsigned NOT NULL DEFAULT '0',
  `timer` bigint unsigned NOT NULL DEFAULT '0',
  `mob_count1` int unsigned NOT NULL DEFAULT '0',
  `mob_count2` int unsigned NOT NULL DEFAULT '0',
  `mob_count3` int unsigned NOT NULL DEFAULT '0',
  `mob_count4` int unsigned NOT NULL DEFAULT '0',
  `item_count1` int unsigned NOT NULL DEFAULT '0',
  `item_count2` int unsigned NOT NULL DEFAULT '0',
  `item_count3` int unsigned NOT NULL DEFAULT '0',
  `item_count4` int unsigned NOT NULL DEFAULT '0',
  `reward_choice` int unsigned NOT NULL DEFAULT '0',
  PRIMARY KEY (`guid`,`quest`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Player System';
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;