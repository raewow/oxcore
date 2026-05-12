-- MySQL dump
--
-- Table structure for table `character_stats`
--

DROP TABLE IF EXISTS `character_stats`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `character_stats` (
  `guid` int unsigned NOT NULL DEFAULT '0' COMMENT 'Global Unique Identifier, Low part',
  `max_health` int unsigned NOT NULL DEFAULT '0',
  `max_power1` int unsigned NOT NULL DEFAULT '0',
  `max_power2` int unsigned NOT NULL DEFAULT '0',
  `max_power3` int unsigned NOT NULL DEFAULT '0',
  `max_power4` int unsigned NOT NULL DEFAULT '0',
  `max_power5` int unsigned NOT NULL DEFAULT '0',
  `max_power6` int unsigned NOT NULL DEFAULT '0',
  `max_power7` int unsigned NOT NULL DEFAULT '0',
  `strength` float NOT NULL DEFAULT '0',
  `agility` float NOT NULL DEFAULT '0',
  `stamina` float NOT NULL DEFAULT '0',
  `intellect` float NOT NULL DEFAULT '0',
  `spirit` float NOT NULL DEFAULT '0',
  `armor` int NOT NULL DEFAULT '0',
  `holy_res` int NOT NULL DEFAULT '0',
  `fire_res` int NOT NULL DEFAULT '0',
  `nature_res` int NOT NULL DEFAULT '0',
  `frost_res` int NOT NULL DEFAULT '0',
  `shadow_res` int NOT NULL DEFAULT '0',
  `arcane_res` int NOT NULL DEFAULT '0',
  `block_chance` float NOT NULL DEFAULT '0',
  `dodge_chance` float NOT NULL DEFAULT '0',
  `parry_chance` float NOT NULL DEFAULT '0',
  `crit_chance` float NOT NULL DEFAULT '0',
  `ranged_crit_chance` float NOT NULL DEFAULT '0',
  `spell_crit_chance` float NOT NULL DEFAULT '0',
  `attack_power` int unsigned NOT NULL DEFAULT '0',
  `ranged_attack_power` int unsigned NOT NULL DEFAULT '0',
  `spell_damage` int unsigned NOT NULL DEFAULT '0',
  `spell_healing` int unsigned NOT NULL DEFAULT '0',
  PRIMARY KEY (`guid`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;