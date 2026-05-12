-- MySQL dump
--
-- Table structure for table `battleground_template`
-- Table data for table `battleground_template`
--

DROP TABLE IF EXISTS `battleground_template`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `battleground_template` (
  `id` mediumint unsigned NOT NULL,
  `patch` tinyint unsigned NOT NULL DEFAULT '0' COMMENT 'Content patch in which this exact version of the entry was added',
  `min_players_per_team` smallint unsigned NOT NULL DEFAULT '0',
  `max_players_per_team` smallint unsigned NOT NULL DEFAULT '0',
  `min_level` tinyint unsigned NOT NULL DEFAULT '0',
  `max_level` tinyint unsigned NOT NULL DEFAULT '0',
  `alliance_win_spell` smallint unsigned NOT NULL DEFAULT '0',
  `alliance_lose_spell` smallint unsigned NOT NULL DEFAULT '0',
  `horde_win_spell` smallint unsigned NOT NULL DEFAULT '0',
  `horde_lose_spell` smallint unsigned NOT NULL DEFAULT '0',
  `alliance_start_location` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'WorldSafeLocs.dbc',
  `horde_start_location` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'WorldSafeLocs.dbc',
  `player_loot_id` mediumint unsigned NOT NULL DEFAULT '0' COMMENT 'entry from reference_loot_template',
  PRIMARY KEY (`id`,`patch`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `battleground_template`
--

LOCK TABLES `battleground_template` WRITE;
/*!40000 ALTER TABLE `battleground_template` DISABLE KEYS */;
INSERT INTO `battleground_template` (`id`, `patch`, `min_players_per_team`, `max_players_per_team`, `min_level`, `max_level`, `alliance_win_spell`, `alliance_lose_spell`, `horde_win_spell`, `horde_lose_spell`, `alliance_start_location`, `horde_start_location`, `player_loot_id`) VALUES
(1, 0, 20, 40, 61, 61, 0, 0, 0, 0, 611, 610, 0),
(2, 0, 4, 10, 61, 61, 0, 0, 0, 0, 769, 770, 0),
(3, 0, 6, 15, 61, 61, 0, 0, 0, 0, 890, 889, 0),
(2, 3, 4, 10, 21, 60, 23661, 0, 23702, 0, 769, 770, 0),
(1, 3, 30, 40, 51, 60, 0, 0, 0, 0, 611, 610, 1),
(3, 5, 6, 15, 20, 60, 24017, 0, 24017, 0, 890, 889, 0),
(2, 5, 4, 10, 20, 60, 23661, 24637, 23702, 24637, 769, 770, 0),
(2, 6, 4, 10, 10, 60, 24951, 24950, 24951, 24950, 769, 770, 0),
(1, 6, 20, 40, 51, 60, 24955, 24954, 24955, 24954, 611, 610, 1),
(3, 6, 6, 15, 20, 60, 24953, 24952, 24953, 24952, 890, 889, 0);
/*!40000 ALTER TABLE `battleground_template` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;