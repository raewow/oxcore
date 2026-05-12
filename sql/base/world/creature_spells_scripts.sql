-- MySQL dump
--
-- Table structure for table `creature_spells_scripts`
-- Table data for table `creature_spells_scripts`
--

DROP TABLE IF EXISTS `creature_spells_scripts`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `creature_spells_scripts` (
  `id` int unsigned NOT NULL DEFAULT '0',
  `delay` int unsigned NOT NULL DEFAULT '0',
  `priority` tinyint unsigned NOT NULL DEFAULT '0',
  `command` tinyint unsigned NOT NULL DEFAULT '0',
  `datalong` int unsigned NOT NULL DEFAULT '0',
  `datalong2` int unsigned NOT NULL DEFAULT '0',
  `datalong3` int unsigned NOT NULL DEFAULT '0',
  `datalong4` int unsigned NOT NULL DEFAULT '0',
  `target_param1` int unsigned NOT NULL DEFAULT '0',
  `target_param2` int unsigned NOT NULL DEFAULT '0',
  `target_type` tinyint unsigned NOT NULL DEFAULT '0',
  `data_flags` tinyint unsigned NOT NULL DEFAULT '0',
  `dataint` int NOT NULL DEFAULT '0',
  `dataint2` int NOT NULL DEFAULT '0',
  `dataint3` int NOT NULL DEFAULT '0',
  `dataint4` int NOT NULL DEFAULT '0',
  `x` float NOT NULL DEFAULT '0',
  `y` float NOT NULL DEFAULT '0',
  `z` float NOT NULL DEFAULT '0',
  `o` float NOT NULL DEFAULT '0',
  `condition_id` mediumint unsigned NOT NULL DEFAULT '0',
  `comments` varchar(255) NOT NULL
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `creature_spells_scripts`
--

LOCK TABLES `creature_spells_scripts` WRITE;
/*!40000 ALTER TABLE `creature_spells_scripts` DISABLE KEYS */;
INSERT INTO `creature_spells_scripts` (`id`, `delay`, `priority`, `command`, `datalong`, `datalong2`, `datalong3`, `datalong4`, `target_param1`, `target_param2`, `target_type`, `data_flags`, `dataint`, `dataint2`, `dataint3`, `dataint4`, `x`, `y`, `z`, `o`, `condition_id`, `comments`) VALUES
(21147, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9071, 0, 0, 0, 0, 0, 0, 0, 0, 'Azuregos - Arcane Vacuum - Say Text'),
(7621, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6535, 0, 0, 0, 0, 0, 0, 0, 0, 'Archmage Arugal - Arugal\'s Curse - Arugal Say Text'),
(7803, 1, 0, 3, 2, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 'Archmage Arugal - Thundershock - Move Away from Target'),
(19096, 0, 0, 39, 19096, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 'Nathanos Blightcaller - 50% Chance to Yell Text'),
(3019, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7798, 0, 0, 0, 0, 0, 0, 0, 0, 'Blackrock Scout - Say Text on Enrage'),
(26381, 0, 0, 29, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -100, 0, 0, 0, 0, 'Hive\'Zara Sandstalker - Reset Threat'),
(26381, 1, 0, 26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Hive\'Zara Sandstalker - Start Attack'),
(17473, 0, 0, 15, 17475, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Cast Raise Dead 1'),
(17473, 0, 0, 15, 17476, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Cast Raise Dead 2'),
(17473, 0, 0, 15, 17477, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Cast Raise Dead 3'),
(17473, 0, 0, 15, 17478, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Cast Raise Dead 4'),
(17473, 0, 0, 15, 17479, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Cast Raise Dead 5'),
(17473, 0, 0, 15, 17480, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Cast Raise Dead 6'),
(17473, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6511, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Say Text'),
(17473, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6512, 0, 0, 0, 0, 0, 0, 0, 0, 'Baron Rivendare - Say Text'),
(57640, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2077, 0, 0, 0, 0, 0, 0, 0, 0, 'Guardian of Blizzard - Say Text'),
(8646, 0, 0, 20, 19, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 2428, 'Ashenvale Outrunner - Move Away from Target'),
(12540, 0, 0, 29, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -100, 0, 0, 0, 0, 'Blackhand Assassin - Reduce Target Threat after Casting Gouge');
/*!40000 ALTER TABLE `creature_spells_scripts` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;