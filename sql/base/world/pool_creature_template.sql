-- MySQL dump
--
-- Table structure for table `pool_creature_template`
-- Table data for table `pool_creature_template`
--

DROP TABLE IF EXISTS `pool_creature_template`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `pool_creature_template` (
  `id` int unsigned NOT NULL DEFAULT '0',
  `pool_entry` smallint unsigned NOT NULL DEFAULT '0',
  `chance` float unsigned NOT NULL DEFAULT '0',
  `description` varchar(255) NOT NULL,
  `flags` int unsigned NOT NULL DEFAULT '0' COMMENT 'FLAG_SPAWN_ENABLE_IF_WORLD_POP_OVER_BLIZZLIKE = 1',
  `patch_min` tinyint unsigned NOT NULL DEFAULT '0' COMMENT 'Minimum content patch to load this entry',
  `patch_max` tinyint unsigned NOT NULL DEFAULT '10' COMMENT 'Maximum content patch to load this entry',
  PRIMARY KEY (`id`),
  KEY `pool_idx` (`pool_entry`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8mb3;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `pool_creature_template`
--

LOCK TABLES `pool_creature_template` WRITE;
/*!40000 ALTER TABLE `pool_creature_template` DISABLE KEYS */;
INSERT INTO `pool_creature_template` (`id`, `pool_entry`, `chance`, `description`, `flags`, `patch_min`, `patch_max`) VALUES
(15466, 14006, 0, 'Minion of Omen', 0, 7, 10),
(9956, 25610, 0, 'BRD - Shadowforge Flame Keeper', 0, 0, 10),
(462, 462, 0, 'Vultros (462)', 0, 0, 10),
(3735, 3735, 0, 'Apothecary Falthis (3735)', 0, 0, 10),
(448, 1270, 0, 'Elwynn Forest - Forests Edge - Hogger (448)', 0, 0, 10),
(730, 125, 0, 'Tethis (730)', 0, 0, 10),
(2038, 108, 0, 'Lord Melenas', 0, 0, 10),
(7319, 109, 0, 'Lady Sathra', 0, 0, 10),
(3467, 110, 0, 'Baron Longshore', 0, 0, 10),
(3204, 111, 0, 'Gazz\'uz', 0, 0, 10),
(2304, 112, 0, 'Captain Ironhill', 0, 0, 10),
(5822, 113, 0, 'Felweaver Scornn', 0, 0, 10),
(5808, 114, 0, 'Warlord Kolkanis', 0, 0, 10),
(5824, 115, 0, 'Captain Flat Tusk', 0, 0, 10),
(5826, 116, 0, 'Geolord Mottle', 0, 0, 10),
(5809, 117, 0, 'Watch Commander Zalaphil', 0, 0, 10),
(3392, 118, 0, 'Prospector Khazgorm', 0, 0, 10),
(5847, 119, 0, 'Heggin Stonewhisker', 0, 0, 10),
(5859, 120, 0, 'Hagg Taurenbane', 0, 0, 10),
(3472, 121, 0, 'Washte Pawne', 0, 0, 10),
(3473, 122, 0, 'Owatanka', 0, 0, 10),
(3474, 123, 0, 'Lakota\'mani', 0, 0, 10),
(14275, 124, 0, 'Tamra Stormpike', 0, 0, 10),
(521, 521, 0, 'Lupos', 0, 0, 10),
(2598, 2598, 0, 'Darbel Montrose', 0, 0, 10),
(2605, 2605, 0, 'Zalas Witherbark', 0, 0, 10),
(2606, 2606, 0, 'Nimar the Slayer', 0, 0, 10),
(2779, 2779, 0, 'Prince Nazjak', 0, 0, 10),
(2850, 2850, 0, 'Broken Tooth', 0, 0, 10),
(61, 3200, 0, 'Thuros Lightfingers', 0, 0, 10),
(7850, 9002, 0, 'Kernobee', 0, 0, 10),
(832, 10003, 0, 'Dust Devil', 0, 0, 10),
(3671, 10004, 0, 'Lady Anacondra', 0, 0, 10),
(12902, 12902, 0, 'Lorgus Jett', 0, 0, 10),
(14222, 14222, 0, 'Araga', 0, 0, 10),
(14281, 14281, 0, 'Jimmy the Bleeder', 0, 0, 10),
(10558, 25468, 25, 'Hearthsinger Forresten', 0, 0, 10),
(14231, 42939, 0, 'Drogoth the Roamer', 0, 0, 10),
(14233, 43157, 0, 'Ripscale', 0, 0, 10),
(14230, 43517, 0, 'Burgle Eye', 0, 0, 10),
(13602, 43525, 0, 'The Abominable Greench', 0, 0, 10),
(1112, 1608, 0, 'Wetlands - Leech Widow', 0, 0, 10),
(3736, 1609, 0, 'Ashenvale - Darkslayer Mordenthal', 0, 0, 10),
(4202, 1610, 0, 'Stonetalon Mountains - Gerenzo Wrenchwhistle', 0, 0, 10),
(5350, 1604, 0, 'Feralas - Qirot', 0, 0, 10),
(5863, 1611, 0, 'Barrens - Geopriest Gukk\'rok', 0, 0, 10),
(7057, 1605, 0, 'Uldaman (Outside) - Digmaster Shovelphlange', 0, 0, 10),
(7895, 1606, 0, 'Barrens - Ambassador Bloodrage', 0, 0, 10),
(8212, 1607, 0, 'Hinterlands - The Reak', 0, 0, 10),
(10077, 1602, 0, 'Burning Steppes - Deathmaw', 0, 0, 10),
(3581, 279, 0, 'Stormwind City - Sewer Beast', 0, 0, 10),
(815, 9917, 0, 'Bookie Herod', 0, 0, 10),
(2431, 1999, 0, 'Jailor Borhuin', 0, 0, 10),
(2306, 2139, 0, 'Baron Vardus', 0, 0, 10);
/*!40000 ALTER TABLE `pool_creature_template` ENABLE KEYS */;
UNLOCK TABLES;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;