-- MySQL dump
--
-- Table structure for table `character_talent`
--

DROP TABLE IF EXISTS `character_talent`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `character_talent` (
  `guid` int unsigned NOT NULL COMMENT 'Global Unique Identifier',
  `talent_id` int unsigned NOT NULL COMMENT 'Talent Identifier',
  `current_rank` tinyint unsigned NOT NULL DEFAULT '0' COMMENT 'Current talent rank (0-5)',
  PRIMARY KEY (`guid`,`talent_id`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3 ROW_FORMAT=DYNAMIC COMMENT='Player System';
/*!40101 SET character_set_client = @saved_cs_client */;

